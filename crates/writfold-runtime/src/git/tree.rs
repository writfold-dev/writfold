//! Whole-tree reads at one pinned commit, for the read-only Git storage
//! backend (`writfold-store::backend::git`) that Tuft's `find` lowers onto.
//!
//! The commit must be named by its full object id and resolve to exactly that
//! id; a ref, abbreviation or `HEAD` is refused, so the current checkout is
//! never substituted. Listing and blob reads go through the same closed
//! command environment and process supervision as every other native Git
//! read. Gitlinks (submodules) are not blobs and are not listed. This grants
//! no authority and writes nothing.
use super::{GitBudgets, GitError, GitExecutable, command};
use crate::process::{self, ProcessLimits};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::GitObjectId;

/// One blob of a pinned tree (a regular, executable or symlink entry).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeBlob {
    pub path: String,
    pub blob: GitObjectId,
}

/// Parallel `cat-file --batch` readers for a whole-tree content load.
const READERS: usize = 8;
/// Below this many blobs one reader is used.
const BLOBS_PER_READER: usize = 2_048;

fn git(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    args: &[&str],
    input: &[u8],
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
        input,
        ProcessLimits::new(budgets.stdout, budgets.stderr),
        deadline,
        cancelled,
        executable.supervisor(),
    )?;
    Ok(output.stdout)
}

/// Every blob of the tree of `commit`, in strict binary path order.
pub fn pinned_tree(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    commit: &GitObjectId,
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<TreeBlob>, GitError> {
    let peeled = format!("{}^{{commit}}", commit.as_str());
    let resolved = git(
        executable,
        git_dir,
        &["rev-parse", "--verify", "--end-of-options", &peeled],
        &[],
        budgets,
        cancelled,
    )
    .map_err(|error| match error {
        GitError::Process(process::ProcessError::Command { .. }) => {
            GitError::Malformed("pinned commit")
        }
        other => other,
    })?;
    if resolved.strip_suffix(b"\n") != Some(commit.as_str().as_bytes()) {
        return Err(GitError::Malformed("pinned commit"));
    }
    let listing = git(
        executable,
        git_dir,
        &["ls-tree", "-r", "-z", "--full-tree", commit.as_str()],
        &[],
        budgets,
        cancelled,
    )?;
    let mut blobs = Vec::new();
    for entry in listing.split(|&b| b == 0).filter(|e| !e.is_empty()) {
        if let Some(blob) = tree_entry(entry, commit)? {
            blobs.push(blob);
        }
    }
    blobs.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    if blobs.windows(2).any(|w| w[0].path == w[1].path) {
        return Err(GitError::Malformed("duplicate path in pinned tree"));
    }
    Ok(blobs)
}

/// `<mode> SP <type> SP <id> TAB <path>`; gitlinks are skipped.
fn tree_entry(entry: &[u8], commit: &GitObjectId) -> Result<Option<TreeBlob>, GitError> {
    let tab = entry
        .iter()
        .position(|&b| b == b'\t')
        .ok_or(GitError::Malformed("ls-tree entry"))?;
    let (head, path) = (&entry[..tab], &entry[tab + 1..]);
    let head = std::str::from_utf8(head).map_err(|_| GitError::Malformed("ls-tree entry"))?;
    let mut fields = head.split(' ');
    let (Some(_mode), Some(kind), Some(id), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(GitError::Malformed("ls-tree entry"));
    };
    match kind {
        "commit" => return Ok(None),
        "blob" => {}
        _ => return Err(GitError::Malformed("ls-tree entry type")),
    }
    let path = std::str::from_utf8(path)
        .map_err(|_| GitError::UnsupportedProfile("non-UTF-8 path in pinned tree"))?;
    let blob = GitObjectId::new(commit.format(), id.to_owned())
        .map_err(|_| GitError::Malformed("ls-tree blob id"))?;
    Ok(Some(TreeBlob {
        path: path.to_owned(),
        blob,
    }))
}

/// The exact bytes of each blob in `ids`, in the same order. Large loads are
/// split over parallel supervised readers; each reader has the full budgets.
pub fn pinned_blobs(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    ids: &[GitObjectId],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<u8>>, GitError> {
    let readers = ids.len().div_ceil(BLOBS_PER_READER).clamp(1, READERS);
    let chunk = ids.len().div_ceil(readers).max(1);
    let results: Vec<Result<Vec<Vec<u8>>, GitError>> = std::thread::scope(|scope| {
        let handles: Vec<_> = ids
            .chunks(chunk)
            .map(|part| scope.spawn(move || batch(executable, git_dir, part, budgets, cancelled)))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or(Err(GitError::Malformed("blob reader thread failed")))
            })
            .collect()
    });
    let mut out = Vec::with_capacity(ids.len());
    for result in results {
        out.extend(result?);
    }
    Ok(out)
}

fn batch(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    ids: &[GitObjectId],
    budgets: GitBudgets,
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<u8>>, GitError> {
    let mut input = Vec::with_capacity(ids.len() * 65);
    for id in ids {
        input.extend_from_slice(id.as_str().as_bytes());
        input.push(b'\n');
    }
    let output = git(
        executable,
        git_dir,
        &["cat-file", "--batch"],
        &input,
        budgets,
        cancelled,
    )?;
    let mut rest = output.as_slice();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let newline = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(GitError::Malformed("cat-file header"))?;
        let header = std::str::from_utf8(&rest[..newline])
            .map_err(|_| GitError::Malformed("cat-file header"))?;
        let size = header
            .strip_prefix(id.as_str())
            .and_then(|h| h.strip_prefix(" blob "))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or(GitError::ObjectType)?;
        let body = rest
            .get(newline + 1..newline + 1 + size)
            .ok_or(GitError::Malformed("cat-file body"))?;
        if rest.get(newline + 1 + size) != Some(&b'\n') {
            return Err(GitError::Malformed("cat-file body"));
        }
        out.push(body.to_vec());
        rest = &rest[newline + 2 + size..];
    }
    if !rest.is_empty() {
        return Err(GitError::Malformed("cat-file trailing output"));
    }
    Ok(out)
}
