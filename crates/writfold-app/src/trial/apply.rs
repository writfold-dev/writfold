//! `writfold trial apply`: re-verify, guard, install, `Applied`.
//!
//! The candidate is installed only if, at apply time: the grant and approval
//! envelopes verify strictly under the policy; the approval binds exactly this
//! run's scope, prepared edit, report, procedure, resource and generation and
//! is inside its validity window; the kernel's authority check finds a current
//! grant covering Write on the worktree; and the worktree file still holds the
//! exact retained source bytes. A crash after install but before `Applied` is
//! recorded leaves an Unknown outcome: a second `apply` then refuses because
//! the file no longer matches the source, and the worktree shows what happened.
use super::approve::{GENERATION, RESOURCE, procedure};
use super::flow::{Blob, Event, Premises};
use super::state::Trial;
use super::{BUDGET, Result, limits, now, scope, stream};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use writfold_domain::engineering::execution::Capability;
use writfold_domain::engineering::grant::Grant;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::worktree::{PreparedEditDigest, Scope, WorktreeDigest};
use writfold_domain::{EpochSeconds, LedgerSequence, ResourceGeneration, ResourceId};
use writfold_kernel::authority::{Authority, Authorization};
use writfold_kernel::ledger::Record;
use writfold_runtime::fs::{ContainedRoot, LockKey, ManagedLocks, RelativePath};
use writfold_wire::signatures::edit::{ExpectedEdit, SignedEditApproval, verify_edit};
use writfold_wire::signatures::grant::{Signed, verify};

/// Every authorization check `apply` makes before touching the file, against
/// the retained envelopes: strict grant and approval verification, exact
/// bindings, validity, and the kernel current-grant check for Write.
pub fn authorize(
    trial: &Trial,
    scope: &Scope,
    prepared: [u8; 32],
    report: &Blob,
    grant: &Blob,
    approval: &Blob,
) -> Result<()> {
    let at = EpochSeconds::new(now()?)?;
    let signed_grant = Signed::<Grant>::decode(&trial.fetch(grant)?, limits()?)?;
    verify(&signed_grant, &trial.policy)?;
    let signed_approval = SignedEditApproval::decode(&trial.fetch(approval)?, limits()?)?;
    let resource: ResourceId = RESOURCE.parse()?;
    let expected = ExpectedEdit {
        scope,
        prepared: PreparedEditDigest::from_digest(prepared),
        report: report.digest,
        procedure: procedure(),
        resource: &resource,
        generation: ResourceGeneration::new(GENERATION)?,
    };
    verify_edit(&signed_approval, &trial.policy, expected, at)?;
    let mut authority = Authority::new(trial.realm.clone(), trial.policy.identity().clone(), at);
    let retained = BTreeSet::from([grant.digest]);
    authority
        .install(
            Authorization {
                realm: trial.realm.clone(),
                policy: trial.policy.identity().clone(),
                grant: signed_grant.record().clone(),
                envelope: grant.digest,
            },
            &retained,
        )
        .map_err(|r| format!("grant refused: {r:?}"))?;
    if !authority.covers(scope, Capability::Write, &[resource]) {
        return Err("no current grant covers writing this worktree".into());
    }
    Ok(())
}

/// What happened to the file. `Refused` means the original was never replaced.
pub(super) enum Outcome {
    Installed,
    Refused(String),
}

