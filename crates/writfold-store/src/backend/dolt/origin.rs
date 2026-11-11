//! The realm's origin: the head before the first Writfold write.
//!
//! The writer advances `main` linearly, and every commit it makes carries the
//! `writfold-dolt-write/1` marker family. So the realm's origin is the current
//! head while no such commit is on the head's history, and afterwards the
//! parent of the earliest one. A bootstrap writes only from an unwritten origin,
//! checked under the same coordination lease as its commit, and its recovery
//! recomputes the same origin, and so the same request identity.
use super::DoltBackend;
use super::write::{FAMILY, deadline, execute, head, runtime};
use crate::backend::{Capability, Origin, Refusal};
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::DoltIntegrationCommitId;
use writfold_domain::dolt::DoltReadPin;
use writfold_runtime::dolt::{DoltLease, DoltSql};
use writfold_wire::dolt::decode_applied;
use writfold_wire::dolt_operation::DoltTarget;

/// Read-only: the origin of the writer's realm.
pub(super) fn origin(
    backend: &DoltBackend<'_>,
    cancelled: &AtomicBool,
) -> Result<Origin<DoltReadPin>, Refusal> {
    let writer = backend
        .writer
        .as_ref()
        .ok_or(Refusal::Unavailable(Capability::Write))?;
    let deadline = deadline(backend)?;
    let lease = backend.database.acquire().map_err(runtime)?;
    let current = head(backend, &lease, deadline, cancelled)?;
    let first = first_parent(backend, &lease, deadline, cancelled)?;
    lease.check_current().map_err(runtime)?;
    Ok(match first {
        Some(parent) => Origin {
            pin: pin(&writer.target, parent),
            written: true,
        },
        None => Origin {
            pin: pin(&writer.target, current.head),
            written: false,
        },
    })
}

/// The parent of the earliest Writfold write on the head's history, if any.
pub(super) fn first_parent(
    backend: &DoltBackend<'_>,
    lease: &DoltLease<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Option<DoltIntegrationCommitId>, Refusal> {
    let sql = DoltSql::origin_probe(FAMILY).map_err(runtime)?;
    let bytes = execute(backend, lease, &sql, deadline, cancelled)?;
    decode_applied(&bytes, backend.budgets.wire())
        .map_err(|error| Refusal::Backend(error.to_string()))
}

/// The writer's realm identity at `commit`.
fn pin(target: &DoltTarget, commit: DoltIntegrationCommitId) -> DoltReadPin {
    DoltReadPin::new(
        target.realm().clone(),
        target.kb().clone(),
        target.database().clone(),
        commit,
        target.epoch().clone(),
        target.profile().clone(),
        target.producer().clone(),
    )
}
