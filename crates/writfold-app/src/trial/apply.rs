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

fn install(
    trial: &Trial,
    worktree: &Path,
    path: &str,
    run: &str,
    source: &[u8],
    candidate: &[u8],
) -> Result<()> {
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
    let observed = guard.observe_replacement(limit)?;
    if observed.preimage().bytes != source {
        return Err(format!("{path} changed since it was prepared; nothing was applied").into());
    }
    let stage = guard
        .stage_replacement(observed, candidate, limit)
        .map_err(|f| f.source)?;
    stage.install()?;
    Ok(())
}

/// Apply the approved edit to the run's worktree. Returns the worktree path.
pub fn apply(state: &Path, run: &str) -> Result<PathBuf> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let ledger = trial.ledger(&lease, &executable)?;
    let loaded = ledger.load::<Event, Premises>(&stream(run)?)?;
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
    let scope = scope(run, base)?;
    authorize(&trial, &scope, *prepared, report, grant, approval)?;
    let worktree = trial.worktree(run);
    let candidate_bytes = trial.fetch(candidate)?;
    install(
        &trial,
        &worktree,
        path,
        run,
        &trial.fetch(source)?,
        &candidate_bytes,
    )?;
    let event = Event::Applied {
        candidate: *candidate,
    };
    let record = Record {
        id: format!("{run}.applied").parse()?,
        stream: stream(run)?,
        sequence: LedgerSequence::new(3)?,
        predecessor: Some(second.id.clone()),
        profile: ReducerProfile::EngineeringV1,
        closure: event.blobs(),
        payload: event,
        historical: Premises,
    };
    ledger.append(&stream(run)?, Some(&second.head()), record)?;
    Ok(worktree)
}
