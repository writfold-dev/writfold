use crate::{
    fs::{FsError, WriteError},
    process::ProcessError,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DoltError {
    #[error(transparent)]
    Filesystem(#[from] FsError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error("writer claim record: {0}")]
    Claim(#[from] WriteError),
    #[error("Dolt executable digest differs from the external tool generation")]
    ExecutableMismatch,
    #[error("unsupported managed Dolt profile: {0}")]
    UnsupportedProfile(&'static str),
    #[error("Dolt {stream} byte budget exceeded (limit {limit})")]
    Budget { stream: &'static str, limit: u64 },
    #[error("Dolt query row-limit overflow")]
    RowLimit,
    #[error("invalid compiled Dolt query contract")]
    QueryContract,
}
