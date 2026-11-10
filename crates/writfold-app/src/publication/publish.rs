//! `publish`: steps 1–5 of the publication order (see the module docs).
use super::release::CANDIDATE_PATH;
use super::{
    Candidate, Manifest, Publication, PublicationAuthority, PublicationError, ReleasePin, Releases,
    same_release, storage_base,
};
use std::sync::atomic::AtomicBool;
use writfold_domain::{GitObjectId, PlanDigest};
use writfold_kernel::publication::{Admission, Before, admit};
use writfold_runtime::fs::RelativePath;
use writfold_runtime::git::{GitManifestEntry, GitPrepareInput, GitPrepared, GitWriteError};
use writfold_store::backend::{ExactRead, Refusal, StorageBackend, WriteOutcome};

/// Publish `candidate` under `authority`. Holding `releases` and a writer
/// `backend` makes this the realm's only publisher. Never retries.
pub fn publish<B>(
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
    let base = storage_base(backend, authority, cancelled)?;
    let release = authority.base().map(|(release, _)| release);
    let completed = releases.completed(cancelled)?;
    let completed_is_base = same_release(release, completed.as_ref());
    let base_pin_matches = match (&completed, completed_is_base) {
        (Some(current), true) => {
            releases
                .interpret::<B::Pin>(current, cancelled)?
                .manifest
                .pin
                == base.pin
        }
        (None, true) => !base.written,
        _ => false,
    };
    let before = Before {
        bootstrap: release.is_none(),
        completed_is_base,
        base_pin_matches,
        prepared: releases
            .prepared(authority.operation(), cancelled)?
            .is_some(),
    };
    match admit(&before) {
        Admission::Proceed => {}
        Admission::Conflict => {
            return Ok(Publication::Conflict {
                observed: completed,
            });
        }
        Admission::Misinterpreted => return Err(PublicationError::Misinterpreted),
        Admission::Initialized => return Err(PublicationError::Initialized),
        Admission::AlreadyPrepared => return Err(PublicationError::AlreadyPrepared),
    }
    let blob = retain(releases, candidate, cancelled)?;
    let request = candidate.request(&base.pin)?;
    let new = match backend.admit_write(&request, Some(authority), cancelled) {
        Ok(WriteOutcome::Committed { new }) => new,
        Ok(WriteOutcome::Stale { observed }) => return Ok(Publication::Stale { observed }),
        Ok(WriteOutcome::Unknown) => return Ok(Publication::Unknown),
        Err(refusal) => return Err(refusal.into()),
    };
    let finished = finish(
        backend,
        releases,
        Pending {
            operation: authority.operation(),
            base: completed.as_ref(),
            candidate,
            blob,
            pin: new,
        },
        None,
        cancelled,
    );
    Ok(finished.unwrap_or(Publication::Unknown))
}

/// The storage commit exists; everything still owed to Git.
pub(super) struct Pending<'p, P> {
    pub(super) operation: &'p PlanDigest,
    /// The Git expected-old release; `None` for a bootstrap.
    pub(super) base: Option<&'p GitObjectId>,
    pub(super) candidate: &'p Candidate,
    pub(super) blob: GitObjectId,
    pub(super) pin: P,
}

/// Steps 4–5. Any error here leaves a state `observe` resolves; the caller
/// reports `Unknown`. `prepared` is a prepared commit `observe` already
/// checked names `pending.pin`.
pub(super) fn finish<B>(
    backend: &mut B,
    releases: &Releases<'_, '_>,
    pending: Pending<'_, B::Pin>,
    prepared: Option<GitPrepared>,
    cancelled: &AtomicBool,
) -> Result<Publication<B::Pin>, PublicationError>
where
    B: StorageBackend,
    B::Pin: ReleasePin,
{
    let prepared = match prepared {
        Some(prepared) => prepared,
        None => {
            backend.retain(&pending.pin, cancelled)?;
            readback(backend, &pending, cancelled)?;
            let manifest = Manifest {
                operation: pending.operation.clone(),
                candidate: pending.candidate.digest().clone(),
                blob: pending.blob.clone(),
                pin: pending.pin.clone(),
            }
            .encode()?;
            let entries = [GitManifestEntry {
                path: path(CANDIDATE_PATH)?,
                blob: pending.blob.clone(),
            }];
            let input = GitPrepareInput::new(pending.operation, pending.base, &manifest, &entries);
            let native = releases.native();
            native
                .prepare(releases.executable(), input, releases.budgets(), cancelled)
                .map_err(git)?
        }
    };
    let native = releases.native();
    native
        .publish(
            releases.executable(),
            &prepared,
            releases.budgets(),
            cancelled,
        )
        .map_err(git)?;
    Ok(Publication::Published {
        release: prepared.commit().clone(),
        pin: pending.pin,
    })
}

/// Every change reads back exactly at the new pin through the exact reader.
fn readback<B: StorageBackend>(
    backend: &B,
    pending: &Pending<'_, B::Pin>,
    cancelled: &AtomicBool,
) -> Result<(), PublicationError> {
    for change in pending.candidate.changes() {
        let read = backend.read_exact(
            &pending.pin,
            &ExactRead::new(change.key.clone(), None),
            cancelled,
        );
        let exact = match (&change.content, read) {
            (Some(content), Ok(record)) => {
                record.content() == content.as_slice() && record.kind() == &change.kind
            }
            (None, Err(Refusal::NotFound)) => true,
            (_, Err(refusal)) => return Err(refusal.into()),
            (None, Ok(_)) => false,
        };
        if !exact {
            return Err(PublicationError::Diverged);
        }
    }
    Ok(())
}

/// Step 2: the candidate's canonical bytes as a retained Git blob.
pub(super) fn retain(
    releases: &Releases<'_, '_>,
    candidate: &Candidate,
    cancelled: &AtomicBool,
) -> Result<GitObjectId, PublicationError> {
    releases
        .native()
        .write_blob(
            releases.executable(),
            candidate.bytes(),
            releases.budgets(),
            cancelled,
        )
        .map_err(git)
}

fn git(error: GitWriteError) -> PublicationError {
    match error {
        GitWriteError::Unissued(error) | GitWriteError::Unknown(error) => error.into(),
        GitWriteError::Conflict { .. } => PublicationError::Diverged,
        GitWriteError::PreparedExists(_) => PublicationError::AlreadyPrepared,
    }
}

fn path(value: &str) -> Result<RelativePath, PublicationError> {
    RelativePath::new(value).map_err(|_| PublicationError::Manifest("path"))
}
