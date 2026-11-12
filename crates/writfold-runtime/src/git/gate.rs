//! Supervised Git commands for `writfold kb gate`: the blobs of a commit's
//! tree, the blob IDs of work-tree files, and a detached worktree at a
//! recorded commit for `gate verify`. Like the KB reads, these run against
//! the user's own repository in the closed environment; adding and removing
//! the verification worktree writes worktree metadata into it and nothing
//! else.
use super::kb::{object_id, run};
use super::{GitBudgets, GitError, GitExecutable, KbRepository};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use writfold_domain::GitObjectId;

/// One blob (or other tree entry) of a commit: its path and object ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateEntry {
    pub path: String,
    pub id: String,
}

/// Every entry of `commit`'s tree, recursively (`ls-tree -r -z`), as
/// top-relative paths. Non-UTF-8 paths are refused.
pub fn gate_ls_tree(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    commit: &GitObjectId,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<GateEntry>, GitError> {
    let repository = KbRepository {
        git_dir,
        work_tree: None,
    };
    let args = ["ls-tree", "-r", "-z", "--full-tree", commit.as_str()];
    let output = run(executable, repository, &args, &[], budgets, cancelled)?;
    let mut entries = Vec::new();
    for record in output.split(|&b| b == 0).filter(|r| !r.is_empty()) {
        let line = std::str::from_utf8(record).map_err(|_| GitError::Malformed("tree path"))?;
        let (meta, path) = line
            .split_once('\t')
            .ok_or(GitError::Malformed("ls-tree entry"))?;
        let id = meta
            .rsplit(' ')
            .next()
            .ok_or(GitError::Malformed("ls-tree entry"))?;
        entries.push(GateEntry {
            path: path.to_owned(),
            id: id.to_owned(),
        });
    }
    Ok(entries)
}

/// The blob ID Git would store for each work-tree file (`hash-object
/// --stdin-paths`, with the repository's filters, writing nothing). Paths
/// are relative to the top of `work_tree`.
pub fn gate_hash_paths(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    work_tree: &Path,
    paths: &[String],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut input = Vec::new();
    for path in paths {
        if path.contains('\n') {
            return Err(GitError::Malformed("path holds a line break"));
        }
        input.extend_from_slice(path.as_bytes());
        input.push(b'\n');
    }
    let repository = KbRepository {
        git_dir,
        work_tree: Some(work_tree),
    };
    let args = ["hash-object", "--stdin-paths"];
    let output = run(executable, repository, &args, &input, budgets, cancelled)?;
    let text = std::str::from_utf8(&output).map_err(|_| GitError::Malformed("hash-object"))?;
    let ids: Vec<String> = text.lines().map(str::to_owned).collect();
    if ids.len() != paths.len() || ids.iter().any(|id| object_id(id).is_none()) {
        return Err(GitError::Malformed("hash-object answer count"));
    }
    Ok(ids)
}

/// Add a detached worktree of `commit` at `destination`, which must not
/// exist, with every file checked out.
pub fn gate_worktree_add(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    destination: &Path,
    commit: &GitObjectId,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<(), GitError> {
    if destination.exists() {
        return Err(GitError::UnsupportedProfile("worktree already exists"));
    }
    let target = destination
        .to_str()
        .ok_or(GitError::UnsupportedProfile("non-UTF-8 worktree path"))?;
    let repository = KbRepository {
        git_dir,
        work_tree: None,
    };
    let args = [
        "worktree",
        "add",
        "--quiet",
        "--detach",
        target,
        commit.as_str(),
    ];
    run(executable, repository, &args, &[], budgets, cancelled)?;
    Ok(())
}

/// Remove a worktree [`gate_worktree_add`] created, with its metadata.
pub fn gate_worktree_remove(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    destination: &Path,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<(), GitError> {
    let target = destination
        .to_str()
        .ok_or(GitError::UnsupportedProfile("non-UTF-8 worktree path"))?;
    let repository = KbRepository {
        git_dir,
        work_tree: None,
    };
    let args = ["worktree", "remove", "--force", "--force", target];
    run(executable, repository, &args, &[], budgets, cancelled)?;
    Ok(())
}
