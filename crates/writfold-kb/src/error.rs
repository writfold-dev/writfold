use writfold_runtime::fs::FsError;
use writfold_runtime::git::GitError;
use writfold_runtime::process::ProcessError;

/// Why a KB verb could not answer. `Refused` (exit 3) is a typed refusal the
/// caller can act on; every other variant is an error (exit 1).
#[derive(Debug, thiserror::Error)]
pub enum KbError {
    #[error("{0}")]
    Usage(String),
    #[error("refused: {0}")]
    Refused(String),
    #[error("{0}")]
    NoKb(String),
    #[error("refused: {0}")]
    Cursor(#[from] tuft::cursor::CursorError),
    #[error("filesystem: {0}")]
    Io(#[from] std::io::Error),
    #[error("index: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("git: {0}")]
    Git(#[from] GitError),
    #[error("git: {0}")]
    Process(#[from] ProcessError),
    #[error("git: {0}")]
    Fs(#[from] FsError),
    #[error("index: {0}")]
    Mismatch(String),
    #[error("limit: {0}")]
    Quantity(#[from] writfold_domain::QuantityError),
}

impl KbError {
    /// 3 for a refusal, 1 for anything else.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Refused(_) | Self::Cursor(_) => 3,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, KbError>;
