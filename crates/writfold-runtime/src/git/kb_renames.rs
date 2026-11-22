//! Which paths were renamed between two commits, for following a cited
//! path that is absent at the target commit.
//!
//! One `diff-tree -r -M --diff-filter=R` spawn per pair of commits: Git's
//! own rename detection (content similarity, its default threshold) over
//! the whole tree, so a chain of renames between the two commits reads as
//! one. Nothing here fetches or writes.
use super::kb::{KbRepository, object_id, run};
use super::{GitBudgets, GitError, GitExecutable};
use crate::process;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

/// What following renames from a cited commit found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Renames {
    /// Old path to new path for every rename Git detected.
    Found(BTreeMap<String, String>),
    /// The cited commit is not a commit id or not in this repository (a
    /// shallow or single-branch clone, an unfetched or rewritten commit):
    /// nothing to follow from, which is no evidence of a removal (#90).
    UnknownCommit,
}

/// Old path to new path for every rename Git detects from `old` to `new`.
/// `old` is a commit id, full or abbreviated (at least seven hex digits);
/// `new` is a full commit id. [`Renames::UnknownCommit`] when `old` is not
/// hex or Git does not know it; any other Git failure is an error, never
/// read as an unknown commit.
pub fn kb_renames(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    old: &str,
    new: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Renames, GitError> {
    let hex = (7..=64).contains(&old.len()) && old.bytes().all(|b| b.is_ascii_hexdigit());
    if !hex || object_id(new).is_none() {
        return Ok(Renames::UnknownCommit);
    }
    let args = [
        "diff-tree",
        "-r",
        "-M",
        "--diff-filter=R",
        "--no-commit-id",
        "--name-status",
        "-z",
        old,
        new,
    ];
    match run(executable, repository, &args, &[], budgets, cancelled) {
        Ok(output) => Ok(Renames::Found(parse(&output))),
        Err(GitError::Process(process::ProcessError::Command { stderr, .. }))
            if unknown_commit(&stderr) =>
        {
            Ok(Renames::UnknownCommit)
        }
        Err(error) => Err(error),
    }
}

/// Whether Git's stderr says a named commit is not in the repository.
fn unknown_commit(stderr: &[u8]) -> bool {
    let said = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    [
        "bad object",
        "unknown revision",
        "not a valid object name",
        "invalid object name",
    ]
    .iter()
    .any(|phrase| said.contains(phrase))
}

/// `R<score>\0<old>\0<new>\0` records; anything else is skipped.
fn parse(output: &[u8]) -> BTreeMap<String, String> {
    let mut fields = output
        .split(|&b| b == 0)
        .filter(|f| !f.is_empty())
        .map(|f| String::from_utf8_lossy(f).into_owned());
    let mut out = BTreeMap::new();
    while let Some(status) = fields.next() {
        if !status.starts_with('R') {
            continue;
        }
        let (Some(from), Some(to)) = (fields.next(), fields.next()) else {
            break;
        };
        out.insert(from, to);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{parse, unknown_commit};

    #[test]
    fn only_an_unknown_commit_reads_as_one() {
        assert!(unknown_commit(b"fatal: bad object 1234567890abcdef\n"));
        assert!(unknown_commit(
            b"fatal: ambiguous argument 'abc1234': unknown revision or path not in the working tree.\n"
        ));
        assert!(!unknown_commit(b"fatal: unable to read tree 0123abc\n"));
        assert!(!unknown_commit(b"error: could not lock config file\n"));
        assert!(!unknown_commit(b""));
    }

    #[test]
    fn rename_records() {
        let out = parse(b"R087\0src/a.rs\0src/b/a.rs\0R100\0x\0y\0");
        assert_eq!(out.len(), 2);
        assert_eq!(out.get("src/a.rs").map(String::as_str), Some("src/b/a.rs"));
        assert_eq!(out.get("x").map(String::as_str), Some("y"));
        assert!(parse(b"").is_empty());
        assert!(parse(b"R050\0only\0").is_empty());
    }
}
