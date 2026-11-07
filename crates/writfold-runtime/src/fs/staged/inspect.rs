use super::super::identity::digest;
use super::super::read::open_regular;
use super::super::{BudgetKind, FsError, LockedFile};
use super::{StageError, StageIdentity, StageImage, StageMetadata, StageWitness};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FileType, OFlags, Stat, flistxattr, fstat, statat};
use rustix::io::{Errno, pread};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use writfold_domain::ByteCount;

pub(super) fn identity(stat: &Stat) -> StageIdentity {
    StageIdentity {
        device: stat.st_dev,
        inode: stat.st_ino,
    }
}

pub(super) fn metadata(stat: &Stat) -> Result<StageMetadata, StageError> {
    match FileType::from_raw_mode(stat.st_mode) {
        FileType::RegularFile => {}
        FileType::Symlink => return Err(FsError::Symlink.into()),
        _ => return Err(FsError::NonRegular.into()),
    }
    if stat.st_nlink != 1 {
        return Err(FsError::HardLinked.into());
    }
    if stat.st_mode & 0o7000 != 0 || stat.st_uid == u32::MAX || stat.st_gid == u32::MAX {
        return Err(StageError::UnsupportedMetadata);
    }
    if stat.st_mtime_nsec >= 1_000_000_000 || stat.st_ctime_nsec >= 1_000_000_000 {
        return Err(StageError::UnsupportedMetadata);
    }
    Ok(StageMetadata {
        identity: identity(stat),
        mode: stat.st_mode,
        uid: stat.st_uid,
        gid: stat.st_gid,
        links: stat.st_nlink,
        length: u64::try_from(stat.st_size).map_err(|_| StageError::Conflict)?,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: stat.st_mtime_nsec,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec,
    })
}

pub(super) fn no_attributes(descriptor: &OwnedFd) -> Result<(), StageError> {
    // A zero-sized query observes the required size without unbounded allocation.
    // Unsupported listing is a refusal, not an assumption that attributes are absent.
    let mut empty = [0_u8; 0];
    if flistxattr(descriptor, &mut empty[..]).map_err(FsError::from)? != 0 {
        return Err(StageError::UnsupportedMetadata);
    }
    Ok(())
}

pub(super) fn check_named(
    parent: &OwnedFd,
    name: &OsStr,
    descriptor: &OwnedFd,
    expected: &StageMetadata,
) -> Result<(), StageError> {
    no_attributes(descriptor)?;
    let opened = metadata(&fstat(descriptor).map_err(FsError::from)?)?;
    let named = metadata(&statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(FsError::from)?)?;
    if opened != *expected || named != *expected {
        return Err(StageError::Conflict);
    }
    Ok(())
}

pub(super) fn capture(
    parent: &OwnedFd,
    name: &OsStr,
    limit: ByteCount,
) -> Result<(OwnedFd, StageImage), StageError> {
    let (descriptor, _) = open_regular(parent, name, OFlags::RDONLY)?;
    let image = capture_open(parent, name, &descriptor, limit)?;
    Ok((descriptor, image))
}

pub(super) fn capture_open(
    parent: &OwnedFd,
    name: &OsStr,
    descriptor: &OwnedFd,
    limit: ByteCount,
) -> Result<StageImage, StageError> {
    let metadata = metadata(&fstat(descriptor).map_err(FsError::from)?)?;
    check_named(parent, name, descriptor, &metadata)?;
    let bytes = read_exact(descriptor, metadata.length, limit)?;
    check_named(parent, name, descriptor, &metadata)?;
    let digest = digest(&bytes)?;
    Ok(StageImage {
        metadata,
        bytes,
        digest,
    })
}

