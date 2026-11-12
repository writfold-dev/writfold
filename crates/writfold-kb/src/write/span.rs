//! Reading the lines a code citation pins: the blob at `<commit>:<path>`
//! and the span digest of the cited lines (each trimmed, empty lines
//! dropped, joined by LF), for any number of spans with one batch check and
//! one blob read. `cite make` and the Tuft adapter both use it.
use crate::error::{KbError, Result};
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_batch_check, pinned_blobs};
use writfold_sources::citations::normalize_span;

/// Lines of one file to cite: a one-based inclusive range, or the lines a
/// half-open byte range touches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lines {
    Range(u32, u32),
    Bytes(u64, u64),
}

/// One span to cite: a repository path and its lines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    pub path: String,
    pub lines: Lines,
}

/// A span read at a commit: its line range, the file's blob and the span
/// digest of the cited lines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spanned {
    pub path: String,
    pub first: u32,
    pub last: u32,
    pub blob: String,
    pub span: String,
}

/// A blob and its bytes.
pub type Blob = (GitObjectId, Vec<u8>);

/// The blob at `<commit>:<path>` for each path, with its bytes (`None` when
/// the path holds no blob).
pub fn blobs(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    commit: &str,
    paths: &[&str],
) -> Result<Vec<Option<Blob>>> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    let names: Vec<String> = paths.iter().map(|p| format!("{commit}:{p}")).collect();
    let found = kb_batch_check(git, reads, &names, budgets()?, &CANCELLED)?;
    let ids: Vec<GitObjectId> = found
        .iter()
        .flatten()
        .filter(|(_, kind)| kind == "blob")
        .map(|(id, _)| id.clone())
        .collect();
    let mut read = pinned_blobs(git, git_dir, &ids, budgets()?, &CANCELLED)?.into_iter();
    let mut out = Vec::with_capacity(found.len());
    for answer in found {
        out.push(match answer {
            Some((id, kind)) if kind == "blob" => {
                let bytes = read
                    .next()
                    .ok_or_else(|| KbError::Usage("git returned fewer blobs than asked".into()))?;
                Some((id, bytes))
            }
            _ => None,
        });
    }
    Ok(out)
}

/// The one-based first and last line `lines` names in `bytes`. A byte
/// range covers the lines of its first and last byte (an empty range, the
/// line it sits on).
pub fn line_range(bytes: &[u8], lines: Lines) -> (usize, usize) {
    match lines {
        Lines::Range(a, b) => (a as usize, b as usize),
        Lines::Bytes(lo, hi) => {
            let at = |offset: u64| {
                let end = usize::try_from(offset)
                    .unwrap_or(usize::MAX)
                    .min(bytes.len());
                1 + bytes[..end].iter().filter(|b| **b == b'\n').count()
            };
            (at(lo), at(hi.max(lo.saturating_add(1)) - 1))
        }
    }
}

/// Read every span's file at `commit` and digest its cited lines, in the
/// order asked.
pub fn spans(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    commit: &str,
    wanted: &[Span],
) -> Result<Vec<Spanned>> {
    let mut paths: Vec<&str> = wanted.iter().map(|s| s.path.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    let read = blobs(git, git_dir, commit, &paths)?;
    let mut out = Vec::with_capacity(wanted.len());
    for want in wanted {
        let found = paths
            .binary_search(&want.path.as_str())
            .ok()
            .and_then(|i| read.get(i))
            .and_then(Option::as_ref);
        let Some((blob, bytes)) = found else {
            return Err(KbError::Refused(format!(
                "{} is not a file at {commit}",
                want.path
            )));
        };
        out.push(digest_lines(want, blob, bytes, commit)?);
    }
    Ok(out)
}

fn digest_lines(want: &Span, blob: &GitObjectId, bytes: &[u8], commit: &str) -> Result<Spanned> {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    let (a, b) = line_range(bytes, want.lines);
    let window = (a >= 1 && b >= a)
        .then(|| lines.get(a - 1..b))
        .flatten()
        .ok_or_else(|| {
            KbError::Usage(format!(
                "{} has {} lines at {commit}",
                want.path,
                lines.len()
            ))
        })?;
    Ok(Spanned {
        path: want.path.clone(),
        first: u32::try_from(a).unwrap_or(u32::MAX),
        last: u32::try_from(b).unwrap_or(u32::MAX),
        blob: blob.as_str().to_owned(),
        span: normalize_span(window),
    })
}

#[cfg(test)]
mod tests {
    use super::{Lines, line_range};

    #[test]
    fn byte_ranges_cover_their_lines() {
        let text = b"one\ntwo\nthree\n";
        assert_eq!(line_range(text, Lines::Bytes(0, 3)), (1, 1));
        // A range ending after a line's LF stays on that line.
        assert_eq!(line_range(text, Lines::Bytes(0, 8)), (1, 2));
        assert_eq!(line_range(text, Lines::Bytes(4, 14)), (2, 3));
        // An empty range sits on its line.
        assert_eq!(line_range(text, Lines::Bytes(5, 5)), (2, 2));
        assert_eq!(line_range(text, Lines::Range(2, 3)), (2, 3));
    }
}
