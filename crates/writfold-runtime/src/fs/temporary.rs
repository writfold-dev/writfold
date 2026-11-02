use super::{BudgetKind, FileIdentity, FsError, PinnedFile};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, Mode, OFlags, fstat, fsync, linkat, openat, renameat, statat, unlinkat};
use rustix::io::{Errno, write};
use std::ffi::{OsStr, OsString};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
const NAME_ATTEMPTS: u64 = 128;

pub(super) struct Temporary<'a> {
    pub descriptor: OwnedFd,
    parent: &'a OwnedFd,
    name: OsString,
    owned_entry: bool,
}

impl<'a> Temporary<'a> {
    pub fn create(target: &'a PinnedFile) -> Result<Self, FsError> {
        for _ in 0..NAME_ATTEMPTS {
            let sequence = NEXT_TEMPORARY
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| FsError::Budget {
                    kind: BudgetKind::TemporaryNames,
                    limit: u64::MAX,
                })?;
            let name = OsString::from(format!(".writfold-tmp-{}-{sequence}", std::process::id()));
            if name == target.name {
                continue;
            }
            let flags = OFlags::WRONLY
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::CLOEXEC
                | OFlags::NONBLOCK;
            match openat(&target.parent, &name, flags, Mode::RUSR | Mode::WUSR) {
                Ok(descriptor) => {
                    return Ok(Self {
                        descriptor,
                        parent: &target.parent,
                        name,
                        owned_entry: true,
                    });
                }
                Err(Errno::EXIST) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(FsError::Budget {
            kind: BudgetKind::TemporaryNames,
            limit: NAME_ATTEMPTS,
        })
    }

    pub fn fill_and_sync(&self, content: &[u8]) -> Result<(), FsError> {
        let mut remaining = content;
        while !remaining.is_empty() {
            match write(&self.descriptor, remaining) {
                Ok(0) => return Err(FsError::Io(Errno::IO)),
                Ok(count) => remaining = &remaining[count..],
                Err(Errno::INTR) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        fsync(&self.descriptor).map_err(FsError::from)
    }

    pub fn identity(&self) -> Result<FileIdentity, FsError> {
        Ok(FileIdentity::of(
            &fstat(&self.descriptor).map_err(FsError::from)?,
        ))
    }

    pub fn publish_new(&self, destination: &OsStr) -> Result<(), FsError> {
        self.check_owned()?;
        linkat(
            self.parent,
            &self.name,
            self.parent,
            destination,
            AtFlags::empty(),
        )
        .map_err(FsError::from)
    }

    pub fn publish_replace(&mut self, destination: &OsStr) -> Result<(), FsError> {
        self.check_owned()?;
        renameat(self.parent, &self.name, self.parent, destination).map_err(FsError::from)?;
        self.owned_entry = false;
        Ok(())
    }

    fn check_owned(&self) -> Result<(), FsError> {
        let opened = self.identity()?;
        let named =
            statat(self.parent, &self.name, AtFlags::SYMLINK_NOFOLLOW).map_err(FsError::from)?;
        if FileIdentity::of(&named) != opened {
            return Err(FsError::ChangedSource);
        }
        Ok(())
    }

    /// Never scan names or remove a pre-existing collision. A mismatched entry
    /// is left alone; hostile replacement between check/unlink is outside the
    /// cooperating namespace assumption, not something fstat can prove absent.
    pub fn remove_owned(&mut self) -> Result<(), FsError> {
        if !self.owned_entry {
            return Ok(());
        }
        self.owned_entry = false;
        match self.check_owned() {
            Err(FsError::Missing) => return Ok(()),
            other => other?,
        }
        unlinkat(self.parent, &self.name, AtFlags::empty()).map_err(FsError::from)
    }
}

impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        // Failure leaves at most this operation's temporary; it is not swept by
        // a later write. Publication paths report explicit cleanup failures.
        let _ = self.remove_owned();
    }
}
