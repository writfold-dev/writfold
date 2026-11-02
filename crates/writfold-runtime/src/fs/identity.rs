use super::FsError;
use rustix::fs::{FileType, Stat};
use sha2::{Digest, Sha256};
use writfold_domain::ContentDigest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    device: u64,
    inode: u64,
}

impl FileIdentity {
    pub fn device(self) -> u64 {
        self.device
    }

    pub fn inode(self) -> u64 {
        self.inode
    }

    pub(super) fn of(stat: &Stat) -> Self {
        Self {
            device: stat.st_dev,
            inode: stat.st_ino,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FileStamp {
    pub identity: FileIdentity,
    pub size: u64,
    pub links: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    modified: (i64, u64),
    changed: (i64, u64),
}

impl FileStamp {
    pub fn regular(stat: &Stat) -> Result<Self, FsError> {
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::RegularFile => {}
            FileType::Symlink => return Err(FsError::Symlink),
            _ => return Err(FsError::NonRegular),
        }
        Ok(Self {
            identity: FileIdentity::of(stat),
            size: u64::try_from(stat.st_size).map_err(|_| FsError::ChangedSource)?,
            links: stat.st_nlink,
            mode: stat.st_mode,
            uid: stat.st_uid,
            gid: stat.st_gid,
            modified: (stat.st_mtime, stat.st_mtime_nsec),
            changed: (stat.st_ctime, stat.st_ctime_nsec),
        })
    }
}

pub(super) fn digest(content: &[u8]) -> Result<ContentDigest, FsError> {
    finish_digest(Sha256::new_with_prefix(content))
}

pub(super) fn finish_digest(hasher: Sha256) -> Result<ContentDigest, FsError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in hasher.finalize() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 15)]));
    }
    ContentDigest::try_from(encoded).map_err(FsError::Digest)
}
