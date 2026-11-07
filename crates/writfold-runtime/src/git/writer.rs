use super::{
    GitBudgets, GitError, GitExecutable, GitRepository, command, objects::Session, writer_profile,
};
use crate::fs::{
    ContainedRoot, FsError, LockKey, LockedFile, ManagedLocks, PinnedFile, RelativePath,
};
use crate::process::{self, ProcessLimits};
use rustix::fs::{Mode, fstat, fsync, mkdirat};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use thiserror::Error;
use writfold_domain::{ByteCount, GitObjectFormat, GitObjectId};

#[derive(Debug, Error)]
pub enum GitWriteError {
    #[error("native mutation was not issued: {0}")]
    Unissued(#[source] GitError),
    #[error("native mutation may have occurred; retain and reconcile, never blindly retry: {0}")]
    Unknown(#[source] GitError),
    #[error("completed predecessor conflict (observed {actual:?})")]
    Conflict { actual: Option<GitObjectId> },
    #[error("operation already has a prepared retention root: {0:?}")]
    PreparedExists(GitObjectId),
}

/// Separate owner-controlled writer admission, never granted by a read pin.
/// Private root and permanent `writfold-locks/knowledge.lock` are required.
/// Same-uid administrative mutation and noncooperating writers/GC are excluded.
#[derive(Debug)]
pub struct GitWriter {
    pub(super) repository: GitRepository,
    locks: ManagedLocks,
    target: PinnedFile,
    key: LockKey,
}

/// Exclusive cooperating-writer lease. Observed native facts are not approval.
#[derive(Debug)]
pub struct GitWriterLease<'a> {
    pub(super) writer: &'a GitWriter,
    guard: LockedFile<'a>,
}

impl GitWriter {
    /// Open only: does not initialize a repository or lock directory.
    pub fn open(
        root: ContainedRoot,
        format: GitObjectFormat,
        metadata_limit: ByteCount,
    ) -> Result<Self, GitError> {
        let repository = GitRepository::admit(root, format, metadata_limit)?;
        writer_profile::check(&repository)?;
        let locks_root = repository
            .root
            .open_child(&relative(writer_profile::LOCKS)?)?;
        writer_profile::private_root(&locks_root)?;
        let locks = ManagedLocks::from_root(locks_root);
        let target = repository.root.pin(&relative("config")?)?;
        Ok(Self {
            repository,
            locks,
            target,
            key: LockKey::new("knowledge")?,
        })
    }

    /// Explicit new bare SHA-256 repository only; the leaf must not exist.
    /// Failed attempts remain on disk. Fsync is a process-crash boundary, not a
    /// claim about storage hardware, power loss, or application authorization.
    pub fn initialize(
        path: impl AsRef<Path>,
        executable: &GitExecutable<'_>,
        metadata_limit: ByteCount,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<Self, GitWriteError> {
        let deadline = deadline(budgets).map_err(GitWriteError::Unissued)?;
        process::check_stop(deadline, cancelled)
            .map_err(GitError::from)
            .map_err(GitWriteError::Unissued)?;
        executable.path().map_err(GitWriteError::Unissued)?;
        let root = ContainedRoot::create(path)
            .map_err(GitError::from)
            .map_err(GitWriteError::Unknown)?;
        Self::initialize_root(
            root,
            executable,
            metadata_limit,
            budgets,
            cancelled,
            deadline,
        )
        .map_err(GitWriteError::Unknown)
    }

    fn initialize_root(
        root: ContainedRoot,
        executable: &GitExecutable<'_>,
        metadata_limit: ByteCount,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
        deadline: Instant,
    ) -> Result<Self, GitError> {
        writer_profile::private_root(&root)?;
        let path = super::admission::descriptor_path(root.descriptor());
        let mut native = command::command(executable, &path)?;
        native
            .args([
                "init",
                "--bare",
                "--template=",
                "--object-format=sha256",
                "--initial-branch=unused",
            ])
            .arg(&path);
        process::run(
            &mut native,
            &[],
            ProcessLimits::new(budgets.stdout, budgets.stderr),
            deadline,
            cancelled,
            executable.supervisor(),
        )?;
        executable.path()?;
        mkdirat(root.descriptor(), writer_profile::LOCKS, Mode::RWXU)?;
        let child = root.open_child(&relative(writer_profile::LOCKS)?)?;
        fsync(child.descriptor())?;
        fsync(root.descriptor())?;
        let writer = Self::open(root, GitObjectFormat::Sha256, metadata_limit)?;
        writer.try_lease()?.check()?;
        process::check_stop(deadline, cancelled)?;
        Ok(writer)
    }

    pub fn repository(&self) -> &GitRepository {
        &self.repository
    }

    /// Creates only the permanent stable lock entry if absent; never removes it.
    pub fn try_lease(&self) -> Result<GitWriterLease<'_>, GitError> {
        writer_profile::check(&self.repository)?;
        let guard = self.target.try_lock(&self.locks, &self.key)?;
        let lease = GitWriterLease {
            writer: self,
            guard,
        };
        lease.check()?;
        Ok(lease)
    }
}

impl GitWriterLease<'_> {
    pub(super) fn check(&self) -> Result<(), GitError> {
        self.guard.check_current()?;
        writer_profile::check(&self.writer.repository)?;
        writer_profile::private_root(
            &self
                .writer
                .repository
                .root
                .open_child(&relative(writer_profile::LOCKS)?)?,
        )?;
        Ok(())
    }

    pub(super) fn session<'a, 'file>(
        &'a self,
        executable: &'a GitExecutable<'file>,
        budgets: GitBudgets,
        cancelled: &'a AtomicBool,
    ) -> Result<Session<'a, 'file>, GitError> {
        let session = Session::new(&self.writer.repository, executable, budgets, cancelled)?;
        self.check()?;
        session.check_stop()?;
        Ok(session)
    }

    pub(super) fn identity(&self) -> Result<(u64, u64), GitError> {
        let stat = fstat(self.writer.repository.root.descriptor())?;
        Ok((stat.st_dev, stat.st_ino))
    }
}

pub(super) fn relative(path: &str) -> Result<RelativePath, GitError> {
    RelativePath::new(path)
        .map_err(FsError::from)
        .map_err(GitError::from)
}

fn deadline(budgets: GitBudgets) -> Result<Instant, GitError> {
    Instant::now()
        .checked_add(budgets.timeout)
        .ok_or(GitError::Malformed("deadline overflow"))
}
