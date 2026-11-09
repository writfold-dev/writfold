//! `writfold-trial-worktree/1`: a cooperating-user trial profile that adds a
//! detached Git worktree of an ordinary repository at its current HEAD.
//!
//! Unlike the hardened Worktrunk path this accepts packed objects, remotes and
//! ordinary configuration, because it runs plain `git worktree add` against the
//! user's own repository. It assumes the user does not race Writfold on the
//! worktree and that the repository is not hostile. It writes worktree metadata
//! into the user's repository and grants no write authority by itself.
use super::{GitBudgets, GitError, GitExecutable, command};
use crate::process::{self, ProcessLimits};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::{GitObjectFormat, GitObjectId};

/// Resolve the repository's HEAD commit, then add a detached worktree of that
/// exact commit at `destination`, which must not exist. Returns the commit.
pub fn trial_worktree_add(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    destination: &Path,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<GitObjectId, GitError> {
    if destination.exists() {
        return Err(GitError::UnsupportedProfile(
            "trial worktree already exists",
        ));
    }
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
    let commit = GitObjectId::new(format, text.to_owned())
        .map_err(|_| GitError::Malformed("HEAD commit"))?;
    let destination = destination.to_str().ok_or(GitError::UnsupportedProfile(
        "non-UTF-8 trial worktree path",
    ))?;
    git(
        executable,
        git_dir,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            destination,
            commit.as_str(),
        ],
        budgets,
        cancelled,
    )?;
    Ok(commit)
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
