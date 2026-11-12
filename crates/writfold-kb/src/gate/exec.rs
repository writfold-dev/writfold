//! Running one gate command: the argv as written (never through a shell),
//! in the definition's working directory, with the user's environment plus
//! `WRITFOLD_GATE` and `WRITFOLD_GATE_ARTIFACTS`, stdin at end of file, a
//! timeout, and the last bytes of stdout and stderr kept. The process group
//! is supervised by the runtime: nothing it starts outlives it.
use super::def::Definition;
use super::places::{Places, expand};
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

/// Run `argv` (as recorded) for `gate` in `workdir` under `top`, with its
/// placeholders expanded from `places`.
pub fn execute(
    gate: &str,
    argv: &[String],
    workdir: &str,
    timeout_seconds: u64,
    top: &Path,
    places: &Places<'_>,
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
    let argv = expand(argv, places);
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
        .env("WRITFOLD_GATE_DIR", places.gate);
    if let Some(artifacts) = places.artifacts {
        command.env("WRITFOLD_GATE_ARTIFACTS", artifacts);
    }
    if let Some(candidate) = places.candidate {
        command.env("WRITFOLD_CANDIDATE", candidate);
    }
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

/// Run a repository gate's definition as it stands on `top`.
pub fn run(d: &Definition, top: &Path, artifacts: &Path) -> Evaluation {
    let places = Places {
        gate: &d.dir,
        artifacts: Some(artifacts),
        candidate: None,
    };
    execute(&d.id, &d.argv, &d.workdir, d.timeout_seconds, top, &places)
}

/// Run a candidate gate's definition against the payload file
/// `candidate`, in its working directory under the KB root `root`.
pub fn run_on_candidate(d: &Definition, root: &Path, candidate: &Path) -> Evaluation {
    let places = Places {
        gate: &d.dir,
        artifacts: None,
        candidate: Some(candidate),
    };
    execute(&d.id, &d.argv, &d.workdir, d.timeout_seconds, root, &places)
}
