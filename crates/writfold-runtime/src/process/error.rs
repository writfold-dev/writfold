use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("native process IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported process supervision profile: {0}")]
    UnsupportedProfile(&'static str),
    #[error("native process stdin closed")]
    ClosedStdin,
    #[error("native process {stream} byte budget exceeded (limit {limit})")]
    Budget { stream: &'static str, limit: u64 },
    #[error("native process total deadline elapsed")]
    Deadline,
    #[error("native process operation cancelled")]
    Cancelled,
    #[error("native process failed (status {status:?}); bounded stderr: {stderr:?}")]
    Command {
        status: Option<i32>,
        stderr: Vec<u8>,
    },
    #[error("native process child cleanup failed: {0}")]
    Cleanup(std::io::Error),
}

impl From<rustix::io::Errno> for ProcessError {
    fn from(value: rustix::io::Errno) -> Self {
        Self::Io(value.into())
    }
}
