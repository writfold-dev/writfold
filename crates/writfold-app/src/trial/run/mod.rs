//! `writfold trial run`: evidence to a prepared edit in one command, stopping
//! at approval. Each step calls exactly the function its own command calls:
//!
//! 1. `tuft`: [`crate::tuft::execute`] runs the program; `retain` keeps its
//!    hits as `writfold-citation/1` bindings in this state.
//! 2. `weft` (optional): [`placeholder::substitute`] replaces each
//!    `citation "tuft:@VIEW#N"` template with that row's binding digest, the
//!    substituted source is retained, and the named Weft binary writes a
//!    `weft.report/1` ([`weft::assess`]).
//! 3. `prepare`: exactly [`crate::trial::prepare`] with that report.
//!
//! A failure stops the run and names its step. Approve, apply and recover
//! stay separate commands; this one never approves.
pub mod placeholder;
#[cfg(test)]
mod tests;
pub mod weft;

use super::{Prepared, Result, Trial, prepare};
use crate::tuft::{Executed, Invocation, execute};
use placeholder::Resolved;
use std::path::{Path, PathBuf};
use tuft::emit::Format;
use writfold_kernel::trial::{Check, CheckResult};

/// A failed step: which one, and why.
#[derive(Debug)]
pub struct StepError {
    pub step: &'static str,
    pub cause: Box<dyn std::error::Error + Send + Sync>,
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "step {} refused: {}", self.step, self.cause)
    }
}

impl std::error::Error for StepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

fn step<T>(name: &'static str, result: Result<T>) -> Result<T> {
    result.map_err(|cause| StepError { step: name, cause }.into())
}

pub struct Request<'a> {
    pub state: &'a Path,
    pub repo: &'a Path,
    pub file: &'a str,
    pub recipe: &'a Path,
    pub tuft: Option<&'a Path>,
    /// The Weft review source and the Weft binary to run it with.
    pub weft: Option<(&'a Path, &'a Path)>,
    /// The sha256 the Weft binary must have (pinned when `serve` started);
    /// `None` on the CLI, where the operator names the binary per command.
    pub weft_sha256: Option<&'a str>,
}

pub struct WeftStep {
    pub bin: PathBuf,
    pub bin_sha256: String,
    /// sha256 of the substituted source Weft read, retained in this state.
    pub source_sha256: String,
    pub resolved: Vec<Resolved>,
    pub checks: Vec<Check>,
}

pub struct Ran {
    pub tuft: Option<Executed>,
    pub weft: Option<WeftStep>,
    pub prepared: Prepared,
}

fn run_tuft(request: &Request<'_>, program: &Path) -> Result<Executed> {
    let source = std::fs::read(program)?;
    let name = program.display().to_string();
    execute(&Invocation {
        source: &source,
        name: &name,
        repo: request.repo,
        state: request.state,
        cursor: None,
        format: Format::Compact,
        working_tree: false,
        index: None,
    })
}

/// Substitute, retain and write the source Weft reads, then run Weft and
/// write its report where `prepare` reads it. Returns the report path.
fn run_weft(
    request: &Request<'_>,
    tuft: Option<&Executed>,
    (review, bin): (&Path, &Path),
) -> Result<(WeftStep, PathBuf)> {
    let text = String::from_utf8(std::fs::read(review)?)
        .map_err(|_| format!("{} is not UTF-8", review.display()))?;
    let view = tuft.map(|t| (t.plan.view.binding.as_str(), t.projected.rows.as_slice()));
    let (source, resolved) = placeholder::substitute(&text, view)?;
    let trial = Trial::open(request.state)?;
    trial.retain(source.as_bytes())?;
    let source_sha256 = weft::sha256_hex(source.as_bytes());
    let dir = request.state.join("weft");
    std::fs::create_dir_all(&dir)?;
    let source_path = dir.join(format!("{source_sha256}.weft"));
    std::fs::write(&source_path, &source)?;
    let assessed = weft::assess(bin, &source_path, request.weft_sha256)?;
    let report_path = dir.join(format!(
        "{}.report.json",
        weft::sha256_hex(&assessed.report)
    ));
    std::fs::write(&report_path, &assessed.report)?;
    let step = WeftStep {
        bin: assessed.bin,
        bin_sha256: assessed.bin_sha256,
        source_sha256,
        resolved,
        checks: assessed.checks,
    };
    Ok((step, report_path))
}

/// Run the steps in order; the first failure stops the run.
pub fn run(request: &Request<'_>) -> Result<Ran> {
    let tuft = request
        .tuft
        .map(|program| step("tuft", run_tuft(request, program)))
        .transpose()?;
    let weft = request
        .weft
        .map(|weft| step("weft", run_weft(request, tuft.as_ref(), weft)))
        .transpose()?;
    let report = weft.as_ref().map(|(_, path)| path.as_path());
    let prepared = step(
        "prepare",
        prepare(
            request.state,
            request.repo,
            request.file,
            request.recipe,
            report,
        ),
    )?;
    Ok(Ran {
        tuft,
        weft: weft.map(|(step, _)| step),
        prepared,
    })
}

/// The one compact summary `trial run` prints.
pub fn render(ran: &Ran) -> String {
    let mut out = String::new();
    if let Some(tuft) = &ran.tuft {
        let bindings: std::collections::BTreeSet<_> = tuft
            .projected
            .rows
            .iter()
            .filter_map(|row| row.evidence.as_ref().map(|e| &e.binding))
            .collect();
        let trailer = tuft.emitted.trailer.trim_start_matches("# ");
        out.push_str(&format!(
            "tuft @{} retained={} {trailer}\n",
            tuft.plan.view.binding,
            bindings.len()
        ));
    }
    let mut require = String::new();
    if let Some(weft) = &ran.weft {
        for r in &weft.resolved {
            out.push_str(&format!(
                "  {} = {} ({}:{})\n",
                r.placeholder, r.binding, r.path, r.line
            ));
        }
        out.push_str(&format!(
            "weft bin sha256:{} source sha256:{} (results are hypothetical)\n",
            weft.bin_sha256, weft.source_sha256
        ));
        for check in &weft.checks {
            out.push_str(&format!(
                "  check {}: {}\n",
                check.name,
                check.result.as_str()
            ));
            if check.result == CheckResult::Entailed {
                require.push_str(&format!(" --require {}", check.name));
            }
        }
    }
    let p = &ran.prepared;
    out.push_str(&format!(
        "prepared run {}\n  file {} at {}\n  prepared edit {}\n",
        p.run, p.path, p.base, p.prepared
    ));
    out.push_str(&format!("next: writfold trial approve {}{require}", p.run));
    out
}
