//! Retaining a run's hits: one pinned blob read per file, each range bound
//! as a `writfold-citation/1` binding in an [`EvidenceStore`] (the trial
//! state given by `--state`, or Tuft's KB-owned state, [`super::state`]).
use super::search;
use crate::trial::Result;
use crate::trial::retain::{EvidenceStore, PinnedBlob, read_pinned, retain_read};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tuft::locate::{around, line_at};
use tuft::view::{Evidence, Row};
use writfold_runtime::git::GitExecutable;

/// Retain every hit (or its whole lines `around`), one pinned blob read per
/// file, and check the retained bytes against the bytes the search read.
pub(super) fn retain_rows(
    store: &dyn EvidenceStore,
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
    let paths: Vec<&str> = by_path
        .keys()
        .copied()
        .filter(|path| !found.dirty.contains(*path))
        .collect();
    let reads = read_all(executable, repo, commit, &paths)?;
    let mut rows = dirty_rows(found, &by_path)?;
    let mut bindings = BTreeSet::new();
    let pinned = by_path
        .into_iter()
        .filter(|(path, _)| !found.dirty.contains(*path));
    for ((path, ranges), read) in pinned.zip(reads) {
        let content = found.files.get(path).ok_or("hit without file bytes")?;
        if &read.content != content {
            return Err(
                format!("{path} read for retention differs from the searched bytes").into(),
            );
        }
        for retained in retain_read(store, &read, &ranges)? {
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

/// Dirty evidence has an explicit non-binding marker and is never written as
/// a pinned citation descriptor. The caller carries the searched bytes to KB.
fn dirty_rows(found: &search::Found, paths: &BTreeMap<&str, Vec<(u64, u64)>>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for (path, ranges) in paths
        .iter()
        .filter(|(path, _)| found.dirty.contains(**path))
    {
        let content = found.files.get(*path).ok_or("dirty hit without bytes")?;
        for &(lo, hi) in ranges {
            let bytes = content
                .get(lo as usize..hi as usize)
                .ok_or("dirty range outside bytes")?;
            rows.push(Row {
                path: (*path).to_owned(),
                range: (lo, hi),
                line: line_at(content, lo as usize),
                evidence: Some(Evidence {
                    binding: format!("dirty:sha256:{}", tuft::digest_hex(bytes)),
                    bytes: bytes.to_vec(),
                }),
            });
        }
    }
    Ok(rows)
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
