//! `writfold-trial-worktree/1`: a cooperating-user trial profile that adds a
//! detached, sparse Git worktree of an ordinary repository at its current HEAD.
//!
//! Unlike the hardened Worktrunk path this accepts packed objects, remotes and
//! ordinary configuration, because it runs plain `git worktree add` against the
//! user's own repository. It assumes the user does not race Writfold on the
//! worktree and that the repository is not hostile. It writes worktree metadata
//! into the user's repository and grants no write authority by itself.
use super::{GitBudgets, GitError, GitExecutable, command};
use crate::process::{self, ProcessLimits};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::{GitObjectFormat, GitObjectId};

/// Resolve the repository's HEAD commit, then add a detached worktree of that
/// exact commit at `destination`, which must not exist, populating only the
/// `paths` the run edits. Returns the commit.
///
/// The worktree is added without a checkout and filled by one sparse
/// `read-tree -mu` whose patterns name exactly `paths`: every other entry is
/// marked skip-worktree, so `git status`, `git diff` and commits in the
/// worktree behave normally while the cost no longer grows with the size of
/// the repository. `core.sparseCheckout` is set for that single command only;
/// the user's repository configuration is not changed.
pub fn trial_worktree_add(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    destination: &Path,
    paths: &[&str],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<GitObjectId, GitError> {
    if destination.exists() {
        return Err(GitError::UnsupportedProfile(
            "trial worktree already exists",
        ));
    }
    let commit = head_commit(executable, git_dir, budgets, cancelled)?;
    let target = destination.to_str().ok_or(GitError::UnsupportedProfile(
        "non-UTF-8 trial worktree path",
    ))?;
    git(
        executable,
        git_dir,
        &[
            "worktree",
            "add",
            "--quiet",
            "--no-checkout",
            "--detach",
            target,
            commit.as_str(),
        ],
        budgets,
        cancelled,
    )?;
    let worktree_git_dir = linked_git_dir(git_dir, destination)?;
    let info = worktree_git_dir.join("info");
    std::fs::create_dir_all(&info).map_err(|_| GitError::Malformed("worktree info directory"))?;
    let mut patterns = String::new();
    for path in paths {
        patterns.push('/');
        patterns.push_str(&sparse_pattern(path));
        patterns.push('\n');
    }
    std::fs::write(info.join("sparse-checkout"), patterns)
        .map_err(|_| GitError::Malformed("sparse checkout patterns"))?;
    git(
        executable,
        &worktree_git_dir,
        &[
            "--work-tree",
            target,
            "-c",
            "core.sparseCheckout=true",
            "read-tree",
            "-mu",
            commit.as_str(),
        ],
        budgets,
        cancelled,
    )?;
    Ok(commit)
}

fn head_commit(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<GitObjectId, GitError> {
    let head = git(
        executable,
        git_dir,
        &["rev-parse", "--verify", "HEAD^{commit}"],
        budgets,
        cancelled,
    )?;
    let text = std::str::from_utf8(&head)
        .ok()
        .and_then(|t| t.strip_suffix('\n'))
        .ok_or(GitError::Malformed("HEAD commit"))?;
    let format = match text.len() {
        40 => GitObjectFormat::Sha1,
        64 => GitObjectFormat::Sha256,
        _ => return Err(GitError::Malformed("HEAD commit")),
    };
    GitObjectId::new(format, text.to_owned()).map_err(|_| GitError::Malformed("HEAD commit"))
}

/// The linked worktree's private Git directory, read from the `.git` file
/// `git worktree add` wrote; it must lie under the repository's `worktrees/`.
fn linked_git_dir(git_dir: &Path, destination: &Path) -> Result<PathBuf, GitError> {
    let pointer = std::fs::read_to_string(destination.join(".git"))
        .map_err(|_| GitError::Malformed("worktree .git file"))?;
    let linked = pointer
        .strip_prefix("gitdir: ")
        .and_then(|p| p.strip_suffix('\n'))
        .map(PathBuf::from)
        .ok_or(GitError::Malformed("worktree .git file"))?;
    if linked.parent() != Some(git_dir.join("worktrees").as_path()) {
        return Err(GitError::Malformed("worktree .git file"));
    }
    Ok(linked)
}

/// A sparse-checkout pattern matching exactly `path`: gitignore-style
/// metacharacters and spaces are escaped.
fn sparse_pattern(path: &str) -> String {
    let mut pattern = String::with_capacity(path.len());
    for c in path.chars() {
        if matches!(c, '\\' | '*' | '?' | '[' | ']' | '!' | '#' | ' ') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern
}

/// The exact bytes of `path` at the full commit id `commit`, with the blob's
/// native identity. `commit` must name an existing commit by its full object
/// id; a ref, abbreviation or `HEAD` is refused, so the current checkout is
/// never substituted for the pinned revision.
pub fn trial_blob(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    commit: &GitObjectId,
    path: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<(GitObjectId, Vec<u8>), GitError> {
    let resolved = git(
        executable,
        git_dir,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{}^{{commit}}", commit.as_str()),
        ],
        budgets,
        cancelled,
    )?;
    if resolved.strip_suffix(b"\n") != Some(commit.as_str().as_bytes()) {
        return Err(GitError::Malformed("pinned commit"));
    }
    let spec = format!("{}:{path}", commit.as_str());
    let kind = git(
        executable,
        git_dir,
        &["cat-file", "-t", "--end-of-options", &spec],
        budgets,
        cancelled,
    )?;
    if kind != b"blob\n" {
        return Err(GitError::UnsupportedProfile("pinned path is not a file"));
    }
    let id = git(
        executable,
        git_dir,
        &["rev-parse", "--verify", "--end-of-options", &spec],
        budgets,
        cancelled,
    )?;
    let text = std::str::from_utf8(&id)
        .ok()
        .and_then(|t| t.strip_suffix('\n'))
        .ok_or(GitError::Malformed("blob id"))?;
    let blob = GitObjectId::new(commit.format(), text.to_owned())
        .map_err(|_| GitError::Malformed("blob id"))?;
    let bytes = git(
        executable,
        git_dir,
        &["cat-file", "blob", blob.as_str()],
        budgets,
        cancelled,
    )?;
    Ok((blob, bytes))
}

/// Remove a trial worktree this profile created, including its metadata in
/// the user's repository. Used only to undo a refused preparation.
pub fn trial_worktree_remove(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    destination: &Path,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<(), GitError> {
    let destination = destination.to_str().ok_or(GitError::UnsupportedProfile(
        "non-UTF-8 trial worktree path",
    ))?;
    git(
        executable,
        git_dir,
        &["worktree", "remove", "--force", destination],
        budgets,
        cancelled,
    )?;
    Ok(())
}

fn git(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    args: &[&str],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, GitError> {
    let deadline = Instant::now()
        .checked_add(budgets.timeout)
        .ok_or(GitError::Malformed("deadline overflow"))?;
    let mut native = command::command(executable, git_dir)?;
    native.args(args);
    let output = process::run(
        &mut native,
        &[],
        ProcessLimits::new(budgets.stdout, budgets.stderr),
        deadline,
        cancelled,
        executable.supervisor(),
    )?;
    Ok(output.stdout)
}
