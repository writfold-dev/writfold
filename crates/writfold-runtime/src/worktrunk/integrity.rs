use super::command::{Session, line};
use super::tree::{Entries, Kind, object_id};
use super::{Result, WorktrunkError};
use crate::git::GitError;
use std::collections::BTreeSet;
use std::path::Path;
use writfold_domain::{ByteCount, GitObjectId};

/// Verify only the selected commit and its complete checkout tree, not unrelated
/// history. A named Git object or successful cat-file call is not hash evidence.
/// Git supplies the existing admitted SHA-1/SHA-256 hash-object implementation.
pub(super) fn verify_base(
    session: &Session<'_, '_, '_>,
    repository: &Path,
    base: &GitObjectId,
) -> Result<()> {
    let mut metadata = session.limits.metadata.get();
    let commit = read_verified(session, repository, base.as_str(), "commit", metadata)?;
    charge_metadata(&mut metadata, commit.len())?;
    let tree_line = commit
        .split(|byte| *byte == b'\n')
        .next()
        .and_then(|line| line.strip_prefix(b"tree "))
        .ok_or(WorktrunkError::Observation("commit tree header"))?;
    let root = std::str::from_utf8(tree_line)
        .map_err(|_| WorktrunkError::Observation("commit tree ID"))?;
    object_id(base.format(), root)?;
    let root_bytes = read_verified(session, repository, root, "tree", metadata)?;
    charge_metadata(&mut metadata, root_bytes.len())?;
    drop(root_bytes);
    let listing = session.git(repository, &["ls-tree", "-r", "-t", "-l", "-z", root])?;
    let mut verified = BTreeSet::from([(Kind::Tree, root)]);
    for entry in Entries::new(&listing, base.format(), session.limits.checkout.get())? {
        let entry = entry?;
        session.check()?;
        if !verified.insert((entry.kind, entry.object)) {
            continue;
        }
        let limit = match entry.kind {
            Kind::Tree => metadata,
            Kind::Blob => entry.bytes,
        };
        let bytes = read_verified(
            session,
            repository,
            entry.object,
            entry.kind.as_str(),
            limit,
        )?;
        match entry.kind {
            Kind::Tree => charge_metadata(&mut metadata, bytes.len())?,
            Kind::Blob if bytes.len() as u64 != entry.bytes => {
                return Err(WorktrunkError::Observation("native blob size"));
            }
            Kind::Blob => {}
        }
    }
    Ok(())
}

fn charge_metadata(remaining: &mut u64, bytes: usize) -> Result<()> {
    *remaining = remaining
        .checked_sub(bytes as u64)
        .ok_or(WorktrunkError::Profile("object metadata byte budget"))?;
    Ok(())
}

fn read_verified(
    session: &Session<'_, '_, '_>,
    repository: &Path,
    object: &str,
    kind: &str,
    limit: u64,
) -> Result<Vec<u8>> {
    let stdout = ByteCount::new(limit.min(session.limits.stdout.get()))
        .map_err(|_| WorktrunkError::Profile("object byte budget"))?;
    let bytes = session.git_io(repository, &["cat-file", kind, object], &[], stdout)?;
    let actual = session.git_io(
        repository,
        &["hash-object", "-t", kind, "--stdin", "--no-filters"],
        &bytes,
        session.limits.stdout,
    )?;
    if line(&actual)? != object {
        return Err(GitError::ObjectMismatch.into());
    }
    Ok(bytes)
}
