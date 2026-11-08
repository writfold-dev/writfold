use rustix::io::Errno;
use thiserror::Error;
use writfold_domain::{ContentDigest, IdentityError};

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum PathError {
    #[error("a contained path must be relative")]
    Absolute,
    #[error("empty paths and empty path components are forbidden")]
    EmptyComponent,
    #[error("dot and parent components are forbidden")]
    DotComponent,
    #[error("NUL bytes are forbidden in filesystem paths")]
    Nul,
    #[error("exactly one child path component is required")]
    MultipleComponents,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetKind {
    Read,
    Write,
    Allocation,
    TemporaryNames,
}

#[derive(Debug, Error)]
pub enum FsError {
    #[error(transparent)]
    InvalidPath(#[from] PathError),
    #[error("lock keys require 1..=128 ASCII letters, digits, hyphens or underscores")]
    InvalidLockKey,
    #[error("filesystem entry is missing")]
    Missing,
    #[error("filesystem access was denied")]
    Permission,
    #[error("symbolic links are refused")]
    Symlink,
    #[error("a regular file is required")]
    NonRegular,
    #[error("a directory is required")]
    NotDirectory,
    #[error("the opened source or its directory entry changed")]
    ChangedSource,
    #[error("multiply linked files are refused for this operation")]
    HardLinked,
    #[error("the cooperating-writer lock is busy")]
    LockBusy,
    #[error("the destination already exists")]
    AlreadyExists,
    #[error("the expected content revision is stale")]
    StaleRevision {
        expected: Option<ContentDigest>,
        actual: Option<ContentDigest>,
    },
    #[error("{kind:?} budget exceeded (limit {limit})")]
    Budget { kind: BudgetKind, limit: u64 },
    #[error("the filesystem does not support the required operation: {0}")]
    Unsupported(#[source] Errno),
    #[error("filesystem IO failed: {0}")]
    Io(#[source] Errno),
    #[error("a generated digest failed domain validation: {0}")]
    Digest(#[source] IdentityError),
    #[error("a directory is visible but its synchronization is unknown: {0}")]
    RootDurabilityUnknown(#[source] Errno),
}

impl From<Errno> for FsError {
    fn from(error: Errno) -> Self {
        match error {
            Errno::NOENT => Self::Missing,
            Errno::ACCESS | Errno::PERM | Errno::ROFS => Self::Permission,
            Errno::LOOP => Self::Symlink,
            Errno::NOTDIR => Self::NotDirectory,
            Errno::EXIST => Self::AlreadyExists,
            Errno::NOSYS | Errno::OPNOTSUPP => Self::Unsupported(error),
            _ => Self::Io(error),
        }
    }
}

/// A failed write never contains a successful synchronization receipt.
#[derive(Debug, Error)]
pub enum WriteError {
    #[error("the operation did not publish: {0}")]
    NotPublished(#[source] FsError),
    #[error("new bytes became visible, but durability is unknown: {source}")]
    PublishedButDurabilityUnknown {
        digest: ContentDigest,
        bytes: usize,
        #[source]
        source: FsError,
    },
}

impl From<FsError> for WriteError {
    fn from(error: FsError) -> Self {
        Self::NotPublished(error)
    }
}
