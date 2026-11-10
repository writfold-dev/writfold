//! Reconciliation of an interrupted publication: `observe` is read-only and
//! decides through the pure kernel; `complete` finishes only an `Interrupted`
//! publication, issuing Git steps and never the storage write.
use super::publish::{Pending, finish, retain};
use super::{
    Candidate, Observation, Publication, PublicationAuthority, PublicationError, ReleasePin,
    Releases, same_release, storage_base,
};
use std::sync::atomic::AtomicBool;
use writfold_domain::GitObjectId;
use writfold_kernel::publication::{Facts, Resolution, resolve};
use writfold_runtime::git::GitPrepared;
use writfold_store::backend::StorageBackend;

/// Observe one publication without writing. The caller holds `releases` and a
/// writer `backend`, so no step of it can still be in flight.
pub fn observe<B>(
    backend: &B,
    releases: &Releases<'_, '_>,
    authority: &B::WriteAuthority,
    candidate: &Candidate,
    cancelled: &AtomicBool,
) -> Result<Observation<B::Pin>, PublicationError>
where
    B: StorageBackend,
    B::Pin: ReleasePin,
    B::WriteAuthority: PublicationAuthority<B::Pin>,
{
    Ok(observed(backend, releases, authority, candidate, cancelled)?.0)
}

/// `observe`, plus the prepared commit when the publication is interrupted.
fn observed<B>(
    backend: &B,
    releases: &Releases<'_, '_>,
    authority: &B::WriteAuthority,
    candidate: &Candidate,
    cancelled: &AtomicBool,
) -> Result<(Observation<B::Pin>, Option<GitPrepared>), PublicationError>
where
    B: StorageBackend,
    B::Pin: ReleasePin,
    B::WriteAuthority: PublicationAuthority<B::Pin>,
{
    let completed = releases.completed(cancelled)?;
    let prepared = releases.prepared(authority.operation(), cancelled)?;
    let base = storage_base(backend, authority, cancelled)?;
    let storage = backend.observe(&candidate.request(&base.pin)?, cancelled)?;
    let mut named = None;
    let mut published = false;
    if let Some(prepared) = &prepared {
        let released = releases.interpret::<B::Pin>(prepared.commit(), cancelled)?;
        named = Some(released.manifest.pin);
        published = releases.includes(completed.as_ref(), prepared, cancelled)?;
    }
    let facts = Facts {
        published,
        prepared: prepared.is_some(),
        prepared_names_applied: named.is_some() && named == storage.applied,
        applied: storage.applied.is_some(),
        head_is_applied: storage.applied.as_ref() == Some(&storage.head),
        completed_is_base: same_release(
            authority.base().map(|(release, _)| release),
            completed.as_ref(),
        ),
    };
    let observation = match (resolve(&facts), storage.applied, &prepared) {
        (Resolution::Published, Some(pin), Some(prepared)) => Observation::Published {
            release: prepared.commit().clone(),
            pin,
        },
        (Resolution::NotPublished, _, _) => Observation::NotPublished,
        (Resolution::Interrupted { .. }, Some(pin), _) => Observation::Interrupted {
            pin,
            prepared: prepared.as_ref().map(|p| p.commit().clone()),
        },
        _ => return Err(PublicationError::Diverged),
    };
    Ok((observation, prepared))
}

/// Finish an `Interrupted` publication: clear this lease's abandoned reference
/// locks, retain the candidate blob (idempotent), retain the storage head and
/// prepare if absent, then the Git expected-old CAS. Anything else refuses
/// `NotInterrupted` without writing. The outcome may again be `Unknown`.
pub fn complete<B>(
    backend: &mut B,
    releases: &Releases<'_, '_>,
    authority: &B::WriteAuthority,
    candidate: &Candidate,
    cancelled: &AtomicBool,
) -> Result<Publication<B::Pin>, PublicationError>
where
    B: StorageBackend,
    B::Pin: ReleasePin,
    B::WriteAuthority: PublicationAuthority<B::Pin>,
{
    let (observation, prepared) = observed(backend, releases, authority, candidate, cancelled)?;
    let Observation::Interrupted { pin, .. } = observation else {
        return Err(PublicationError::NotInterrupted);
    };
    let base: Option<GitObjectId> = authority
        .base()
        .map(|(release, _)| release.as_str().parse())
        .transpose()
        .map_err(|_| PublicationError::Manifest("base release"))?;
    // This lease's own killed `update-ref` may have left a reference lock.
    releases.native().clear_stale_ref_locks()?;
    let Ok(blob) = retain(releases, candidate, cancelled) else {
        return Ok(Publication::Unknown);
    };
    let pending = Pending {
        operation: authority.operation(),
        base: base.as_ref(),
        candidate,
        blob,
        pin,
    };
    Ok(finish(backend, releases, pending, prepared, cancelled).unwrap_or(Publication::Unknown))
}
