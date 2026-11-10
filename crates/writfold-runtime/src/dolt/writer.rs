//! The realm's single Dolt writer: one exclusive, recorded ownership lease.
//!
//! The lease is a non-blocking exclusive flock on the preprovisioned
//! `coordination/writer.lock` entry, held for the writer's whole lifetime. It is
//! separate from the per-operation `database.lock` lease that readers and the
//! writer share, so readers never wait on a long-lived writer. While holding the
//! lease the claimant replaces `coordination/writer.claim` with its own record,
//! so the current owner is recorded, not only enforced. Like every coordination
//! entry, this binds cooperating Writfold processes; hostile same-user processes
//! are outside the profile.
use super::{DoltDatabase, DoltError};
use crate::fs::{
    ContainedRoot, FsError, LockKey, LockedFile, ManagedLocks, PinnedFile, ReadExtent, RelativePath,
};
use writfold_domain::ByteCount;

/// Pinned writer-lease entry and claim record for one database. The slot
/// borrows its database, so a claim can never cover a different database.
#[derive(Debug)]
pub struct DoltWriterSlot<'d> {
    database: &'d DoltDatabase,
    locks: ManagedLocks,
    record: PinnedFile,
    key: LockKey,
}

impl<'d> DoltWriterSlot<'d> {
    pub(super) fn open(
        database: &'d DoltDatabase,
        coordination: ContainedRoot,
    ) -> Result<Self, DoltError> {
        let record =
            coordination.pin(&RelativePath::new("writer.claim").map_err(FsError::from)?)?;
        Ok(Self {
            database,
            locks: ManagedLocks::from_root(coordination),
            record,
            key: LockKey::new("writer")?,
        })
    }

    pub fn database(&self) -> &'d DoltDatabase {
        self.database
    }

    /// Take the exclusive writer lease without waiting, then record `claim` in
    /// place of the previous owner's record while holding it. A lease held by
    /// another writer refuses as `LockBusy`; the lock entry is never created.
    pub fn claim(&self, claim: &[u8], limit: ByteCount) -> Result<DoltWriterClaim<'_>, DoltError> {
        let mut lock = self.record.try_lock_existing(&self.locks, &self.key)?;
        let previous = match lock.read(limit) {
            Ok(read) if read.extent() == ReadExtent::Complete => Some(read.digest().clone()),
            Ok(_) => {
                return Err(DoltError::Budget {
                    stream: "writer claim",
                    limit: limit.get(),
                });
            }
            Err(FsError::Missing) => None,
            Err(error) => return Err(error.into()),
        };
        lock.compare_and_write(previous.as_ref(), claim, limit, limit)?;
        Ok(DoltWriterClaim { lock })
    }
}

/// A held writer lease. Dropping it releases ownership; the record remains.
#[derive(Debug)]
pub struct DoltWriterClaim<'a> {
    lock: LockedFile<'a>,
}

impl DoltWriterClaim<'_> {
    /// The lease entry is still the one locked, under the same namespace.
    pub fn check_current(&self) -> Result<(), DoltError> {
        Ok(self.lock.check_current()?)
    }
}
