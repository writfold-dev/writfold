use super::super::{FsError, LockedFile};
use super::inspect::{check_named, identity, match_image, validate};
use super::{InstalledStage, StageError, StageReconciliation, StageWitness};
use rustix::fs::{AtFlags, fsync, statat, unlinkat};
use rustix::io::Errno;
use std::ffi::OsStr;
use writfold_domain::ByteCount;

impl LockedFile<'_> {
    /// Only after owner-admitted quiescence/reservation and current-root checks.
    /// This never installs. An exact destination stage inode plus exact candidate
    /// is required for confirmation; equal bytes on another inode remain Unknown.
    /// Only the exact witnessed stage may be removed in the no-install branch.
    /// Invalid/incomplete records, missing stages and any failed sync stay Unknown.
    pub fn reconcile_stage(
        &mut self,
        witness: &StageWitness,
        limit: ByteCount,
    ) -> StageReconciliation {
        match self.reconcile_observed(witness, limit) {
            Ok(outcome) => outcome,
            Err(error) => StageReconciliation::Unknown(error),
        }
    }

    fn reconcile_observed(
        &self,
        witness: &StageWitness,
        limit: ByteCount,
    ) -> Result<StageReconciliation, StageError> {
        validate(self, witness, limit)?;
        let current = statat(
            &self.target.parent,
            &self.target.name,
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(FsError::from)?;
        if identity(&current) == witness.candidate.metadata.identity {
            self.confirm_installed(witness)
        } else if identity(&current) == witness.preimage.metadata.identity {
            self.clean_uninstalled(witness)
        } else {
            Err(StageError::Conflict)
        }
    }

    fn confirm_installed(&self, witness: &StageWitness) -> Result<StageReconciliation, StageError> {
        let installed = match_image(
            &self.target.parent,
            &self.target.name,
            &witness.candidate,
            true,
        )?;
        match statat(
            &self.target.parent,
            witness.stage_name.as_str(),
            AtFlags::SYMLINK_NOFOLLOW,
        ) {
            Err(Errno::NOENT) => {}
            Ok(_) => return Err(StageError::Conflict),
            Err(error) => return Err(FsError::from(error).into()),
        }
        self.check_current()?;
        fsync(&installed).map_err(FsError::from)?;
        fsync(&self.target.parent).map_err(FsError::from)?;
        // Re-observe after synchronization; do not turn a stale digest into a receipt.
        let _current = match_image(
            &self.target.parent,
            &self.target.name,
            &witness.candidate,
            true,
        )?;
        self.check_current()?;
        Ok(StageReconciliation::InstalledObservedSync(InstalledStage {
            identity: witness.candidate.metadata.identity,
        }))
    }

    fn clean_uninstalled(&self, witness: &StageWitness) -> Result<StageReconciliation, StageError> {
        let original = match_image(
            &self.target.parent,
            &self.target.name,
            &witness.preimage,
            false,
        )?;
        let name = OsStr::new(&witness.stage_name);
        let staged = match_image(&self.target.parent, name, &witness.candidate, false)?;
        self.check_current()?;
        check_named(
            &self.target.parent,
            &self.target.name,
            &original,
            &witness.preimage.metadata,
        )?;
        check_named(
            &self.target.parent,
            name,
            &staged,
            &witness.candidate.metadata,
        )?;
        // Holding both descriptors prevents accidental inode reuse during cleanup.
        // Check/unlink still requires the private/cooperating namespace assumption.
        unlinkat(&self.target.parent, name, AtFlags::empty()).map_err(FsError::from)?;
        fsync(&self.target.parent).map_err(FsError::from)?;
        check_named(
            &self.target.parent,
            &self.target.name,
            &original,
            &witness.preimage.metadata,
        )?;
        self.check_current()?;
        Ok(StageReconciliation::NotInstalledCleaned)
    }
}
