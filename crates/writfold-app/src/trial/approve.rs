//! `writfold trial approve`: operator grant and prepared-edit approval, both
//! signed with the state's operator key under its policy, then `Approved`.
use super::flow::{Blob, Event, Premises};
use super::state::Trial;
use super::{Result, VALIDITY_SECONDS, limits, now, scope, stream};
use std::path::Path;
use writfold_domain::engineering::approval::{EditApproval, Signer, Validity};
use writfold_domain::engineering::execution::Capability;
use writfold_domain::engineering::grant::Grant;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::engineering::{EngineeringDigest, ProcedureRevision};
use writfold_domain::worktree::{PreparedEditDigest, Scope, WorktreeDigest};
use writfold_domain::{
    EpochSeconds, ExecutionCost, LedgerSequence, ResourceGeneration, ResourceId,
};
use writfold_kernel::ledger::Record;

/// The single resource a trial run may write: its own worktree.
pub const RESOURCE: &str = "worktree";
pub const GENERATION: u64 = 1;

/// A fixed procedure identity for the trial profile; not an admitted procedure.
pub fn procedure() -> ProcedureRevision {
    let digest: [u8; 32] = <sha2::Sha256 as sha2::Digest>::digest(super::PROFILE.as_bytes()).into();
    ProcedureRevision::from_digest(digest)
}

pub fn validity(from: u64) -> Result<Validity> {
    Ok(Validity::new(
        EpochSeconds::new(from)?,
        EpochSeconds::new(from + VALIDITY_SECONDS)?,
    )?)
}

fn signer(trial: &Trial) -> Result<Signer> {
    Ok(Signer::new(
        "operator".parse()?,
        trial.policy.identity().clone(),
    ))
}

/// A signed operator grant of Write on `resource` for this run scope.
pub(super) fn grant(
    trial: &Trial,
    run: &str,
    scope: &Scope,
    validity: Validity,
    resource: &str,
) -> Result<Vec<u8>> {
    let grant = Grant::new(
        format!("{run}.grant").parse()?,
        scope.clone(),
        vec![resource.parse::<ResourceId>()?],
        vec![Capability::Write],
        ExecutionCost::new(1)?,
        validity,
        signer(trial)?,
    )?;
    let signed = trial
        .signing_key()?
        .sign_record(grant, &trial.policy, limits()?)?;
    Ok(signed.encode(limits()?)?)
}

pub(super) fn approval(
    trial: &Trial,
    scope: &Scope,
    prepared: [u8; 32],
    report: &Blob,
    validity: Validity,
) -> Result<Vec<u8>> {
    let approval = EditApproval::new(
        scope.clone(),
        PreparedEditDigest::from_digest(prepared),
        report.digest,
        procedure(),
        RESOURCE.parse()?,
        ResourceGeneration::new(GENERATION)?,
        validity,
        signer(trial)?,
    );
    let signed = trial
        .signing_key()?
        .sign_edit(approval, &trial.policy, limits()?)?;
    Ok(signed.encode(limits()?)?)
}

/// Sign and record approval of the run's prepared edit. Returns the expiry.
pub fn approve(state: &Path, run: &str) -> Result<u64> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let ledger = trial.ledger(&lease, &executable)?;
    let loaded = ledger.load::<Event, Premises>(&stream(run)?)?;
    let [prepared_record] = loaded.records.as_slice() else {
        return Err(format!("run {run} is not awaiting approval").into());
    };
    let Event::Prepared {
        base,
        report,
        prepared,
        ..
    } = &prepared_record.payload
    else {
        return Err(format!("run {run} has no prepared edit").into());
    };
    let scope = scope(run, base)?;
    let validity = validity(now()?)?;
    let event = Event::Approved {
        grant: trial.retain(&grant(&trial, run, &scope, validity, RESOURCE)?)?,
        approval: trial.retain(&approval(&trial, &scope, *prepared, report, validity)?)?,
    };
    let record = Record {
        id: format!("{run}.approved").parse()?,
        stream: stream(run)?,
        sequence: LedgerSequence::new(2)?,
        predecessor: Some(prepared_record.id.clone()),
        profile: ReducerProfile::EngineeringV1,
        closure: event.blobs(),
        payload: event,
        historical: Premises,
    };
    ledger.append(&stream(run)?, Some(&prepared_record.head()), record)?;
    Ok(validity.expires().get())
}
