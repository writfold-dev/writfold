//! Finding and running one checker. A checker named `<name>` is the
//! executable `writfold-check-<name>` on `PATH` (as Git finds credential
//! helpers); the KB holds no command line and no secret. Finding it stats
//! `PATH` only. Running it is one supervised process group per exchange:
//! the requests are written to stdin, which is then closed, and the replies
//! are read from stdout until it exits or the deadline passes (TERM, a
//! grace period, then KILL), so nothing it started outlives it.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use writfold_runtime::process::{TailLimits, TailedEnd, run_tailed_input};

/// The executable's name for a checker name.
pub fn program(name: &str) -> String {
    format!("writfold-check-{name}")
}

/// The first executable `writfold-check-<name>` on `PATH`.
pub fn find(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(program(name)))
        .find(|file| {
            std::fs::metadata(file)
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}

/// Default deadline of one exchange, in seconds.
pub const TIMEOUT: u64 = 60;
/// Between TERM and KILL at the deadline.
const GRACE: Duration = Duration::from_secs(2);
/// Reply bytes accepted from one exchange.
const LIMIT: usize = 16 << 20;

/// The exchange's deadline: `WRITFOLD_CHECKER_TIMEOUT` seconds (1 to 3600),
/// else [`TIMEOUT`].
pub fn timeout() -> u64 {
    std::env::var("WRITFOLD_CHECKER_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| (1..=3600).contains(n))
        .unwrap_or(TIMEOUT)
}

/// What one exchange produced: its reply lines, and why the run itself
/// failed (a timeout, a non-zero exit, a signal, too much output), if it
/// did.
#[derive(Clone, Debug, Default)]
pub struct Ran {
    pub lines: Vec<String>,
    pub failed: Option<String>,
}

/// Run `program` in `dir` with `input` on stdin.
pub fn run(program: &Path, dir: &Path, input: &str) -> Ran {
    let failed = |why: String| Ran {
        lines: Vec::new(),
        failed: Some(why),
    };
    let supervisor = match crate::workspace::supervisor() {
        Ok(s) => s,
        Err(error) => return failed(error.to_string()),
    };
    let seconds = timeout();
    let limits = TailLimits {
        tail: LIMIT,
        timeout: Duration::from_secs(seconds),
        grace: GRACE,
    };
    let mut command = Command::new(program);
    command.current_dir(dir);
    let done = match run_tailed_input(&mut command, input.as_bytes(), limits, supervisor) {
        Ok(done) => done,
        Err(error) => return failed(format!("could not run: {error}")),
    };
    let why = match done.end {
        TailedEnd::TimedOut => Some(format!("timed out after {seconds} s")),
        TailedEnd::Signaled(n) => Some(format!("killed by signal {n}")),
        TailedEnd::Exited(0) if done.stdout.total > LIMIT as u64 => {
            Some(format!("more than {LIMIT} bytes of replies"))
        }
        TailedEnd::Exited(0) => None,
        TailedEnd::Exited(code) => Some(format!("exit {code}")),
    };
    Ran {
        lines: String::from_utf8_lossy(&done.stdout.bytes)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_owned)
            .collect(),
        failed: why,
    }
}
