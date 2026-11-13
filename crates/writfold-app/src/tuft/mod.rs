//! `writfold tuft`: run a Tuft v0 program over one pinned Git commit. The
//! pure `tuft` crate compiles it; this adapter lowers `find` onto the B0
//! boundary's read-only pinned-Git-tree backend, retains evidence through
//! the trial retain path (`writfold trial retain`'s bindings), then
//! projects and emits. Evidence goes to the trial state named by `--state`
//! or `WRITFOLD_STATE` (so a Weft report can cite it), else to the state
//! Tuft keeps in the KB's Git directory ([`state`]). It writes only there
//! and in a KB run's receipts (`kb`); it grants nothing.
#[cfg(test)]
mod backend_tests;
pub mod cli;
#[cfg(test)]
mod compact_tests;
#[cfg(test)]
mod json_tests;
pub mod kb;
#[cfg(test)]
mod kb_tests;
mod retain;
mod search;
pub mod state;
#[cfg(test)]
mod tests;

use crate::trial::retain::EvidenceStore;
use crate::trial::{Result, Trial};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tuft::emit::{Cursor, Emitted, Format, Run, emit};
use tuft::view::{Projected, project};
use tuft::{Deferred, Diagnostic, Plan};
use writfold_domain::{ByteCount, GitObjectFormat, GitObjectId};
use writfold_runtime::git::{GitBudgets, GitExecutable};
use writfold_store::backend::git::GitTreeBackend;
use writfold_store::backend::{Capability, Refusal};

pub static CANCELLED: AtomicBool = AtomicBool::new(false);
/// Largest scope (sum of blob bytes) one `find` searches; larger refuses.
pub const MAX_SCOPE_BYTES: u64 = 1 << 30;
const MAX_PROGRAM_BYTES: usize = 64 << 10;

/// A program that does not compile. `Display` is the rendered diagnostics.
#[derive(Debug)]
pub struct ProgramError {
    pub diagnostics: Vec<Diagnostic>,
    rendered: String,
}

impl ProgramError {
    /// Deferred operators as the B0 boundary's typed refusals.
    pub fn refusals(&self) -> Vec<Refusal> {
        self.diagnostics
            .iter()
            .filter_map(|d| match d.code {
                tuft::Code::Unsupported(which) => Some(Refusal::Unsupported(capability(which))),
                _ => None,
            })
            .collect()
    }
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.rendered)
    }
}

impl std::error::Error for ProgramError {}

pub fn capability(which: Deferred) -> Capability {
    match which {
        Deferred::Similar => Capability::Semantic,
        Deferred::Rank => Capability::Ranking,
        Deferred::Follow => Capability::Follow,
    }
}

pub fn compile(source: &[u8], name: &str) -> Result<Plan> {
    if source.len() > MAX_PROGRAM_BYTES {
        return Err(format!("program exceeds {MAX_PROGRAM_BYTES} bytes").into());
    }
    tuft::compile(source).map_err(|diagnostics| {
        let rendered = tuft::diag::render_all(&diagnostics, source, name);
        ProgramError {
            diagnostics,
            rendered,
        }
        .into()
    })
}

pub struct Invocation<'a> {
    pub source: &'a [u8],
    pub name: &'a str,
    pub repo: &'a Path,
    pub state: &'a Path,
    pub cursor: Option<&'a Cursor>,
    pub format: Format,
}

fn budgets() -> Result<GitBudgets> {
    let bytes = ByteCount::new(MAX_SCOPE_BYTES + (64 << 20))?;
    Ok(GitBudgets::new(
        bytes,
        ByteCount::new(1 << 16)?,
        bytes,
        Duration::from_secs(60),
    ))
}

fn commit(text: &str) -> Result<GitObjectId> {
    let format = match text.len() {
        40 => GitObjectFormat::Sha1,
        _ => GitObjectFormat::Sha256,
    };
    Ok(GitObjectId::new(format, text.to_owned())?)
}

/// One run: the plan, all projected rows (before paging) and the page.
pub struct Executed {
    pub plan: Plan,
    pub projected: Projected,
    pub emitted: Emitted,
}

/// Compile and run one program; returns the emitted page.
pub fn run(invocation: &Invocation<'_>) -> Result<Emitted> {
    execute(invocation).map(|executed| executed.emitted)
}
/// Compile and run one program in the trial state `invocation.state`,
/// keeping the projected rows.
pub fn execute(invocation: &Invocation<'_>) -> Result<Executed> {
    let trial = Trial::open(invocation.state)?;
    let executable = trial.executable()?;
    execute_in(invocation, &trial, &executable)
}

/// The same in Tuft's KB-owned state `invocation.state` ([`state`]), with
/// the Git executable the `kb` verbs use.
pub fn execute_kb(invocation: &Invocation<'_>) -> Result<Executed> {
    let store = state::TuftState::open(invocation.state)?;
    let git = writfold_kb::workspace::GitTool::locate()?;
    execute_in(invocation, &store, &git.admit()?)
}

fn execute_in(
    invocation: &Invocation<'_>,
    store: &dyn EvidenceStore,
    executable: &GitExecutable<'_>,
) -> Result<Executed> {
    let plan = compile(invocation.source, invocation.name)?;
    let git_dir = std::fs::canonicalize(invocation.repo.join(".git"))?;
    let pin = commit(&plan.find.commit)?;
    let backend = GitTreeBackend::new(
        executable,
        git_dir,
        plan.find.scope.clone(),
        budgets()?,
        MAX_SCOPE_BYTES,
    );
    let found = search::find(&backend, &pin, &plan.find)?;
    let (rows, retained) = match &plan.retain {
        None => (found.rows(), None),
        Some(retain) => {
            let source = (invocation.repo, plan.find.commit.as_str());
            let (rows, count) =
                retain::retain_rows(store, executable, source, &found, retain.around)?;
            (rows, Some(count))
        }
    };
    let projected = project(rows, &plan.view);
    let run = Run {
        plan: &plan,
        search: found.report,
        retained,
        projected: &projected,
    };
    let emitted = emit(run, invocation.cursor, invocation.format)?;
    Ok(Executed {
        plan,
        projected,
        emitted,
    })
}
