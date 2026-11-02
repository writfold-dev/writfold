use super::identity::{FileStamp, finish_digest};
use super::{BudgetKind, FileIdentity, FsError, PinnedFile};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FileType, Mode, OFlags, fstat, openat, statat};
use rustix::io::{Errno, pread};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use writfold_domain::{ByteCount, ContentDigest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadExtent {
    Complete,
    /// The digest covers only the returned prefix, not the unread suffix.
    Truncated {
        limit: ByteCount,
        observed_size: u64,
    },
}

#[derive(Debug)]
pub struct ReadResult {
    bytes: Vec<u8>,
    digest: ContentDigest,
    identity: FileIdentity,
    extent: ReadExtent,
}

impl ReadResult {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }

    pub fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub fn extent(&self) -> ReadExtent {
        self.extent
    }
}

/// Holds both the leaf and (by borrowing its target) its parent descriptor.
#[derive(Debug)]
pub struct OpenedFile<'a> {
    pub(super) target: &'a PinnedFile,
    pub(super) descriptor: OwnedFd,
    pub(super) stamp: FileStamp,
}

impl PinnedFile {
    pub fn open_read(&self) -> Result<OpenedFile<'_>, FsError> {
        let (descriptor, stamp) = open_regular(&self.parent, &self.name, OFlags::RDONLY)?;
        Ok(OpenedFile {
            target: self,
            descriptor,
            stamp,
        })
    }

    pub fn read(&self, limit: ByteCount) -> Result<ReadResult, FsError> {
        self.open_read()?.read(limit)
    }
}

impl OpenedFile<'_> {
    /// At most `limit` retained bytes plus a one-byte EOF probe are read. A
    /// truncated prefix is explicit, including the zero-budget/nonempty case.
    pub fn read(&self, limit: ByteCount) -> Result<ReadResult, FsError> {
        self.ensure_unchanged()?;
        let capacity =
            usize::try_from(self.stamp.size.min(limit.get())).map_err(|_| FsError::Budget {
                kind: BudgetKind::Allocation,
                limit: limit.get(),
            })?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| FsError::Budget {
                kind: BudgetKind::Allocation,
                limit: limit.get(),
            })?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 16 * 1024];
        let mut reached_eof = false;
        while bytes.len() < capacity {
            let count = (capacity - bytes.len()).min(buffer.len());
            let read = read_at(&self.descriptor, &mut buffer[..count], bytes.len() as u64)?;
            if read == 0 {
                reached_eof = true;
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            hasher.update(&buffer[..read]);
        }
        let mut probe = [0_u8; 1];
        let truncated =
            !reached_eof && read_at(&self.descriptor, &mut probe, bytes.len() as u64)? != 0;
        self.ensure_unchanged()?;
        if (!truncated && self.stamp.size != bytes.len() as u64)
            || (truncated && self.stamp.size <= bytes.len() as u64)
        {
            return Err(FsError::ChangedSource);
        }
        let extent = if truncated {
            ReadExtent::Truncated {
                limit,
                observed_size: self.stamp.size,
            }
        } else {
            ReadExtent::Complete
        };
        Ok(ReadResult {
            bytes,
            digest: finish_digest(hasher)?,
            identity: self.stamp.identity,
            extent,
        })
    }

    /// Conditional writes need a full digest, not an allocated copy of the old
    /// authored file. Stream the admitted size and refuse any partial revision.
    pub(super) fn revision(&self, limit: ByteCount) -> Result<ContentDigest, FsError> {
        self.ensure_unchanged()?;
        if self.stamp.size > limit.get() {
            return Err(FsError::Budget {
                kind: BudgetKind::Read,
                limit: limit.get(),
            });
        }
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 16 * 1024];
        let mut offset = 0;
        while offset < self.stamp.size {
            let count = (self.stamp.size - offset).min(buffer.len() as u64) as usize;
            let read = read_at(&self.descriptor, &mut buffer[..count], offset)?;
            if read == 0 {
                return Err(FsError::ChangedSource);
            }
            hasher.update(&buffer[..read]);
            offset += read as u64;
        }
        let mut probe = [0_u8; 1];
        if read_at(&self.descriptor, &mut probe, offset)? != 0 {
            return Err(FsError::ChangedSource);
        }
        self.ensure_unchanged()?;
        finish_digest(hasher)
    }

    pub(super) fn ensure_unchanged(&self) -> Result<(), FsError> {
        let opened = FileStamp::regular(&fstat(&self.descriptor).map_err(FsError::from)?)?;
        let named = statat(
            &self.target.parent,
            &self.target.name,
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(|error| match error {
            Errno::NOENT => FsError::ChangedSource,
            _ => FsError::from(error),
        })?;
        if opened != self.stamp || FileStamp::regular(&named).ok() != Some(self.stamp) {
            return Err(FsError::ChangedSource);
        }
        Ok(())
    }
}

pub(super) fn open_regular(
    parent: &OwnedFd,
    name: &OsStr,
    access: OFlags,
) -> Result<(OwnedFd, FileStamp), FsError> {
    // NONBLOCK must be supplied before fstat so a FIFO cannot hang admission.
    let flags = access | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
    let descriptor = openat(parent, name, flags, Mode::RUSR | Mode::WUSR).map_err(|error| {
        if matches!(error, Errno::NXIO | Errno::ISDIR)
            && let Ok(stat) = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)
            && FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        {
            return FsError::NonRegular;
        }
        FsError::from(error)
    })?;
    let stamp = FileStamp::regular(&fstat(&descriptor).map_err(FsError::from)?)?;
    Ok((descriptor, stamp))
}

fn read_at(descriptor: &OwnedFd, bytes: &mut [u8], offset: u64) -> Result<usize, FsError> {
    loop {
        match pread(descriptor, &mut *bytes, offset) {
            Err(Errno::INTR) => continue,
            result => return result.map_err(FsError::from),
        }
    }
}
