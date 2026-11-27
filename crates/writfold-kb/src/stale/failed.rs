//! A Git failure confined to one repository: its citations read
//! `unverifiable` with the reason (`git-error: …`), and the other
//! repositories are still checked.
use super::Checked;
use super::code::missing;
use crate::error::{KbError, Result};
use crate::workspace::CANCELLED;
use std::sync::atomic::Ordering;
use writfold_runtime::git::GitError;
use writfold_runtime::process::ProcessError;

/// A repository's verdicts, or, on a Git failure confined to it, each of
/// `members` unverifiable with the reason.
pub(super) fn per_repo_or(
    found: Result<Vec<(usize, Checked)>>,
    members: &[usize],
) -> Result<Vec<(usize, Checked)>> {
    match found {
        Ok(found) => Ok(found),
        Err(error) if !CANCELLED.load(Ordering::Relaxed) && per_repo(&error) => {
            let reason = format!("git-error: {}", brief(&error));
            Ok(members.iter().map(|&i| (i, missing(&reason))).collect())
        }
        Err(error) => Err(error),
    }
}

/// A Git failure confined to one repository: its citations read as
/// unverifiable with the reason, and the other repositories are still
/// checked.
fn per_repo(error: &KbError) -> bool {
    matches!(
        error,
        KbError::Git(_) | KbError::Process(_) | KbError::Fs(_)
    )
}

/// The first line of an error (of Git's stderr when it exited non-zero),
/// at most 160 characters.
fn brief(error: &KbError) -> String {
    let failed = match error {
        KbError::Process(ProcessError::Command { status, stderr })
        | KbError::Git(GitError::Process(ProcessError::Command { status, stderr })) => {
            let said = String::from_utf8_lossy(stderr);
            let line = said.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
            Some(format!(
                "exit {} {}",
                status.map_or_else(|| "?".to_owned(), |s| s.to_string()),
                line.trim()
            ))
        }
        _ => None,
    };
    let text = failed.unwrap_or_else(|| error.to_string());
    let line = text.lines().next().unwrap_or_default();
    line.trim().chars().take(160).collect()
}
