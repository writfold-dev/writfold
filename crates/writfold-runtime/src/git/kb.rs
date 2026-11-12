//! Supervised reads for the KB v1 continuity verbs (`writfold-kb`).
//!
//! These run against the user's own repository, like the trial profile:
//! ordinary configuration, packs and linked worktrees are accepted, and the
//! user is assumed not to race Writfold or to be hostile. Every command goes
//! through the same closed environment and process supervision as the other
//! native reads. Work-tree commands (`status`, `ls-files`, `log`) run with the
//! work tree as their directory, so pathspecs and paths are relative to the
//! top of the work tree. Nothing here writes, fetches or grants authority.
use super::{GitBudgets, GitError, GitExecutable, command};
use crate::process::{self, ProcessLimits};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::{GitObjectFormat, GitObjectId};

/// Where one read runs: the Git directory, and the work tree when the
/// command needs one.
#[derive(Clone, Copy, Debug)]
pub struct KbRepository<'a> {
    pub git_dir: &'a Path,
    pub work_tree: Option<&'a Path>,
}

/// One `status --porcelain=v2` entry under the pathspec: a path that differs
/// from HEAD in the index or work tree, or is untracked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KbDirty {
    pub path: String,
    /// The path a rename or copy came from, if any.
    pub origin: Option<String>,
}

/// HEAD and the dirty paths, from one spawn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KbStatus {
    /// `None` before the first commit.
    pub head: Option<GitObjectId>,
    pub branch: Option<String>,
    pub dirty: Vec<KbDirty>,
}

/// One commit from `log`: id, committer date (`YYYY-MM-DD`) and subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KbCommit {
    pub id: String,
    pub date: String,
    pub subject: String,
}

