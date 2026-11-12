//! How far a KB branch is from its upstream, from local refs only.
//!
//! One `for-each-ref` spawn: the branch's configured upstream and Git's own
//! ahead/behind count against the remote-tracking ref as last fetched.
//! Nothing here fetches or contacts a remote.
use super::kb::{KbRepository, run};
use super::{GitBudgets, GitError, GitExecutable};
use std::sync::atomic::AtomicBool;

/// A branch's upstream and the commits each side has that the other lacks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KbUpstream {
    /// The upstream's short name, such as `origin/main`.
    pub name: String,
    pub ahead: u64,
    pub behind: u64,
    /// The remote-tracking ref no longer exists.
    pub gone: bool,
}

/// The upstream of `branch` (a full ref name such as `refs/heads/main`),
/// or `None` when the branch is absent or has no upstream configured.
pub fn kb_upstream(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    branch: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Option<KbUpstream>, GitError> {
    if !branch.starts_with("refs/heads/") {
        return Err(GitError::Malformed("upstream of a non-branch ref"));
    }
    let args = [
        "for-each-ref",
        "--format=%(refname)%09%(upstream:short)%09%(upstream:track,nobracket)",
        branch,
    ];
    let output = run(executable, repository, &args, &[], budgets, cancelled)?;
    let text = String::from_utf8(output).map_err(|_| GitError::Malformed("for-each-ref output"))?;
    Ok(text.lines().find_map(|line| parse(line, branch)))
}

/// One `<ref>\t<upstream>\t<track>` line, if it is `branch`'s and names
/// an upstream.
fn parse(line: &str, branch: &str) -> Option<KbUpstream> {
    let mut parts = line.splitn(3, '\t');
    let (name, upstream, track) = (parts.next()?, parts.next()?, parts.next().unwrap_or(""));
    if name != branch || upstream.is_empty() {
        return None;
    }
    let count = |side: &str| {
        track
            .split(", ")
            .find_map(|part| part.strip_prefix(side)?.trim().parse::<u64>().ok())
            .unwrap_or(0)
    };
    Some(KbUpstream {
        name: upstream.to_owned(),
        ahead: count("ahead "),
        behind: count("behind "),
        gone: track.trim() == "gone",
    })
}

#[cfg(test)]
mod tests {
    use super::{KbUpstream, parse};

    #[test]
    fn track_lines() {
        let at = |ahead, behind, gone| {
            Some(KbUpstream {
                name: "origin/main".into(),
                ahead,
                behind,
                gone,
            })
        };
        let main = "refs/heads/main";
        assert_eq!(
            parse("refs/heads/main\torigin/main\t", main),
            at(0, 0, false)
        );
        assert_eq!(
            parse("refs/heads/main\torigin/main\tbehind 4", main),
            at(0, 4, false)
        );
        assert_eq!(
            parse("refs/heads/main\torigin/main\tahead 1, behind 12", main),
            at(1, 12, false)
        );
        assert_eq!(
            parse("refs/heads/main\torigin/main\tgone", main),
            at(0, 0, true)
        );
        assert_eq!(parse("refs/heads/main\t\t", main), None);
        assert_eq!(parse("refs/heads/main/x\torigin/x\tbehind 1", main), None);
    }
}
