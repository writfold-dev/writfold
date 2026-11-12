//! Finding and running the pinned Weft binary. Its path comes from the
//! machine, never from the agent or the definition: `$WRITFOLD_WEFT`, else
//! `weft = "<path>"` in `$XDG_CONFIG_HOME/writfold/config.toml` (or
//! `~/.config/writfold/config.toml`). Writfold never searches `PATH`. The
//! definition pins the binary's SHA-256; a binary with another digest is
//! refused before it runs, as the trial's `--weft` pin is.
//!
//! Weft runs as `weft prove --json -- <source>` under the gate supervision
//! (its own process group, stdin at end of file, the definition's timeout,
//! TERM then KILL). Its exit status is not read as a verdict: a complete
//! proof exits 1 even when entailed, so 0, 1 and 2 hand the report on.
use crate::error::{KbError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use writfold_runtime::process::{TailLimits, TailedEnd, run_tailed};

/// The environment variable naming the Weft binary.
pub const ENV: &str = "WRITFOLD_WEFT";
/// The largest report kept: Weft's own output budget.
const REPORT_LIMIT: usize = 16 << 20;
const GRACE: Duration = Duration::from_secs(5);

/// The binary's path from the machine's configuration, if any.
fn configured() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    let text = std::fs::read_to_string(base.join("writfold").join("config.toml")).ok()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        let value = value.trim();
        (key.trim() == "weft" && value.len() > 1 && value.starts_with('"') && value.ends_with('"'))
            .then(|| PathBuf::from(&value[1..value.len() - 1]))
    })
}

/// Where the binary is said to be, or why there is none.
pub fn locate() -> std::result::Result<PathBuf, String> {
    std::env::var_os(ENV)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(configured)
        .ok_or_else(|| {
            format!(
                "no Weft binary: set {ENV} or weft = \"<path>\" in ~/.config/writfold/config.toml"
            )
        })
}

/// `sha256:<hex>` of the binary's bytes.
pub fn digest(bin: &Path) -> std::result::Result<String, String> {
    let bytes = std::fs::read(bin).map_err(|e| format!("Weft binary {}: {e}", bin.display()))?;
    Ok(format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(&bytes))
    ))
}

/// What running Weft produced.
pub struct Ran {
    /// `sha256:<hex>` of the binary that ran, when it could be read.
    pub binary: Option<String>,
    /// The complete report, when Weft ended with 0, 1 or 2 and printed it
    /// within the limit.
    pub report: Option<Vec<u8>>,
    /// Why there is no report: no binary, a timeout, a signal, another
    /// exit status, a report over the limit.
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl Ran {
    fn failed(binary: Option<String>, why: String, started: std::time::Instant) -> Self {
        Ran {
            binary,
            report: None,
            error: Some(why),
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }
}

/// Run the pinned binary on `source`. A binary whose digest is not `pin`
/// is refused (an error, and nothing runs); a missing binary is a `Ran`
/// with no report, which the caller records as unknown.
pub fn run(pin: &str, source: &Path, timeout_seconds: u64) -> Result<Ran> {
    let started = std::time::Instant::now();
    let bin = match locate().and_then(|b| {
        std::fs::canonicalize(&b).map_err(|e| format!("Weft binary {}: {e}", b.display()))
    }) {
        Ok(bin) if bin.is_file() => bin,
        Ok(bin) => {
            return Ok(Ran::failed(
                None,
                format!("Weft binary {} is not a file", bin.display()),
                started,
            ));
        }
        Err(why) => return Ok(Ran::failed(None, why, started)),
    };
    let binary = match digest(&bin) {
        Ok(d) => d,
        Err(why) => return Ok(Ran::failed(None, why, started)),
    };
    if binary != pin {
        return Err(KbError::Refused(format!(
            "the Weft binary {} has {binary}; the gate pins {pin}",
            bin.display()
        )));
    }
    let mut command = Command::new(&bin);
    command.args(["prove", "--json", "--"]).arg(source);
    let limits = TailLimits {
        tail: REPORT_LIMIT,
        timeout: Duration::from_secs(timeout_seconds),
        grace: GRACE,
    };
    let supervisor = crate::workspace::supervisor()?;
    let done = match run_tailed(&mut command, limits, supervisor) {
        Ok(done) => done,
        Err(e) => {
            return Ok(Ran::failed(
                Some(binary),
                format!("could not run Weft: {e}"),
                started,
            ));
        }
    };
    let kept = u64::try_from(done.stdout.bytes.len()).unwrap_or(u64::MAX);
    let why = match done.end {
        TailedEnd::Exited(0..=2) if done.stdout.total > kept => Some(format!(
            "the Weft report is larger than {REPORT_LIMIT} bytes"
        )),
        TailedEnd::Exited(0..=2) => None,
        TailedEnd::Exited(code) => Some(format!("Weft exited with status {code}")),
        TailedEnd::Signaled(n) => Some(format!("Weft was killed by signal {n}")),
        TailedEnd::TimedOut => Some(format!("Weft ran past {timeout_seconds}s and was stopped")),
    };
    if let Some(why) = why {
        return Ok(Ran::failed(Some(binary), why, started));
    }
    Ok(Ran {
        binary: Some(binary),
        report: Some(done.stdout.bytes),
        error: None,
        duration_ms: u64::try_from(done.elapsed.as_millis()).unwrap_or(u64::MAX),
    })
}
