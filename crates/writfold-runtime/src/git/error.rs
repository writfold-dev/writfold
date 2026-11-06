use crate::fs::FsError;
use crate::process::ProcessError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error(transparent)]
    Filesystem(#[from] FsError),
    #[error("native Git IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("executable digest differs from the admitted tool profile")]
    ExecutableMismatch,
    #[error("unsupported managed repository profile: {0}")]
    UnsupportedProfile(&'static str),
    #[error("Git object format differs from the admitted repository format")]
    ObjectFormat,
    #[error("Git object type differs from the required type")]
    ObjectType,
    #[error("the exact path is absent from the specified commit")]
    MissingPath,
    #[error("the exact blob identity differs from the supplied identity")]
    ObjectMismatch,
    #[error("invalid native Git output: {0}")]
    Malformed(&'static str),
    #[error("native Git {stream} byte budget exceeded (limit {limit})")]
    Budget { stream: &'static str, limit: u64 },
    #[error(transparent)]
    Process(#[from] ProcessError),
}

impl From<rustix::io::Errno> for GitError {
    fn from(value: rustix::io::Errno) -> Self {
        Self::Io(value.into())
    }
}
