//! `writfold tuft`: run a Tuft v0 program over one pinned Git commit.
//!
//! The pure `tuft` crate compiles the program. This adapter lowers `find`
//! onto the B0 storage boundary through the read-only pinned-Git-tree
//! backend, retains evidence through the trial retain path (the same
//! `writfold-citation/1` bindings `writfold trial retain` produces, stored in
//! the same trial state, so a Weft report can cite them and `trial prepare`
//! accepts them), then projects and emits. It writes nothing outside the
//! trial state's evidence and objects, and grants no authority.
#[cfg(test)]
mod backend_tests;
pub mod cli;
#[cfg(test)]
mod compact_tests;
#[cfg(test)]
mod json_tests;
mod search;
#[cfg(test)]
mod tests;

use crate::trial::retain::{PinnedBlob, read_pinned, retain_read};
use crate::trial::{Result, Trial};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tuft::emit::{Emitted, Format, Run, emit};
use tuft::locate::{around, line_at};
use tuft::view::{Evidence, Row, project};
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
    pub cursor: Option<&'a str>,
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

/// Compile and run one program; returns the emitted page.
pub fn run(invocation: &Invocation<'_>) -> Result<Emitted> {
    let plan = compile(invocation.source, invocation.name)?;
    let trial = Trial::open(invocation.state)?;
    let executable = trial.executable()?;
    let git_dir = std::fs::canonicalize(invocation.repo.join(".git"))?;
    let pin = commit(&plan.find.commit)?;
    let backend = GitTreeBackend::new(
        &executable,
        git_dir,
        plan.find.scope.clone(),
        budgets()?,
        MAX_SCOPE_BYTES,
    );
    let found = search::find(&backend, &pin, &plan.find)?;
    let (rows, retained) = match &plan.retain {
        None => (hit_rows(&found), None),
        Some(retain) => {
            let source = (invocation.repo, plan.find.commit.as_str());
            let (rows, count) = retain_rows(&trial, &executable, source, &found, retain.around)?;
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
    Ok(emit(run, invocation.cursor, invocation.format)?)
}

fn hit_rows(found: &search::Found) -> Vec<Row> {
    found
        .hits
        .iter()
        .map(|hit| Row {
            path: hit.path.clone(),
            range: (hit.range.0 as u64, hit.range.1 as u64),
            line: hit.line,
            evidence: None,
        })
        .collect()
}

/// Retain every hit (or its whole lines `around`), one pinned blob read per
/// file, and check the retained bytes against the bytes the search read.
fn retain_rows(
    trial: &Trial,
    executable: &GitExecutable<'_>,
    (repo, commit): (&Path, &str),
    found: &search::Found,
    widen: Option<usize>,
) -> Result<(Vec<Row>, usize)> {
    let mut by_path: BTreeMap<&str, Vec<(u64, u64)>> = BTreeMap::new();
    for hit in &found.hits {
        let content = found.files.get(&hit.path).ok_or("hit without file bytes")?;
        let (lo, hi) = match widen {
            None => hit.range,
            Some(n) => around(content, hit.range.0, hit.range.1, n),
        };
        by_path
            .entry(&hit.path)
            .or_default()
            .push((lo as u64, hi as u64));
    }
    let paths: Vec<&str> = by_path.keys().copied().collect();
    let reads = read_all(executable, repo, commit, &paths)?;
    let mut rows = Vec::new();
    let mut bindings = BTreeSet::new();
    for ((path, ranges), read) in by_path.into_iter().zip(reads) {
        let content = found.files.get(path).ok_or("hit without file bytes")?;
        if &read.content != content {
            return Err(
                format!("{path} read for retention differs from the searched bytes").into(),
            );
        }
        for retained in retain_read(trial, &read, &ranges)? {
            let lo = retained.range.0 as usize;
            bindings.insert(retained.binding.clone());
            rows.push(Row {
                path: path.to_owned(),
                range: retained.range,
                line: line_at(content, lo),
                evidence: Some(Evidence {
                    binding: retained.binding,
                    bytes: retained.bytes,
                }),
            });
        }
    }
    Ok((rows, bindings.len()))
}

/// Read each pinned file through the trial retain path, a few concurrently;
/// results keep the order of `paths`.
fn read_all(
    executable: &GitExecutable<'_>,
    repo: &Path,
    commit: &str,
    paths: &[&str],
) -> Result<Vec<PinnedBlob>> {
    const READERS: usize = 8;
    let chunk = paths.len().div_ceil(READERS).max(1);
    let parts: Vec<Result<Vec<PinnedBlob>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = paths
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|path| read_pinned(executable, repo, commit, path))
                        .collect::<Result<Vec<_>>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("pinned read thread failed".into()))
            })
            .collect()
    });
    let mut out = Vec::with_capacity(paths.len());
    for part in parts {
        out.extend(part?);
    }
    Ok(out)
}
