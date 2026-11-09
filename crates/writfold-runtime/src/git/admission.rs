use super::{
    GitError,
    profile::{check_config, check_packs},
};
use crate::fs::{ContainedRoot, FsError, OpenedFile, PinnedFile, ReadExtent, RelativePath};
use crate::process::ProcessSupervisor;
use rustix::fd::{AsRawFd, OwnedFd};
use rustix::fs::{AtFlags, Mode, OFlags, openat, statat};
use std::path::PathBuf;
use writfold_domain::{ByteCount, ContentDigest, GitObjectFormat};

/// A live pinned executable whose complete bytes match an external identity.
/// Admission borrows the caller's explicit process-wide subreaper opt-in.
#[derive(Debug)]
pub struct GitExecutable<'a> {
    opened: OpenedFile<'a>,
    supervisor: &'a ProcessSupervisor,
}

impl<'a> GitExecutable<'a> {
    pub fn admit(
        file: &'a PinnedFile,
        expected_digest: &ContentDigest,
        byte_limit: ByteCount,
        supervisor: &'a ProcessSupervisor,
    ) -> Result<Self, GitError> {
        supervisor.ensure_current()?;
        let opened = file.open_read()?;
        opened.check_unchanged()?;
        if opened.size() > byte_limit.get() {
            return Err(GitError::Budget {
                stream: "executable",
                limit: byte_limit.get(),
            });
        }
        if &opened.revision(byte_limit)? != expected_digest {
            return Err(GitError::ExecutableMismatch);
        }
        // Scripts cannot preserve the descriptor pin through their interpreter.
        let mut header = [0_u8; 4];
        if opened.read_prefix(&mut header)? != header.len() || header != *b"\x7fELF" {
            return Err(GitError::UnsupportedProfile(
                "ELF native executable required",
            ));
        }
        Ok(Self { opened, supervisor })
    }

    /// The content digest of an ELF executable, for a profile that pins the
    /// bytes it observes (`writfold-trial-worktree/1`). Observation is not
    /// qualification: it says these bytes were seen, not that they behave.
    pub fn observe(file: &PinnedFile, byte_limit: ByteCount) -> Result<ContentDigest, GitError> {
        let opened = file.open_read()?;
        opened.check_unchanged()?;
        if opened.size() > byte_limit.get() {
            return Err(GitError::Budget {
                stream: "executable",
                limit: byte_limit.get(),
            });
        }
        Ok(opened.revision(byte_limit)?)
    }

    pub(crate) fn path(&self) -> Result<PathBuf, GitError> {
        self.opened.check_unchanged()?;
        Ok(descriptor_path(self.opened.descriptor()))
    }

    pub(crate) fn supervisor(&self) -> &ProcessSupervisor {
        self.supervisor
    }
}

/// The actual standalone Git metadata directory (bare directory or `.git`).
/// The administrative owner must keep its metadata and object store stable.
#[derive(Debug)]
pub struct GitRepository {
    pub(super) root: ContainedRoot,
    format: GitObjectFormat,
    pub(super) metadata_limit: ByteCount,
}

impl GitRepository {
    pub fn admit(
        root: ContainedRoot,
        format: GitObjectFormat,
        metadata_limit: ByteCount,
    ) -> Result<Self, GitError> {
        let repository = Self {
            root,
            format,
            metadata_limit,
        };
        repository.check_profile()?;
        Ok(repository)
    }

    pub(super) fn format(&self) -> GitObjectFormat {
        self.format
    }

    pub(super) fn path(&self) -> PathBuf {
        descriptor_path(self.root.descriptor())
    }

    pub(super) fn check_profile(&self) -> Result<(), GitError> {
        let config = self
            .root
            .pin(&RelativePath::new("config").map_err(FsError::from)?)?
            .read(self.metadata_limit)?;
        if config.extent() != ReadExtent::Complete {
            return Err(GitError::Budget {
                stream: "metadata",
                limit: self.metadata_limit.get(),
            });
        }
        check_config(config.bytes(), self.format)?;
        for name in [
            "commondir",
            "gitdir",
            "shallow",
            "config.worktree",
            "info/grafts",
        ] {
            self.refuse_entry(name)?;
        }
        let objects = open_directory(self.root.descriptor(), "objects")?;
        let info = open_directory(&objects, "info")?;
        for name in ["alternates", "http-alternates"] {
            refuse_at(&info, name)?;
        }
        let pack = open_directory(&objects, "pack")?;
        check_packs(&pack, self.format, self.metadata_limit)?;
        Ok(())
    }

    fn refuse_entry(&self, name: &str) -> Result<(), GitError> {
        // Pinning also refuses a symlink in any ancestor of the metadata entry.
        let path = RelativePath::new(name).map_err(FsError::from)?;
        match self
            .root
            .pin(&path)
            .and_then(|file| file.open_read().map(|_| ()))
        {
            Err(FsError::Missing) => Ok(()),
            Err(error) => Err(error.into()),
            Ok(()) => Err(GitError::UnsupportedProfile(
                "linked/worktree/shallow metadata",
            )),
        }
    }
}

fn open_directory(parent: &OwnedFd, name: &str) -> Result<OwnedFd, GitError> {
    Ok(openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?)
}

fn refuse_at(parent: &OwnedFd, name: &str) -> Result<(), GitError> {
    match statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err(GitError::UnsupportedProfile("alternate object stores")),
    }
}

pub(super) fn descriptor_path(fd: &OwnedFd) -> PathBuf {
    // The descriptor is held by this process, not inherited by the child.
    PathBuf::from(format!(
        "/proc/{}/fd/{}",
        std::process::id(),
        fd.as_raw_fd()
    ))
}
