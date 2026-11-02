use super::read::open_regular;
use super::{ContainedRoot, FileIdentity, FsError, PinnedFile, ReadResult};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FlockOperation, OFlags, flock, fstat, fsync, statat};
use rustix::io::Errno;
use std::ffi::OsString;
use std::path::Path;
use writfold_domain::ByteCount;

/// A stable logical record key, not a pathname or a content revision. The app
/// must choose the same key across record renames and writer/recovery processes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LockKey(String);

impl LockKey {
    pub fn new(key: &str) -> Result<Self, FsError> {
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(FsError::InvalidLockKey);
        }
        Ok(Self(key.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A separately admitted, application-managed namespace shared by all readers,
/// writers and recovery. Entries must never be removed while clients can run.
#[derive(Debug)]
pub struct ManagedLocks {
    root: ContainedRoot,
}

impl ManagedLocks {
    /// Open an existing namespace without creating directories or lock entries.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FsError> {
        Ok(Self {
            root: ContainedRoot::open(path)?,
        })
    }

    /// Explicit initialization; missing ancestors are not created.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, FsError> {
        Ok(Self {
            root: ContainedRoot::create(path)?,
        })
    }
}

/// Closing this guard releases flock; its stable namespace entry is not unlinked.
#[derive(Debug)]
pub struct LockedFile<'a> {
    pub(super) target: &'a PinnedFile,
    namespace: &'a ManagedLocks,
    name: OsString,
    descriptor: OwnedFd,
    identity: FileIdentity,
}

impl PinnedFile {
    /// An explicit write-capability operation: creates a managed lock entry when
    /// absent. Busy is returned immediately; callers own retry/cancellation policy.
    pub fn try_lock<'a>(
        &'a self,
        namespace: &'a ManagedLocks,
        key: &LockKey,
    ) -> Result<LockedFile<'a>, FsError> {
        let name = OsString::from(format!("{}.lock", key.as_str()));
        let (descriptor, stamp) = open_regular(
            &namespace.root.directory,
            &name,
            OFlags::RDWR | OFlags::CREATE,
        )?;
        if stamp.links != 1 {
            return Err(FsError::HardLinked);
        }
        flock(&descriptor, FlockOperation::NonBlockingLockExclusive).map_err(|error| {
            if error == Errno::AGAIN {
                FsError::LockBusy
            } else {
                FsError::from(error)
            }
        })?;
        let guard = LockedFile {
            target: self,
            namespace,
            name,
            descriptor,
            identity: stamp.identity,
        };
        guard.ensure_lock()?;
        fsync(&guard.descriptor).map_err(FsError::from)?;
        fsync(&namespace.root.directory).map_err(FsError::from)?;
        Ok(guard)
    }
}

impl LockedFile<'_> {
    pub fn lock_identity(&self) -> FileIdentity {
        self.identity
    }

    /// Explicitly coordinated read, unlike ordinary PinnedFile::read. Acquiring
    /// this guard is a separate operation which may initialize its lock entry.
    pub fn read(&self, limit: ByteCount) -> Result<ReadResult, FsError> {
        self.ensure_lock()?;
        let result = self.target.read(limit)?;
        self.ensure_lock()?;
        Ok(result)
    }

    pub(super) fn ensure_lock(&self) -> Result<(), FsError> {
        let opened = fstat(&self.descriptor).map_err(FsError::from)?;
        let named = statat(
            &self.namespace.root.directory,
            &self.name,
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(|error| match error {
            Errno::NOENT => FsError::ChangedSource,
            _ => FsError::from(error),
        })?;
        if FileIdentity::of(&opened) != self.identity
            || FileIdentity::of(&named) != self.identity
            || opened.st_nlink != 1
            || named.st_nlink != 1
        {
            return Err(FsError::ChangedSource);
        }
        Ok(())
    }
}
