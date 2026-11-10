//! Retention of a write head that an interrupted write left unretained.
use super::DoltBackend;
use super::ownership::owns;
use super::write::{deadline, execute, head, runtime};
use crate::backend::{Capability, Refusal};
use std::sync::atomic::AtomicBool;
use writfold_domain::dolt::DoltReadPin;
use writfold_runtime::dolt::DoltSql;

/// Ensure the retention root of `pin`, which must be the current head. A write
/// killed after `DOLT_COMMIT` and before its in-transaction `DOLT_BRANCH` leaves
/// the head unretained (the next write retains it as its expected-old); this
/// lets a reconciler read that head first. Idempotent; never moves the head.
/// Observation and creation run under one coordination lease.
pub(super) fn retain(
    backend: &DoltBackend<'_>,
    pin: &DoltReadPin,
    cancelled: &AtomicBool,
) -> Result<(), Refusal> {
    let writer = backend
        .writer
        .as_ref()
        .ok_or(Refusal::Unavailable(Capability::Write))?;
    if !owns(&writer.target, pin) {
        return Err(Refusal::NotOwner(
            "pin names a realm this writer does not own",
        ));
    }
    writer.claim.check_current().map_err(runtime)?;
    let deadline = deadline(backend)?;
    let lease = backend.database.acquire().map_err(runtime)?;
    let before = head(backend, &lease, deadline, cancelled)?;
    if &before.head != pin.commit() || !before.active {
        return Err(Refusal::Pin(
            "only the current write head is retained".into(),
        ));
    }
    if !before.retained {
        execute(
            backend,
            &lease,
            &DoltSql::retain_head(pin.commit()),
            deadline,
            cancelled,
        )?;
    }
    let after = head(backend, &lease, deadline, cancelled)?;
    lease.check_current().map_err(runtime)?;
    if &after.head != pin.commit() || !after.retained {
        return Err(Refusal::Backend("the head retention root is absent".into()));
    }
    Ok(())
}
