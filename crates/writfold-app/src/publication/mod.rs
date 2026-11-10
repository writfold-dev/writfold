//! Approved publication of a prepared candidate over the storage boundary
//! (`writfold-dolt-integration/1`, the first DINT02 slice; see
//! `docs/writfold/dolt-integration.md#publication-order-and-recovery`).
//!
//! One signed approval authorizes one publication of one candidate. The steps
//! run in this order, under the realm's exclusive Git writer lease (held by
//! `Releases`) and the backend's exclusive writer lease:
//!
//! 1. Git expected-old check: the completed ref is exactly the approval's base
//!    release, and that release's retained manifest names exactly the
//!    approval's base pin. Otherwise `Conflict` or a refusal; nothing written.
//! 2. Retain the candidate bytes as a Git blob (invisible, idempotent).
//! 3. Conditional storage commit from the base pin (`admit_write`).
//! 4. Read every change back at the new pin, then a Git prepared commit whose
//!    manifest names the operation, the candidate digest and blob, and the new pin.
//! 5. Git expected-old CAS of the completed ref: the only visibility point.
//!
//! Any uncertainty after step 3 may have begun is `Unknown`. It is resolved by
//! `observe` alone, and the storage write is never reissued. Only the state the
//! pure kernel calls `Interrupted` (storage committed, Git ref still the base)
//! may be completed, and completion issues only the Git steps.
//!
//! The approval's signature does not cover the candidate: the signed Dolt
//! operation has no candidate field. Writfold binds the candidate digest to the
//! operation in the retained manifest and refuses a second preparation of the
//! same operation. A signed candidate binding remains open.
mod candidate;
mod publish;
mod recover;
mod release;

pub use candidate::Candidate;
pub use publish::publish;
pub use recover::{complete, observe};
pub use release::{Manifest, Released, Releases};

use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{GitObjectId, PlanDigest, ReleaseId};
use writfold_runtime::git::GitError;
use writfold_store::backend::Refusal;
use writfold_store::backend::dolt::DoltWriteApproval;
use writfold_wire::Limits;
use writfold_wire::dolt::{decode_pin, encode_pin};

const PIN_BYTES: usize = 4096;

/// A backend pin that a release manifest can name exactly and read back.
pub trait ReleasePin: Clone + Eq + std::fmt::Debug + Sized {
    fn encode(&self) -> Result<Vec<u8>, PublicationError>;
    fn decode(bytes: &[u8]) -> Result<Self, PublicationError>;
}

/// The checked fields of a signed approval a publication needs.
pub trait PublicationAuthority<P> {
    /// The signed operation identity; one prepared commit per operation.
    fn operation(&self) -> &PlanDigest;
    /// The signed predecessor release: the Git expected-old value.
    fn release(&self) -> &ReleaseId;
    /// The signed base pin: the storage expected-old head.
    fn base(&self) -> &P;
}

impl ReleasePin for DoltReadPin {
    fn encode(&self) -> Result<Vec<u8>, PublicationError> {
        encode_pin(self, PIN_BYTES).map_err(|_| PublicationError::Manifest("pin encoding"))
    }
    fn decode(bytes: &[u8]) -> Result<Self, PublicationError> {
        let limits = Limits::new(PIN_BYTES, 4, 32, 64)
            .map_err(|_| PublicationError::Manifest("pin limits"))?;
        decode_pin(bytes, limits).map_err(|_| PublicationError::Manifest("pin decoding"))
    }
}

impl PublicationAuthority<DoltReadPin> for DoltWriteApproval {
    fn operation(&self) -> &PlanDigest {
        self.plan()
    }
    fn release(&self) -> &ReleaseId {
        DoltWriteApproval::release(self)
    }
    fn base(&self) -> &DoltReadPin {
        DoltWriteApproval::base(self)
    }
}

/// The typed outcome of `publish` or `complete`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Publication<P> {
    /// The completed ref names `release`, whose manifest names `pin`.
    Published { release: GitObjectId, pin: P },
    /// The completed ref is not the approval's base release. Nothing written.
    Conflict { observed: Option<GitObjectId> },
    /// The storage head is not the base pin. Nothing written to storage or Git
    /// beyond the invisible retained candidate blob.
    Stale { observed: P },
    /// Something may have happened. Resolve with `observe`; never retry.
    Unknown,
}

/// The result of `observe`: read-only reconciliation of one publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Observation<P> {
    Published {
        release: GitObjectId,
        pin: P,
    },
    /// Nothing was committed for this request; it can never become visible.
    NotPublished,
    /// Storage committed `pin`; the Git ref is still the base. `complete` may
    /// finish it. `prepared` names the prepared commit if it exists.
    Interrupted {
        pin: P,
        prepared: Option<GitObjectId>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum PublicationError {
    #[error(transparent)]
    Storage(#[from] Refusal),
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("release manifest refused: {0}")]
    Manifest(&'static str),
    #[error("the base release does not name the approval's base pin")]
    Misinterpreted,
    #[error("this operation is already prepared; reconcile with observe")]
    AlreadyPrepared,
    #[error("observed facts are excluded by the publication order; a person decides")]
    Diverged,
    #[error("only an interrupted publication can be completed")]
    NotInterrupted,
}

/// A Git release identity as the signed `ReleaseId` names it.
fn same_release(release: &ReleaseId, commit: Option<&GitObjectId>) -> bool {
    commit.is_some_and(|commit| commit.as_str() == release.as_str())
}