fn install(
    trial: &Trial,
    worktree: &Path,
    path: &str,
    run: &str,
    source: &[u8],
    candidate: &[u8],
) -> Result<Outcome> {
    let file = worktree.join(path);
    let parent = file.parent().ok_or("file has no parent directory")?;
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("file name")?;
    let locks_path = trial.root.join("locks");
    let locks = if locks_path.exists() {
        ManagedLocks::open(&locks_path)?
    } else {
        ManagedLocks::create(&locks_path)?
    };
    let target = ContainedRoot::open(parent)?.pin(&RelativePath::new(name)?)?;
    let mut guard = target.try_lock(&locks, &LockKey::new(&run.replace('.', "_"))?)?;
    let limit = super::state::count(BUDGET as u64)?;
    let observed = match guard.observe_replacement(limit) {
        Ok(observed) => observed,
        Err(error) => {
            return Ok(Outcome::Refused(format!(
                "could not observe {path}: {error}"
            )));
        }
    };
    if observed.preimage().bytes != source {
        return Ok(Outcome::Refused(format!(
            "{path} changed since it was prepared"
        )));
    }
    let stage = match guard.stage_replacement(observed, candidate, limit) {
        Ok(stage) => stage,
        Err(failure) => {
            return Ok(Outcome::Refused(format!(
                "staging failed: {}",
                failure.source
            )));
        }
    };
    // From here the original may be replaced: errors leave the outcome Unknown.
    stage.install()?;
    Ok(Outcome::Installed)
}

/// Apply the approved edit: authorize, record `Claimed`, install, then record
/// `Applied`, or `Abandoned` if the original was never replaced. Any error
/// after `Claimed` without an outcome leaves the run for `recover`.
pub fn apply(state: &Path, run: &str) -> Result<PathBuf> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let ledger = trial.ledger(&lease, &executable)?;
    let loaded = ledger.load::<Event, Premises>(&stream(run)?)?;
    if loaded.records.len() == 3 {
        return Err(format!(
            "run {run} is claimed without an outcome; run: writfold trial recover {run}"
        )
        .into());
    }
    let [first, second] = loaded.records.as_slice() else {
        return Err(format!("run {run} is not approved and awaiting apply").into());
    };
    let (
        Event::Prepared {
            path,
            base,
            source,
            candidate,
            report,
            prepared,
            ..
        },
        Event::Approved { grant, approval },
    ) = (&first.payload, &second.payload)
    else {
        return Err(format!("run {run} history is not prepared then approved").into());
    };
    authorize(
        &trial,
        &scope(run, base)?,
        *prepared,
        report,
        grant,
        approval,
    )?;
    let claimed = append(
        &ledger,
        run,
        second,
        3,
        "claimed",
        Event::Claimed {
            candidate: *candidate,
        },
    )?;
    let worktree = trial.worktree(run);
    let outcome = install(
        &trial,
        &worktree,
        path,
        run,
        &trial.fetch(source)?,
        &trial.fetch(candidate)?,
    )
    .map_err(|e| format!("outcome unknown ({e}); run: writfold trial recover {run}"))?;
    record_outcome(&ledger, run, &claimed, *candidate, outcome)?;
    Ok(worktree)
}

type R = Record<Event, Premises>;

/// Record the single outcome after `Claimed`; a refusal is an error to the caller.
fn record_outcome(
    ledger: &crate::ledger::Ledger<'_, super::flow::Flow>,
    run: &str,
    claimed: &R,
    candidate: super::flow::Blob,
    outcome: Outcome,
) -> Result<()> {
    match outcome {
        Outcome::Installed => {
            let event = Event::Applied {
                candidate,
                reconciled: false,
            };
            append(ledger, run, claimed, 4, "applied", event)?;
            Ok(())
        }
        Outcome::Refused(reason) => {
            let event = Event::Abandoned {
                reason: reason.clone(),
            };
            append(ledger, run, claimed, 4, "abandoned", event)?;
            Err(format!("{reason}; nothing was applied (recorded as abandoned)").into())
        }
    }
}

/// Append the next record of `run` after `previous`.
pub(super) fn append(
    ledger: &crate::ledger::Ledger<'_, super::flow::Flow>,
    run: &str,
    previous: &R,
    sequence: u64,
    name: &str,
    event: Event,
) -> Result<R> {
    let record = Record {
        id: format!("{run}.{name}").parse()?,
        stream: stream(run)?,
        sequence: LedgerSequence::new(sequence)?,
        predecessor: Some(previous.id.clone()),
        profile: ReducerProfile::EngineeringV1,
        closure: event.blobs(),
        payload: event,
        historical: Premises,
    };
    ledger.append(&stream(run)?, Some(&previous.head()), record.clone())?;
    Ok(record)
}