fn read_exact(descriptor: &OwnedFd, length: u64, limit: ByteCount) -> Result<Vec<u8>, StageError> {
    if length > limit.get() {
        return Err(FsError::Budget {
            kind: BudgetKind::Read,
            limit: limit.get(),
        }
        .into());
    }
    let size = usize::try_from(length).map_err(|_| FsError::Budget {
        kind: BudgetKind::Allocation,
        limit: limit.get(),
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| FsError::Budget {
        kind: BudgetKind::Allocation,
        limit: limit.get(),
    })?;
    bytes.resize(size, 0);
    let mut offset = 0;
    while offset < size {
        match pread(descriptor, &mut bytes[offset..], offset as u64) {
            Ok(0) => return Err(StageError::Conflict),
            Ok(count) => offset += count,
            Err(Errno::INTR) => continue,
            Err(error) => return Err(FsError::from(error).into()),
        }
    }
    let mut probe = [0_u8];
    loop {
        match pread(descriptor, &mut probe, length) {
            Ok(0) => return Ok(bytes),
            Ok(_) => return Err(StageError::Conflict),
            Err(Errno::INTR) => continue,
            Err(error) => return Err(FsError::from(error).into()),
        }
    }
}

pub(super) fn bindings(
    guard: &LockedFile<'_>,
) -> Result<(StageIdentity, StageIdentity), StageError> {
    guard.check_current()?;
    let parent = identity(&fstat(&guard.target.parent).map_err(FsError::from)?);
    let lock = guard.lock_identity();
    Ok((
        parent,
        StageIdentity {
            device: lock.device(),
            inode: lock.inode(),
        },
    ))
}

pub(super) fn validate(
    guard: &LockedFile<'_>,
    witness: &StageWitness,
    limit: ByteCount,
) -> Result<(), StageError> {
    if bindings(guard)? != (witness.parent, witness.lock)
        || guard.target.name.as_bytes() != witness.target_name
    {
        return Err(StageError::Conflict);
    }
    let suffix = witness
        .stage_name
        .strip_prefix(".writfold-stage-")
        .ok_or(StageError::InvalidWitness)?;
    if suffix.is_empty()
        || !suffix.bytes().all(|b| b.is_ascii_digit() || b == b'-')
        || witness.stage_name.as_bytes() == witness.target_name
        || witness.candidate.metadata.identity == witness.preimage.metadata.identity
        || witness.candidate.metadata.identity.device != witness.parent.device
        || !same_permissions(&witness.candidate.metadata, &witness.preimage.metadata)
    {
        return Err(StageError::InvalidWitness);
    }
    for image in [&witness.preimage, &witness.candidate] {
        if image.metadata.length > limit.get() {
            return Err(FsError::Budget {
                kind: BudgetKind::Read,
                limit: limit.get(),
            }
            .into());
        }
        if image.bytes.len() as u64 != image.metadata.length
            || FileType::from_raw_mode(image.metadata.mode) != FileType::RegularFile
            || image.metadata.mode & 0o7000 != 0
            || image.metadata.links != 1
            || image.metadata.modified_nanoseconds >= 1_000_000_000
            || image.metadata.changed_nanoseconds >= 1_000_000_000
            || image.metadata.uid == u32::MAX
            || image.metadata.gid == u32::MAX
            || digest(&image.bytes)? != image.digest
        {
            return Err(StageError::InvalidWitness);
        }
    }
    Ok(())
}

pub(super) fn same_permissions(left: &StageMetadata, right: &StageMetadata) -> bool {
    (left.mode, left.uid, left.gid) == (right.mode, right.uid, right.gid)
}

pub(super) fn match_image(
    parent: &OwnedFd,
    name: &OsStr,
    expected: &StageImage,
    installed: bool,
) -> Result<OwnedFd, StageError> {
    let (descriptor, _) = open_regular(parent, name, OFlags::RDONLY)?;
    let actual = metadata(&fstat(&descriptor).map_err(FsError::from)?)?;
    let mut admitted = expected.metadata;
    if installed {
        // Rename updates ctime; no other metadata drift is accepted.
        admitted.changed_seconds = actual.changed_seconds;
        admitted.changed_nanoseconds = actual.changed_nanoseconds;
    }
    if actual != admitted {
        return Err(StageError::Conflict);
    }
    check_named(parent, name, &descriptor, &actual)?;
    let mut buffer = [0_u8; 16 * 1024];
    let mut offset = 0;
    while offset < expected.bytes.len() {
        let count = (expected.bytes.len() - offset).min(buffer.len());
        match pread(&descriptor, &mut buffer[..count], offset as u64) {
            Ok(0) => return Err(StageError::Conflict),
            Ok(read) => {
                if buffer[..read] != expected.bytes[offset..offset + read] {
                    return Err(StageError::Conflict);
                }
                offset += read;
            }
            Err(Errno::INTR) => continue,
            Err(error) => return Err(FsError::from(error).into()),
        }
    }
    let mut probe = [0_u8];
    loop {
        match pread(&descriptor, &mut probe, offset as u64) {
            Ok(0) => break,
            Ok(_) => return Err(StageError::Conflict),
            Err(Errno::INTR) => continue,
            Err(error) => return Err(FsError::from(error).into()),
        }
    }
    check_named(parent, name, &descriptor, &actual)?;
    Ok(descriptor)
}
