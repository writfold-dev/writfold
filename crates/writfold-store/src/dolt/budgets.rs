use std::time::Duration;
use writfold_domain::{ByteCount, RowLimit};
use writfold_wire::Limits;

/// Per-command byte ceilings, per-table row ceiling, bounded decoding, and one
/// total deadline for the complete read group. No implicit unlimited mode.
#[derive(Clone, Copy, Debug)]
pub struct ReadBudgets {
    stdout: ByteCount,
    stderr: ByteCount,
    rows: RowLimit,
    wire: Limits,
    timeout: Duration,
}

impl ReadBudgets {
    pub fn new(
        stdout: ByteCount,
        stderr: ByteCount,
        rows: RowLimit,
        wire: Limits,
        timeout: Duration,
    ) -> Self {
        Self {
            stdout,
            stderr,
            rows,
            wire,
            timeout,
        }
    }

    pub fn stdout(self) -> ByteCount {
        self.stdout
    }

    pub fn stderr(self) -> ByteCount {
        self.stderr
    }

    pub fn rows(self) -> RowLimit {
        self.rows
    }

    pub fn wire(self) -> Limits {
        self.wire
    }

    pub fn timeout(self) -> Duration {
        self.timeout
    }
}
