//! Running one gate command: the argv as written (never through a shell),
//! in the definition's working directory, with the user's environment plus
//! `WRITFOLD_GATE` and `WRITFOLD_GATE_ARTIFACTS`, stdin at end of file, a
//! timeout, and the last bytes of stdout and stderr kept. The process group
//! is supervised by the runtime: nothing it starts outlives it.
use super::def::{ARTIFACTS, Definition};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use writfold_core::gate::Ending;
use writfold_runtime::process::{Tail, TailLimits, TailedEnd, run_tailed};

/// Bytes kept from the end of each stream.
pub const TAIL: usize = 64 << 10;
/// Between TERM and KILL at the deadline.
const GRACE: Duration = Duration::from_secs(5);

/// What one evaluation observed.
#[derive(Clone, Debug)]
pub struct Evaluation {
    pub ending: Ending,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    /// Why the command could not be started or supervised.
    pub error: Option<String>,
    pub duration_ms: u64,
    pub stdout: Tail,
    pub stderr: Tail,
}

/// The argv with `{artifacts}` replaced.
pub fn expand(argv: &[String], artifacts: &Path) -> Vec<String> {
    let dir = artifacts.to_string_lossy();
    argv.iter().map(|a| a.replace(ARTIFACTS, &dir)).collect()
}

/// Run `argv` (as recorded) for `gate` in `workdir` under `top`.
pub fn execute(
    gate: &str,
    argv: &[String],
    workdir: &str,
    timeout_seconds: u64,
    top: &Path,
    artifacts: &Path,
) -> Evaluation {
    let started = std::time::Instant::now();
    let failed = |why: String| Evaluation {
        ending: Ending::Error,
        exit_code: None,
        signal: None,
        timed_out: false,
        error: Some(why),
        duration_ms: millis(started.elapsed()),
        stdout: Tail::default(),
        stderr: Tail::default(),
    };
    let dir = top.join(workdir);
    if !dir.is_dir() {
        return failed(format!("working directory {workdir} does not exist"));
    }
    let argv = expand(argv, artifacts);
    let Some((program, args)) = argv.split_first() else {
        return failed("empty argv".into());
    };
    let program = if program.contains('/') {
        dir.join(program)
    } else {
        program.into()
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(&dir)
        .env("WRITFOLD_GATE", gate)
        .env("WRITFOLD_GATE_ARTIFACTS", artifacts);
    let limits = TailLimits {
        tail: TAIL,
        timeout: Duration::from_secs(timeout_seconds),
        grace: GRACE,
    };
    let supervisor = match crate::workspace::supervisor() {
        Ok(s) => s,
        Err(error) => return failed(error.to_string()),
    };
    match run_tailed(&mut command, limits, supervisor) {
        Ok(done) => {
            let (ending, exit_code, signal, timed_out) = match done.end {
                TailedEnd::Exited(code) => (Ending::Exited(code), Some(code), None, false),
                TailedEnd::Signaled(n) => (Ending::Signaled, None, Some(n), false),
                TailedEnd::TimedOut => (Ending::TimedOut, None, None, true),
            };
            Evaluation {
                ending,
                exit_code,
                signal,
                timed_out,
                error: None,
                duration_ms: millis(done.elapsed),
                stdout: done.stdout,
                stderr: done.stderr,
            }
        }
        Err(error) => failed(format!("could not run {}: {error}", argv[0])),
    }
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// Run a definition as it stands.
pub fn run(d: &Definition, top: &Path, artifacts: &Path) -> Evaluation {
    execute(
        &d.id,
        &d.argv,
        &d.workdir,
        d.timeout_seconds,
        top,
        artifacts,
    )
}
