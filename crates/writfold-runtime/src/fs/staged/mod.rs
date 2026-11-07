//! Closed single-file content replacement for a private, run-owned worktree.
//!
//! All writers/recovery must cooperate through the same stable ManagedLocks key.
//! The owner admits reservations/quiescence and rechecks root/current guards;
//! these primitives neither authorize writes nor prove hostile race freedom.
//! There must be no inode-forging/reuse attacker or uncoordinated filesystem writer.
//! Admission excludes administrator-installed hidden attributes (e.g. trusted.*
//! hidden from unprivileged listxattr); all enumerable ACLs/xattrs are refused.
//! Only admitted ordinary mode/uid/gid are preserved, not original timestamps.
//! No atomic multi-file or hardware power-loss guarantee is made.
//!
//! Owner order: durable dispatch claim, stage+fsync, durable witness with original
//! effect/intent, current-guard recheck, install+directory fsync, durable receipt.
//! Dropping a stage intentionally retains it. Recovery is bounded observation and
//! qualified exact-stage cleanup, never redispatch of a claimed operation.

mod create;
mod inspect;
mod reconcile;
mod types;

pub use types::{
    InstallError, InstalledStage, RetainedStage, StageError, StageFailure, StageIdentity,
    StageImage, StageMetadata, StageReconciliation, StageWitness,
};

use super::identity::digest;
use super::{BudgetKind, FsError, LockedFile};
use inspect::{bindings, capture, check_named, match_image};
use rustix::fd::OwnedFd;
use rustix::fs::{fsync, renameat};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use writfold_domain::ByteCount;

/// A native exact observation, not a client-supplied hash assertion.
#[derive(Debug)]
pub struct ReplacementObservation {
    parent: StageIdentity,
    lock: StageIdentity,
    target_name: Vec<u8>,
    preimage: StageImage,
}

impl ReplacementObservation {
    pub fn preimage(&self) -> &StageImage {
        &self.preimage
    }
}

/// Single-use handle tied to the same held stable lock. No cleanup on drop.
///
/// One held flock must not permit two live mutators inside the same process:
/// ```compile_fail,E0499
/// use writfold_domain::ByteCount;
/// use writfold_runtime::fs::{LockedFile, ReplacementObservation, StageFailure};
/// fn overlapping(
///     guard: &mut LockedFile<'_>,
///     left: ReplacementObservation,
///     right: ReplacementObservation,
///     limit: ByteCount,
/// ) -> Result<(), StageFailure> {
///     let first = guard.stage_replacement(left, b"first", limit)?;
///     let second = guard.stage_replacement(right, b"second", limit)?;
///     std::thread::scope(|scope| {
///         scope.spawn(move || first.install());
///         scope.spawn(move || second.install());
///     });
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct StagedReplacement<'guard, 'target> {
    guard: &'guard mut LockedFile<'target>,
    descriptor: OwnedFd,
    witness: StageWitness,
}

impl<'target> LockedFile<'target> {
    pub fn observe_replacement(
        &self,
        limit: ByteCount,
    ) -> Result<ReplacementObservation, StageError> {
        let (parent, lock) = bindings(self)?;
        let (_, preimage) = capture(&self.target.parent, &self.target.name, limit)?;
        self.check_current()?;
        Ok(ReplacementObservation {
            parent,
            lock,
            target_name: self.target.name.as_bytes().to_vec(),
            preimage,
        })
    }

    /// Requires a native observation and exact recheck, never just an expected digest.
    /// The caller must already have durably claimed this attempt. Failures can leave
    /// a retained partial stage; they never install or sweep files.
    pub fn stage_replacement<'guard>(
        &'guard mut self,
        observed: ReplacementObservation,
        content: &[u8],
        write_limit: ByteCount,
    ) -> Result<StagedReplacement<'guard, 'target>, StageFailure> {
        if content.len() as u64 > write_limit.get() {
            return Err(StageError::Filesystem(FsError::Budget {
                kind: BudgetKind::Write,
                limit: write_limit.get(),
            })
            .into());
        }
        if bindings(self)? != (observed.parent, observed.lock)
            || self.target.name.as_bytes() != observed.target_name
        {
            return Err(StageError::Conflict.into());
        }
        let _original = match_image(
            &self.target.parent,
            &self.target.name,
            &observed.preimage,
            false,
        )?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(content.len()).map_err(|_| {
            StageError::Filesystem(FsError::Budget {
                kind: BudgetKind::Allocation,
                limit: write_limit.get(),
            })
        })?;
        bytes.extend_from_slice(content);
        let digest = digest(content).map_err(StageError::from)?;
        let (descriptor, retained) = create::create(self.target, observed.parent)?;
        let metadata = create::fill(
            self.target,
            &descriptor,
            &retained,
            content,
            &observed.preimage.metadata,
        )
        .map_err(|source| StageFailure {
            source,
            retained: Some(retained.clone()),
        })?;
        let witness = StageWitness {
            parent: observed.parent,
            lock: observed.lock,
            target_name: observed.target_name,
            stage_name: retained.name.clone(),
            preimage: observed.preimage,
            candidate: StageImage {
                metadata,
                bytes,
                digest,
            },
        };
        let staged = StagedReplacement {
            guard: self,
            descriptor,
            witness,
        };
        staged.check().map_err(|source| StageFailure {
            source,
            retained: Some(retained),
        })?;
        Ok(staged)
    }
}

impl StagedReplacement<'_, '_> {
    /// Persist this witness before calling install. Encoding belongs to the owner.
    pub fn witness(&self) -> &StageWitness {
        &self.witness
    }

    fn check(&self) -> Result<(), StageError> {
        if bindings(self.guard)? != (self.witness.parent, self.witness.lock) {
            return Err(StageError::Conflict);
        }
        check_named(
            &self.guard.target.parent,
            OsStr::new(&self.witness.stage_name),
            &self.descriptor,
            &self.witness.candidate.metadata,
        )?;
        let _stage = match_image(
            &self.guard.target.parent,
            OsStr::new(&self.witness.stage_name),
            &self.witness.candidate,
            false,
        )?;
        let _original = match_image(
            &self.guard.target.parent,
            &self.guard.target.name,
            &self.witness.preimage,
            false,
        )?;
        self.guard.check_current()?;
        Ok(())
    }

    /// Consumes the handle. Pre-issue failures are distinct from all outcomes after
    /// rename was issued; the latter require reconciliation, not blind retry.
    /// Caller must recheck its external current guards immediately before this call.
    pub fn install(self) -> Result<InstalledStage, InstallError> {
        self.check().map_err(InstallError::NotInstalled)?;
        let target = self.guard.target;
        renameat(
            &target.parent,
            self.witness.stage_name.as_str(),
            &target.parent,
            &target.name,
        )
        .map_err(|error| InstallError::OutcomeUnknown(FsError::from(error).into()))?;
        self.finish_install()
            .map_err(InstallError::OutcomeUnknown)?;
        Ok(InstalledStage {
            identity: self.witness.candidate.metadata.identity,
        })
    }

    fn finish_install(&self) -> Result<(), StageError> {
        let target = self.guard.target;
        let installed = match_image(&target.parent, &target.name, &self.witness.candidate, true)?;
        fsync(&installed).map_err(FsError::from)?;
        fsync(&target.parent).map_err(FsError::from)?;
        self.guard.check_current()?;
        Ok(())
    }
}
