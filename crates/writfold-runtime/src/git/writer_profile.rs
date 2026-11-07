use super::{GitError, GitRepository};
use crate::fs::{ContainedRoot, FsError, ReadExtent, RelativePath};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, fstat, openat, statat};
use writfold_domain::GitObjectId;

pub(super) const LOCKS: &str = "writfold-locks";
pub(super) const COMPLETED: &str = "refs/kctx/knowledge";

/// Administrative privacy, not a sandbox against the same uid or root. All
/// cooperating writers use this repository's permanent lock namespace; external
/// Git writers/GC and owner metadata replacement are excluded during leases.
pub(super) fn check(repository: &GitRepository) -> Result<(), GitError> {
    repository.root.check_current()?;
    repository.check_profile()?;
    private_root(&repository.root)?;
    match statat(
        repository.root.descriptor(),
        "packed-refs",
        AtFlags::SYMLINK_NOFOLLOW,
    ) {
        Err(rustix::io::Errno::NOENT) => {}
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(GitError::UnsupportedProfile("writer requires loose refs")),
    }
    let mut remaining = repository.metadata_limit.get();
    walk(repository.root.descriptor(), 0, &mut remaining)?;
    Ok(())
}

pub(super) fn private_root(root: &ContainedRoot) -> Result<(), GitError> {
    root.check_current()?;
    let stat = fstat(root.descriptor())?;
    if stat.st_uid != rustix::process::geteuid().as_raw() || stat.st_mode & 0o077 != 0 {
        return Err(GitError::UnsupportedProfile(
            "private owner-only writer root required",
        ));
    }
    Ok(())
}

fn walk(parent: &OwnedFd, depth: usize, remaining: &mut u64) -> Result<(), GitError> {
    if depth > 64 {
        return Err(GitError::UnsupportedProfile("writer metadata depth"));
    }
    for entry in Dir::read_from(parent)? {
        let entry = entry?;
        let name = entry.file_name();
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        *remaining = remaining
            .checked_sub(name.to_bytes().len() as u64 + 1)
            .ok_or(GitError::UnsupportedProfile("writer metadata budget"))?;
        let file = openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )?;
        let stat = fstat(&file)?;
        if stat.st_uid != rustix::process::geteuid().as_raw() || stat.st_mode & 0o022 != 0 {
            return Err(GitError::UnsupportedProfile(
                "foreign/writable writer metadata",
            ));
        }
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => walk(&file, depth + 1, remaining)?,
            FileType::RegularFile if stat.st_nlink == 1 => {}
            _ => {
                return Err(GitError::UnsupportedProfile(
                    "linked/nonregular writer metadata",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn resolve(
    repository: &GitRepository,
    name: &str,
) -> Result<Option<GitObjectId>, GitError> {
    let path = RelativePath::new(name).map_err(FsError::from)?;
    let read = match repository
        .root
        .pin(&path)
        .and_then(|file| file.read(repository.metadata_limit))
    {
        Ok(read) => read,
        Err(FsError::Missing) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if read.extent() != ReadExtent::Complete {
        return Err(GitError::Malformed("reference budget"));
    }
    let bytes = read
        .bytes()
        .strip_suffix(b"\n")
        .ok_or(GitError::Malformed("reference terminator"))?;
    let text = std::str::from_utf8(bytes).map_err(|_| GitError::Malformed("reference encoding"))?;
    let object = GitObjectId::new(repository.format(), text.to_owned())
        .map_err(|_| GitError::Malformed("direct reference identity"))?;
    if object.as_str().bytes().all(|byte| byte == b'0') {
        return Err(GitError::Malformed("zero reference identity"));
    }
    Ok(Some(object))
}
