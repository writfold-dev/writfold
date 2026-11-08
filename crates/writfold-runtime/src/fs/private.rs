use super::{ContainedRoot, FileIdentity, FsError, OpenedFile, PinnedFile};
use rustix::fs::{FileType, Stat, fstat};

impl ContainedRoot {
    /// Require an effective-user-owned 0700 directory and its observed pathname
    /// binding. This check does not prevent later pathname or permission races;
    /// the owner must keep the namespace private and serialize cooperating effects.
    pub fn check_private(&self) -> Result<(), FsError> {
        self.check_current()?;
        let stat = fstat(&self.directory).map_err(FsError::from)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
            return Err(FsError::NotDirectory);
        }
        if stat.st_uid != rustix::process::geteuid().as_raw() || stat.st_mode & 0o7777 != 0o700 {
            return Err(FsError::Permission);
        }
        Ok(())
    }
}

impl PinnedFile {
    /// Observe a private, singly linked regular file, without creating anything.
    /// Compare the returned identity with a subsequent read under cooperation.
    pub fn check_private(&self) -> Result<FileIdentity, FsError> {
        let opened = self.open_read()?;
        opened.check_private()?;
        Ok(opened.identity())
    }
}

impl OpenedFile<'_> {
    /// Require effective-user ownership, no group/other/special/execute bits,
    /// owner read access, one hardlink, and an unchanged named regular file.
    pub fn check_private(&self) -> Result<(), FsError> {
        self.ensure_unchanged()?;
        private_file(&fstat(&self.descriptor).map_err(FsError::from)?)
    }
}

pub(super) fn private_file(stat: &Stat) -> Result<(), FsError> {
    match FileType::from_raw_mode(stat.st_mode) {
        FileType::RegularFile => {}
        FileType::Symlink => return Err(FsError::Symlink),
        _ => return Err(FsError::NonRegular),
    }
    if stat.st_nlink != 1 {
        return Err(FsError::HardLinked);
    }
    if stat.st_uid != rustix::process::geteuid().as_raw()
        || stat.st_mode & 0o7177 != 0
        || stat.st_mode & 0o400 == 0
    {
        return Err(FsError::Permission);
    }
    Ok(())
}
