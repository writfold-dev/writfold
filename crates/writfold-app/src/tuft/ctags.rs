//! Finding, probing and running universal-ctags for `writfold tuft index`.
//! `ctags` is `$WRITFOLD_CTAGS`, else `ctags` on `PATH`; Writfold links
//! nothing. Every run is one supervised process group (TERM at the
//! deadline, then KILL), as checkers and gate commands are.
use crate::trial::Result;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use writfold_runtime::process::{TailLimits, TailedEnd, run_tailed_input};

/// Default deadline of the indexing run, in seconds.
pub const TIMEOUT: u64 = 300;
/// Deadline of `--version` and `--list-features`.
const PROBE: u64 = 30;

fn executable(file: &Path) -> bool {
    std::fs::metadata(file).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// The ctags program, or why there is none.
pub fn find() -> Result<PathBuf> {
    if let Some(named) = std::env::var_os("WRITFOLD_CTAGS").filter(|v| !v.is_empty()) {
        let file = PathBuf::from(&named);
        return match executable(&file) {
            true => Ok(file),
            false => Err(format!(
                "WRITFOLD_CTAGS={} is not an executable file; universal-ctags with JSON output is needed",
                file.display()
            )
            .into()),
        };
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("ctags"))
        .find(|file| executable(file))
        .ok_or_else(|| {
            "no ctags on PATH (or set WRITFOLD_CTAGS); universal-ctags with JSON output is needed"
                .into()
        })
}

/// The run's deadline: `WRITFOLD_CTAGS_TIMEOUT` seconds (1 to 86400), else
/// [`TIMEOUT`].
pub fn timeout() -> u64 {
    std::env::var("WRITFOLD_CTAGS_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| (1..=86_400).contains(n))
        .unwrap_or(TIMEOUT)
}

/// Run `program args…` in `dir`; its stdout, or why it failed.
pub fn run(program: &Path, args: &[&str], dir: &Path, seconds: u64) -> Result<String> {
    let supervisor = writfold_kb::workspace::supervisor()?;
    let limits = TailLimits {
        tail: 1 << 20,
        timeout: Duration::from_secs(seconds),
        grace: Duration::from_secs(2),
    };
    let mut command = Command::new(program);
    command.args(args).current_dir(dir);
    let done = run_tailed_input(&mut command, &[], limits, supervisor)
        .map_err(|e| format!("could not run {}: {e}", program.display()))?;
    let shown = program.display();
    match done.end {
        TailedEnd::Exited(0) => Ok(String::from_utf8_lossy(&done.stdout.bytes).into_owned()),
        TailedEnd::Exited(code) => Err(format!(
            "{shown} {} exited {code}: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&done.stderr.bytes).trim()
        )
        .into()),
        TailedEnd::Signaled(n) => Err(format!("{shown} was killed by signal {n}").into()),
        TailedEnd::TimedOut => {
            Err(format!("{shown} timed out after {seconds} s and was stopped").into())
        }
    }
}

/// The `--version` line of a universal-ctags that lists the `json`
/// feature, or why `program` is not one.
pub fn probe(program: &Path, dir: &Path) -> Result<String> {
    let version = run(program, &["--version"], dir, PROBE)?;
    let line = version.lines().next().unwrap_or_default().trim().to_owned();
    if !line.contains("Universal Ctags") {
        return Err(format!(
            "{} is not universal-ctags (--version says {line:?}); universal-ctags with JSON output is needed",
            program.display()
        )
        .into());
    }
    let features = run(program, &["--list-features"], dir, PROBE)?;
    let json = features
        .lines()
        .any(|l| l.split_whitespace().next() == Some("json"));
    if !json {
        return Err(format!(
            "{} ({line}) does not list the json feature in --list-features; build it with JSON output (libjansson)",
            program.display()
        )
        .into());
    }
    Ok(line)
}
