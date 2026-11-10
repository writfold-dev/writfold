//! Run an explicitly named Weft binary on one review source. Writfold does
//! not bundle Weft and never searches `PATH`: the binary is the canonical
//! path the operator gave, and its sha256 is recorded with the run.
//!
//! Weft's exit status is not read as a verdict (a complete proof exits 1 even
//! when entailed). Exits 0, 1 and 2 hand the report to `parse_report`, which
//! refuses anything but a complete `weft.report/1`; a signal, another status
//! or a timeout refuses here.
use crate::trial::{BUDGET, Result, limits};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use writfold_kernel::trial::Check;
use writfold_wire::weft_report::parse_report;

/// The longest a Weft check may run before it is killed and the run refuses.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// One Weft invocation: the exact binary, its digest and the report it wrote.
pub struct Assessed {
    pub bin: PathBuf,
    pub bin_sha256: String,
    pub report: Vec<u8>,
    pub checks: Vec<Check>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A Weft binary fixed by the operator before any request: its canonical path
/// and the sha256 it had then (`writfold serve --weft PATH`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pinned {
    pub bin: PathBuf,
    pub sha256: String,
}

fn canonical(bin: &Path) -> Result<PathBuf> {
    let canonical =
        std::fs::canonicalize(bin).map_err(|e| format!("Weft binary {}: {e}", bin.display()))?;
    if !canonical.is_file() {
        return Err(format!("Weft binary {} is not a file", canonical.display()).into());
    }
    Ok(canonical)
}

/// Resolve and digest `bin` once, at startup.
pub fn pin(bin: &Path) -> Result<Pinned> {
    let bin = canonical(bin)?;
    let sha256 = sha256_hex(&std::fs::read(&bin)?);
    Ok(Pinned { bin, sha256 })
}

/// Run `bin prove --json -- source` and read its report. With `pinned`, the
/// binary must still have that digest when it is read just before running.
pub fn assess(bin: &Path, source: &Path, pinned: Option<&str>) -> Result<Assessed> {
    let bin = canonical(bin)?;
    let bin_sha256 = sha256_hex(&std::fs::read(&bin)?);
    if let Some(pinned) = pinned.filter(|p| *p != bin_sha256) {
        return Err(format!(
            "the Weft binary {} changed since it was pinned (sha256 {bin_sha256}, pinned {pinned})",
            bin.display()
        )
        .into());
    }
    let mut child = Command::new(&bin)
        .args(["prove", "--json", "--"])
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", bin.display()))?;
    let stdout = child.stdout.take().ok_or("Weft stdout")?;
    let reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        stdout.take(BUDGET as u64 + 1).read_to_end(&mut bytes)?;
        Ok(bytes)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Weft did not finish within {}s", TIMEOUT.as_secs()).into());
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let report = reader.join().map_err(|_| "Weft reader thread failed")??;
    if report.len() > BUDGET {
        return Err(format!("the Weft report is larger than {BUDGET} bytes").into());
    }
    match status.code() {
        Some(0..=2) => {}
        Some(code) => return Err(format!("Weft exited with status {code}").into()),
        None => return Err("Weft was killed by a signal".into()),
    }
    let checks = parse_report(&report, limits()?)
        .map_err(|e| format!("report refused: {e}"))?
        .checks;
    Ok(Assessed {
        bin,
        bin_sha256,
        report,
        checks,
    })
}