fn run(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    args: &[&str],
    input: &[u8],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, GitError> {
    let deadline = Instant::now()
        .checked_add(budgets.timeout)
        .ok_or(GitError::Malformed("deadline overflow"))?;
    let mut native = command::command(executable, repository.git_dir)?;
    if let Some(work_tree) = repository.work_tree {
        native
            .current_dir(work_tree)
            .arg("--work-tree")
            .arg(work_tree);
    }
    native.args(args);
    let output = process::run(
        &mut native,
        input,
        ProcessLimits::new(budgets.stdout, budgets.stderr),
        deadline,
        cancelled,
        executable.supervisor(),
    )?;
    Ok(output.stdout)
}

fn object_id(text: &str) -> Option<GitObjectId> {
    let format = match text.len() {
        40 => GitObjectFormat::Sha1,
        64 => GitObjectFormat::Sha256,
        _ => return None,
    };
    GitObjectId::new(format, text.to_owned()).ok()
}

fn utf8(bytes: &[u8], what: &'static str) -> Result<String, GitError> {
    String::from_utf8(bytes.to_vec()).map_err(|_| GitError::Malformed(what))
}

/// `status --porcelain=v2 --branch -z --untracked-files=all -- <pathspec>`.
pub fn kb_status(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    pathspec: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<KbStatus, GitError> {
    let output = run(
        executable,
        repository,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
            "--no-renames",
            "--",
            pathspec,
        ],
        &[],
        budgets,
        cancelled,
    )?;
    parse_status(&output)
}

fn parse_status(output: &[u8]) -> Result<KbStatus, GitError> {
    let mut status = KbStatus {
        head: None,
        branch: None,
        dirty: Vec::new(),
    };
    let mut records = output.split(|&b| b == 0).filter(|r| !r.is_empty());
    while let Some(record) = records.next() {
        let line = utf8(record, "status entry")?;
        if let Some(oid) = line.strip_prefix("# branch.oid ") {
            status.head = object_id(oid);
        } else if let Some(name) = line.strip_prefix("# branch.head ") {
            status.branch = (name != "(detached)").then(|| name.to_owned());
        } else if line.starts_with('#') || line.starts_with('!') {
            continue;
        } else if let Some(path) = line.strip_prefix("? ") {
            status.dirty.push(KbDirty {
                path: path.to_owned(),
                origin: None,
            });
        } else {
            let (fields, origin) = match line.as_bytes().first() {
                Some(b'1') => (8, None),
                Some(b'2') => (9, records.next().map(|o| utf8(o, "status origin"))),
                Some(b'u') => (10, None),
                _ => return Err(GitError::Malformed("status entry")),
            };
            let path = line
                .splitn(fields + 1, ' ')
                .nth(fields)
                .ok_or(GitError::Malformed("status entry"))?;
            status.dirty.push(KbDirty {
                path: path.to_owned(),
                origin: origin.transpose()?,
            });
        }
    }
    Ok(status)
}

/// Tracked and untracked, not ignored, files under the pathspec, as
/// work-tree-relative paths in Git's order. A non-UTF-8 path is returned
/// lossily (with U+FFFD) so the caller can count it rather than lose it.
pub fn kb_ls_files(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    pathspec: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, GitError> {
    let output = run(
        executable,
        repository,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--deduplicate",
            "--",
            pathspec,
        ],
        &[],
        budgets,
        cancelled,
    )?;
    Ok(output
        .split(|&b| b == 0)
        .filter(|r| !r.is_empty())
        .map(|r| String::from_utf8_lossy(r).into_owned())
        .collect())
}

/// Paths under the pathspec that differ between commits `old` and `new`
/// (`diff-tree -r --name-only -z --no-renames`), relative to the top of the
/// work tree. `None` when either commit is unknown to the repository.
pub fn kb_diff_tree(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    old: &str,
    new: &str,
    pathspec: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Option<Vec<String>>, GitError> {
    if [old, new].iter().any(|id| object_id(id).is_none()) {
        return Ok(None);
    }
    let args = [
        "diff-tree",
        "-r",
        "--no-commit-id",
        "--name-only",
        "--no-renames",
        "-z",
        old,
        new,
        "--",
        pathspec,
    ];
    match run(executable, repository, &args, &[], budgets, cancelled) {
        Ok(output) => Ok(Some(
            output
                .split(|&b| b == 0)
                .filter(|r| !r.is_empty())
                .map(|r| String::from_utf8_lossy(r).into_owned())
                .collect(),
        )),
        Err(GitError::Process(process::ProcessError::Command { .. })) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The full commit `spec` names, or `None` when it names none.
pub fn kb_commit(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    spec: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Option<GitObjectId>, GitError> {
    let peeled = format!("{spec}^{{commit}}");
    let args = [
        "rev-parse",
        "--verify",
        "--quiet",
        "--end-of-options",
        &peeled,
    ];
    match run(executable, repository, &args, &[], budgets, cancelled) {
        Ok(output) => {
            let text = utf8(&output, "rev-parse output")?;
            Ok(object_id(text.trim_end()))
        }
        Err(GitError::Process(process::ProcessError::Command { .. })) => Ok(None),
        Err(error) => Err(error),
    }
}

/// `cat-file --batch-check` over object names such as `<commit>:<path>`:
/// each answer is the object id and type, or `None` when it is missing.
pub fn kb_batch_check(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    names: &[String],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<Option<(GitObjectId, String)>>, GitError> {
    let mut input = Vec::new();
    for name in names {
        if name.contains('\n') {
            return Err(GitError::Malformed("object name holds a line break"));
        }
        input.extend_from_slice(name.as_bytes());
        input.push(b'\n');
    }
    let args = [
        "cat-file",
        "--batch-check=%(objectname) %(objecttype)",
        "--buffer",
    ];
    let output = run(executable, repository, &args, &input, budgets, cancelled)?;
    let text = utf8(&output, "batch-check output")?;
    let answers: Vec<_> = text
        .lines()
        .map(|line| {
            let mut parts = line.split(' ');
            match (parts.next().and_then(object_id), parts.next()) {
                (Some(id), Some(kind)) if !line.ends_with(" missing") => {
                    Some((id, kind.to_owned()))
                }
                _ => None,
            }
        })
        .collect();
    if answers.len() != names.len() {
        return Err(GitError::Malformed("batch-check answer count"));
    }
    Ok(answers)
}

/// Up to `limit` commits reachable from HEAD touching the pathspec, newest
/// first, optionally only those committed on or after `since` (a date).
pub fn kb_log(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    pathspec: &str,
    limit: usize,
    since: Option<&str>,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<KbCommit>, GitError> {
    let count = format!("--max-count={limit}");
    let after = since.map(|date| format!("--since={date}"));
    let mut args = vec![
        "log",
        "--no-color",
        "--format=%H%x1f%cs%x1f%s",
        count.as_str(),
    ];
    args.extend(after.as_deref());
    args.extend(["HEAD", "--", pathspec]);
    let output = run(executable, repository, &args, &[], budgets, cancelled)?;
    let text = utf8(&output, "log output")?;
    text.lines()
        .map(|line| {
            let mut parts = line.splitn(3, '\u{1f}');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(id), Some(date), Some(subject)) => Ok(KbCommit {
                    id: id.to_owned(),
                    date: date.to_owned(),
                    subject: subject.to_owned(),
                }),
                _ => Err(GitError::Malformed("log entry")),
            }
        })
        .collect()
}

/// One repository configuration value (`config --get`), if set.
pub fn kb_config(
    executable: &GitExecutable<'_>,
    repository: KbRepository<'_>,
    key: &str,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Option<String>, GitError> {
    let args = ["config", "--get", "--end-of-options", key];
    match run(executable, repository, &args, &[], budgets, cancelled) {
        Ok(output) => Ok(Some(utf8(&output, "config value")?.trim_end().to_owned())),
        Err(GitError::Process(process::ProcessError::Command { .. })) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_status;

    #[test]
    fn status_entries_and_branch() -> Result<(), super::GitError> {
        let raw = b"# branch.oid 0123456789012345678901234567890123456789\0# branch.head main\0\
1 .M N... 100644 100644 100644 aa bb notes/a b.md\0? new.md\0\
2 R. N... 100644 100644 100644 aa bb R100 to.md\0from.md\0";
        let status = parse_status(raw)?;
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert!(status.head.is_some());
        let paths: Vec<_> = status.dirty.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(paths, ["notes/a b.md", "new.md", "to.md"]);
        assert_eq!(status.dirty[2].origin.as_deref(), Some("from.md"));
        Ok(())
    }
}
