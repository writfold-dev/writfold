//! Owned Linux subprocess groups shared by native storage readers.
mod error;
mod runner;
mod supervisor;

pub use error::ProcessError;
pub(crate) use runner::{check_stop, run};
pub use supervisor::ProcessSupervisor;
use writfold_domain::ByteCount;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProcessLimits {
    stdout: ByteCount,
    stderr: ByteCount,
}

impl ProcessLimits {
    pub(crate) fn new(stdout: ByteCount, stderr: ByteCount) -> Self {
        Self { stdout, stderr }
    }
}
