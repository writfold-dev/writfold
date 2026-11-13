//! Exact scoped snapshots for glob and index searches.
use super::search::{Collector, Found};
use super::{CANCELLED, MAX_SCOPE_BYTES, budgets};
use crate::trial::Result;
use std::collections::BTreeMap;
use std::path::Path;
use tuft::ir::{FindPlan, MatcherPlan};
use tuft::view::Stop;
use writfold_domain::GitObjectId;
use writfold_domain::citation::Coverage;
use writfold_runtime::git::{GitExecutable, pinned_blobs, pinned_tree};

pub(super) fn snapshot(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    pin: &GitObjectId,
    scope: &str,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let entries = pinned_tree(executable, git_dir, pin, budgets()?, &CANCELLED)?;
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| tuft::scope::matches(scope, &entry.path))
        .collect();
    let ids: Vec<_> = entries.iter().map(|entry| entry.blob.clone()).collect();
    let contents = pinned_blobs(executable, git_dir, &ids, budgets()?, &CANCELLED)?;
    let total: u64 = contents.iter().map(|content| content.len() as u64).sum();
    if total > MAX_SCOPE_BYTES {
        return Err("scope exceeds byte budget; narrow the scope".into());
    }
    Ok(entries
        .into_iter()
        .map(|entry| entry.path)
        .zip(contents)
        .collect())
}

pub(super) fn matches(plan: &FindPlan, bytes: &[u8]) -> Result<Vec<(usize, usize)>> {
    match &plan.matcher {
        MatcherPlan::Text(needle) => Ok(tuft::locate::text_matches(bytes, needle.as_bytes())),
        MatcherPlan::Rx(pattern) => pattern.matches(bytes).map_err(|zero| {
            format!(
                "rx matched zero bytes at byte {}; refused (no empty ranges)",
                zero.offset
            )
            .into()
        }),
        MatcherPlan::Symbol { .. } => Err("symbol search requires a pinned index".into()),
    }
}

pub(super) fn find(files: BTreeMap<String, Vec<u8>>, plan: &FindPlan) -> Result<Found> {
    let total = files.len();
    let mut collector = Collector::new(plan);
    for (examined, (path, bytes)) in files.into_iter().enumerate() {
        if !collector.add(&path, &bytes, &matches(plan, &bytes)?) {
            return Ok(collector.finish(
                Coverage::Measured {
                    examined,
                    total: total as u128,
                },
                Some(Stop::HitLimit),
            ));
        }
    }
    Ok(collector.finish(
        Coverage::Measured {
            examined: total,
            total: total as u128,
        },
        None,
    ))
}
