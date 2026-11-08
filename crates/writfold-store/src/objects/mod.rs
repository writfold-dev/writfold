//! Immutable retained bytes in a private experimental-alpha realm layout.
//!
//! This initializes only the retained-object prerequisite, not a ready realm,
//! keys, policy, services, truth, grants, knowledge, or completed publication.
//! Callers keep the namespace private and use the managed lock protocol. Explicit
//! pathname checks detect observed changes; they do not prevent owner races.
//! Changing a bound realm incarnation requires separately qualified restore.

mod error;
mod identity;
mod retention;

pub use error::ObjectError;
use std::path::Path;
use writfold_domain::engineering::objects::RealmIdentity;
use writfold_domain::{ByteCount, RealmId};
use writfold_runtime::fs::{ContainedRoot, FileIdentity, ManagedLocks, PinnedFile};
use writfold_wire::Limits;

/// Owns descriptors and identity bytes, never self-referential borrowed handles.
/// Successful methods describe retained bytes, not authority to apply effects.
#[derive(Debug)]
pub struct ObjectStore {
    root: ContainedRoot,
    objects: ContainedRoot,
    hashes: ContainedRoot,
    locks: ManagedLocks,
    identity_file: PinnedFile,
    identity: RealmIdentity,
    identity_bytes: Vec<u8>,
    identity_inode: FileIdentity,
    identity_limit: ByteCount,
}

impl ObjectStore {
    /// Validate all identity representation/metadata budgets before creating one
    /// new root. Existing roots refuse, and failed initialization is not readiness.
    pub fn initialize(
        path: impl AsRef<Path>,
        identity: RealmIdentity,
        limits: Limits,
    ) -> Result<Self, ObjectError> {
        identity::initialize(path.as_ref(), identity, limits)
    }

    /// Open an existing prerequisite without creating or synchronizing anything.
    /// Validate the retained identity against the supplied expected realm, then
    /// bind both its exact original bytes and its file incarnation.
    pub fn open(
        path: impl AsRef<Path>,
        expected: &RealmId,
        limits: Limits,
    ) -> Result<Self, ObjectError> {
        identity::open(path.as_ref(), expected, limits)
    }

    pub fn identity(&self) -> &RealmIdentity {
        &self.identity
    }
}
