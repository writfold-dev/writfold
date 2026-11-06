use super::{FileIdentity, FsError, PathError, RelativePath};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, CWD, FileType, Mode, OFlags, fstat, fsync, mkdirat, openat, statat};
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// A directory admitted by opening every component without following symlinks.
#[derive(Debug)]
pub struct ContainedRoot {
    pub(super) directory: OwnedFd,
    bindings: Vec<(OwnedFd, OsString, FileIdentity)>,
}

/// An owned parent descriptor and a single validated leaf. No later operation
/// resolves the original root/ancestor path again.
#[derive(Debug)]
pub struct PinnedFile {
    pub(super) parent: OwnedFd,
    pub(super) name: OsString,
}

impl ContainedRoot {
    pub(crate) fn descriptor(&self) -> &OwnedFd {
        &self.directory
    }

    /// Open an existing root. This never creates directories or writer locks.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref();
        validate_root(path)?;
        let anchor = if path.is_absolute() { "/" } else { "." };
        let mut directory = open_directory_at(CWD, OsStr::new(anchor))?;
        let mut bindings = Vec::new();
        for part in path.as_os_str().as_bytes().split(|byte| *byte == b'/') {
            if !part.is_empty() {
                let name = OsStr::from_bytes(part);
                let child = open_directory_at(&directory, name)?;
                let identity = FileIdentity::of(&fstat(&child).map_err(FsError::from)?);
                bindings.push((directory, name.to_owned(), identity));
                directory = child;
            }
        }
        Ok(Self {
            directory,
            bindings,
        })
    }

    /// Explicitly create one new root (0700 before umask), not missing ancestors.
    /// An existing leaf is a collision. No failed call is a creation receipt.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, FsError> {
        let path = path.as_ref();
        validate_root(path)?;
        let name = path.file_name().ok_or(PathError::EmptyComponent)?;
        let parent_path = path.parent().filter(|path| !path.as_os_str().is_empty());
        let parent = Self::open(parent_path.unwrap_or_else(|| Path::new(".")))?;
        mkdirat(&parent.directory, name, Mode::RWXU).map_err(FsError::from)?;
        let directory = open_directory_at(&parent.directory, name)?;
        fsync(&directory).map_err(FsError::RootDurabilityUnknown)?;
        fsync(&parent.directory).map_err(FsError::RootDurabilityUnknown)?;
        let identity = FileIdentity::of(&fstat(&directory).map_err(FsError::from)?);
        let mut bindings = parent.bindings;
        bindings.push((parent.directory, name.to_owned(), identity));
        Ok(Self {
            directory,
            bindings,
        })
    }

    pub fn pin(&self, path: &RelativePath) -> Result<PinnedFile, FsError> {
        let mut directory = rustix::io::dup(&self.directory).map_err(FsError::from)?;
        let mut components = path.as_path().components().peekable();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                return Ok(PinnedFile {
                    parent: directory,
                    name: component.as_os_str().to_owned(),
                });
            }
            directory = open_directory_at(&directory, component.as_os_str())?;
        }
        Err(PathError::EmptyComponent.into())
    }

    /// Open an existing contained child directory without creating anything.
    /// The parent root remains responsible for checking its own ancestor chain.
    pub fn open_child(&self, path: &RelativePath) -> Result<Self, FsError> {
        self.check_current()?;
        let mut directory = rustix::io::dup(&self.directory).map_err(FsError::from)?;
        let mut bindings = Vec::new();
        for component in path.as_path().components() {
            let name = component.as_os_str();
            let child = open_directory_at(&directory, name)?;
            let identity = FileIdentity::of(&fstat(&child).map_err(FsError::from)?);
            bindings.push((directory, name.to_owned(), identity));
            directory = child;
        }
        Ok(Self {
            directory,
            bindings,
        })
    }

    /// Opt-in pathname identity check; ordinary descriptor-relative FS reads
    /// retain their historical ability to operate after a directory rename.
    pub fn check_current(&self) -> Result<(), FsError> {
        for (parent, name, identity) in &self.bindings {
            let named =
                statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| match error {
                    rustix::io::Errno::NOENT => FsError::ChangedSource,
                    _ => FsError::from(error),
                })?;
            if FileIdentity::of(&named) != *identity
                || FileType::from_raw_mode(named.st_mode) != FileType::Directory
            {
                return Err(FsError::ChangedSource);
            }
        }
        Ok(())
    }
}

fn validate_root(path: &Path) -> Result<(), PathError> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.is_empty() {
        return Err(PathError::EmptyComponent);
    }
    if bytes.contains(&0) {
        return Err(PathError::Nul);
    }
    if bytes.split(|byte| *byte == b'/').any(|part| part == b"..") {
        return Err(PathError::DotComponent);
    }
    Ok(())
}

pub(super) fn open_directory_at(
    parent: impl rustix::fd::AsFd,
    name: &OsStr,
) -> Result<OwnedFd, FsError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    openat(&parent, name, flags, Mode::empty()).map_err(|error| {
        // Linux reports ENOTDIR, rather than ELOOP, for O_DIRECTORY symlinks.
        // This extra observation refines the diagnostic; it never admits a path.
        if error == rustix::io::Errno::NOTDIR
            && let Ok(stat) = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW)
            && FileType::from_raw_mode(stat.st_mode) == FileType::Symlink
        {
            return FsError::Symlink;
        }
        FsError::from(error)
    })
}
