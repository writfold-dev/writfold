//! Each authorization check and validator rule refuses on its own.
use super::approve::{RESOURCE, approval, grant, validity};
use super::flow::{Blob, Event, Flow, Premises};
use super::{Result, Trial, authorize, now, scope};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_domain::LedgerSequence;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::worktree::Scope;
use writfold_kernel::ledger::{Record, Validator};

static NEXT: AtomicU64 = AtomicU64::new(0);
const RUN: &str = "run.1.aaaaaa";
const BASE: &str = "0123456789abcdef0123456789abcdef01234567";

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn state() -> Result<(Temp, Trial)> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-trial-unit-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    Trial::init(&root.join("state"))?;
    let trial = Trial::open(&root.join("state"))?;
    Ok((Temp(root), trial))
}

struct Envelopes {
    scope: Scope,
    report: Blob,
    grant: Blob,
    approval: Blob,
}

fn envelopes(
    trial: &Trial,
    signer: &Trial,
    resource: &str,
    prepared: [u8; 32],
) -> Result<Envelopes> {
    let scope = scope(RUN, BASE)?;
    let window = validity(now()?)?;
    let report = trial.retain(b"report")?;
    let grant = trial.retain(&grant(signer, RUN, &scope, window, resource)?)?;
    let approval = trial.retain(&approval(trial, &scope, prepared, &report, window)?)?;
    Ok(Envelopes {
        scope,
        report,
        grant,
        approval,
    })
}

#[test]
fn authorize_accepts_the_matching_grant_and_approval() -> Result<()> {
    let (_t, trial) = state()?;
    let e = envelopes(&trial, &trial, RESOURCE, [1; 32])?;
    authorize(&trial, &e.scope, [1; 32], &e.report, &e.grant, &e.approval)
}

#[test]
fn each_authorization_check_refuses_alone() -> Result<()> {
    let (_t, trial) = state()?;
    let (_u, other) = state()?;
    // A grant signed by a key this policy does not trust; the approval is fine.
    let e = envelopes(&trial, &other, RESOURCE, [1; 32])?;
    assert!(authorize(&trial, &e.scope, [1; 32], &e.report, &e.grant, &e.approval).is_err());
    // A valid grant, but the approval binds a different prepared edit.
    let e = envelopes(&trial, &trial, RESOURCE, [2; 32])?;
    assert!(authorize(&trial, &e.scope, [1; 32], &e.report, &e.grant, &e.approval).is_err());
    // Both verify, but the grant does not cover the worktree resource.
    let e = envelopes(&trial, &trial, "other", [1; 32])?;
    assert!(authorize(&trial, &e.scope, [1; 32], &e.report, &e.grant, &e.approval).is_err());
    Ok(())
}

fn record(
    sequence: u64,
    event: Event,
    closure: Vec<writfold_domain::engineering::BlobDigest>,
) -> Result<Record<Event, Premises>> {
    Ok(Record {
        id: format!("r{sequence}").parse()?,
        stream: RUN.parse()?,
        sequence: LedgerSequence::new(sequence)?,
        predecessor: None,
        profile: ReducerProfile::EngineeringV1,
        payload: event,
        historical: Premises,
        closure,
    })
}

#[test]
fn the_flow_validator_requires_order_material_and_the_prepared_candidate() -> Result<()> {
    use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
    let blob = |n: u8| Blob {
        digest: BlobDigest::from_digest([n; 32]),
        bytes: 1,
    };
    let prepared = Event::Prepared {
        path: "src/a.rs".into(),
        base: BASE.into(),
        source: blob(1),
        recipe: blob(2),
        candidate: blob(3),
        report: blob(4),
        prepared: [9; 32],
    };
    let first = record(1, prepared.clone(), prepared.blobs())?;
    assert!(Flow.validate(&[], &first));
    assert!(
        !Flow.validate(&[], &record(1, prepared.clone(), vec![])?),
        "material must be carried"
    );
    let approved = Event::Approved {
        grant: blob(5),
        approval: blob(6),
    };
    let second = record(2, approved.clone(), approved.blobs())?;
    assert!(
        !Flow.validate(&[], &second),
        "approval needs a prepared edit"
    );
    assert!(Flow.validate(std::slice::from_ref(&first), &second));
    let claim = |n| Event::Claimed { candidate: blob(n) };
    let third = record(3, claim(3), claim(3).blobs())?;
    let pa = [first.clone(), second.clone()];
    assert!(Flow.validate(&pa, &third));
    let wrong = record(3, claim(7), claim(7).blobs())?;
    assert!(
        !Flow.validate(&pa, &wrong),
        "the claim must name the prepared candidate"
    );
    let applied = |n| Event::Applied {
        candidate: blob(n),
        reconciled: false,
    };
    assert!(
        !Flow.validate(&pa, &record(3, applied(3), applied(3).blobs())?),
        "no outcome without a claim"
    );
    let pac = [first.clone(), second.clone(), third.clone()];
    assert!(Flow.validate(&pac, &record(4, applied(3), applied(3).blobs())?));
    assert!(
        !Flow.validate(&pac, &record(4, applied(7), applied(7).blobs())?),
        "candidate must be the prepared one"
    );
    let abandon = |r: &str| Event::Abandoned { reason: r.into() };
    assert!(Flow.validate(&pac, &record(4, abandon("not installed"), vec![])?));
    assert!(
        !Flow.validate(&pac, &record(4, abandon(""), vec![])?),
        "an abandonment says why"
    );
    let done = [
        first,
        second,
        third,
        record(4, applied(3), applied(3).blobs())?,
    ];
    assert!(
        !Flow.validate(&done, &record(5, abandon("again"), vec![])?),
        "nothing after an outcome"
    );
    Ok(())
}
