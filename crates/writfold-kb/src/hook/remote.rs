//! The session-start "behind remote" line: whether the KB clone's branch
//! lags its upstream as last fetched. Local refs only: one `for-each-ref`
//! spawn, never a fetch, so a machine that has not fetched sees what it
//! last knew, and the line says when that was.
use crate::time;
use crate::workspace::{CANCELLED, GitTool, Repo, budgets};
use std::path::{Path, PathBuf};
use writfold_runtime::git::{KbRepository, KbUpstream, kb_upstream};

/// `KB is N commits behind origin/main (last fetched …); run git pull`,
/// or `None` when it is not behind, has no upstream, HEAD is detached, or
/// anything fails (the hook stays silent about what it cannot tell).
pub fn behind(repo: &Repo, now: i64) -> Option<String> {
    let branch = head_branch(&repo.git_dir)?;
    let tool = GitTool::locate().ok()?;
    let git = tool.admit().ok()?;
    let reads = KbRepository {
        git_dir: &repo.git_dir,
        work_tree: None,
    };
    let upstream = kb_upstream(&git, reads, &branch, budgets().ok()?, &CANCELLED).ok()??;
    line(&upstream, fetched(&repo.git_dir), now)
}

/// The line for an upstream, given when the clone last fetched.
pub fn line(upstream: &KbUpstream, fetched: Option<i64>, now: i64) -> Option<String> {
    if upstream.behind == 0 || upstream.gone {
        return None;
    }
    let n = upstream.behind;
    let commits = if n == 1 { "commit" } else { "commits" };
    let when = fetched.map_or_else(
        || "never fetched".to_owned(),
        |at| format!("last fetched {}, {}", time::stamp(at), ago(now - at)),
    );
    Some(format!(
        "KB is {n} {commits} behind {} ({when}); run git pull\n",
        upstream.name
    ))
}

/// `3d ago`, `5h ago`, `12m ago`, `just now`.
fn ago(seconds: i64) -> String {
    match seconds {
        s if s >= 86_400 => format!("{}d ago", s / 86_400),
        s if s >= 3_600 => format!("{}h ago", s / 3_600),
        s if s >= 60 => format!("{}m ago", s / 60),
        _ => "just now".into(),
    }
}

/// The branch HEAD names (`refs/heads/…`), read from the file; `None`
/// when detached or unreadable.
fn head_branch(git_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let target = text.strip_prefix("ref: ")?.trim();
    target.starts_with("refs/heads/").then(|| target.to_owned())
}

/// The newest `FETCH_HEAD` modification time, in this worktree's Git
/// directory or the common one, in seconds since the epoch.
fn fetched(git_dir: &Path) -> Option<i64> {
    let mut dirs: Vec<PathBuf> = vec![git_dir.to_path_buf()];
    if let Ok(common) = std::fs::read_to_string(git_dir.join("commondir")) {
        dirs.push(git_dir.join(common.trim()));
    }
    dirs.iter()
        .filter_map(|d| {
            std::fs::metadata(d.join("FETCH_HEAD"))
                .ok()?
                .modified()
                .ok()
        })
        .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .filter_map(|d| i64::try_from(d.as_secs()).ok())
        .max()
}

#[cfg(test)]
mod tests {
    use super::{KbUpstream, line};

    fn upstream(behind: u64, gone: bool) -> KbUpstream {
        KbUpstream {
            name: "origin/main".into(),
            ahead: 2,
            behind,
            gone,
        }
    }

    #[test]
    fn behind_lines() {
        let now = 1_794_393_005;
        assert_eq!(line(&upstream(0, false), Some(now), now), None);
        assert_eq!(line(&upstream(3, true), Some(now), now), None);
        assert_eq!(
            line(&upstream(1, false), None, now).as_deref(),
            Some("KB is 1 commit behind origin/main (never fetched); run git pull\n")
        );
        assert_eq!(
            line(&upstream(4, false), Some(now - 3 * 86_400 - 5), now).as_deref(),
            Some(
                "KB is 4 commits behind origin/main (last fetched 2026-11-08T10:30:00Z, 3d ago); run git pull\n"
            )
        );
    }
}
