use super::super::{BudgetKind, FsError, PinnedFile};
use super::inspect::{check_named, identity, metadata, no_attributes, same_permissions};
use super::{RetainedStage, StageError, StageFailure, StageIdentity, StageMetadata};
use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags, fchmod, fchown, fstat, fsync, openat};
use rustix::io::{Errno, write};
use rustix::process::{Gid, Uid};
use std::ffi::OsStr;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

pub(super) fn create(
    target: &PinnedFile,
    parent: StageIdentity,
) -> Result<(OwnedFd, RetainedStage), StageFailure> {
    // In particular refuse a parent default ACL rather than silently inheriting it.
    no_attributes(&target.parent)?;
    for _ in 0..128 {
        let sequence = NEXT_STAGE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| {
                StageError::Filesystem(FsError::Budget {
                    kind: BudgetKind::TemporaryNames,
                    limit: u64::MAX,
                })
            })?;
        let name = format!(".writfold-stage-{}-{sequence}", std::process::id());
        if OsStr::new(&name) == target.name.as_os_str() {
            continue;
        }
        let flags = OFlags::RDWR
            | OFlags::CREATE
            | OFlags::EXCL
            | OFlags::NOFOLLOW
            | OFlags::CLOEXEC
            | OFlags::NONBLOCK;
        let descriptor = match openat(
            &target.parent,
            name.as_str(),
            flags,
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(descriptor) => descriptor,
            Err(Errno::EXIST) => continue,
            Err(error) => return Err(StageError::from(FsError::from(error)).into()),
        };
        let mut retained = RetainedStage {
            parent,
            name,
            identity: None,
        };
        match fstat(&descriptor) {
            Ok(stat) => retained.identity = Some(identity(&stat)),
            Err(error) => {
                return Err(StageFailure {
                    source: FsError::from(error).into(),
                    retained: Some(retained),
                });
            }
        }
        return Ok((descriptor, retained));
    }
    Err(StageError::Filesystem(FsError::Budget {
        kind: BudgetKind::TemporaryNames,
        limit: 128,
    })
    .into())
}

pub(super) fn fill(
    target: &PinnedFile,
    descriptor: &OwnedFd,
    retained: &RetainedStage,
    content: &[u8],
    admitted: &StageMetadata,
) -> Result<StageMetadata, StageError> {
    no_attributes(descriptor)?;
    let before = metadata(&fstat(descriptor).map_err(FsError::from)?)?;
    if Some(before.identity) != retained.identity
        || before.identity.device != retained.parent.device
    {
        return Err(StageError::Conflict);
    }
    let mut remaining = content;
    while !remaining.is_empty() {
        match write(descriptor, remaining) {
            Ok(0) => return Err(FsError::Io(Errno::IO).into()),
            Ok(count) => remaining = &remaining[count..],
            Err(Errno::INTR) => continue,
            Err(error) => return Err(FsError::from(error).into()),
        }
    }
    if (before.uid, before.gid) != (admitted.uid, admitted.gid) {
        fchown(
            descriptor,
            Some(Uid::from_raw(admitted.uid)),
            Some(Gid::from_raw(admitted.gid)),
        )
        .map_err(FsError::from)?;
    }
    fchmod(descriptor, Mode::from_raw_mode(admitted.mode & 0o777)).map_err(FsError::from)?;
    let after = metadata(&fstat(descriptor).map_err(FsError::from)?)?;
    if !same_permissions(&after, admitted) || after.length != content.len() as u64 {
        return Err(StageError::UnsupportedMetadata);
    }
    check_named(
        &target.parent,
        OsStr::new(&retained.name),
        descriptor,
        &after,
    )?;
    fsync(descriptor).map_err(FsError::from)?;
    // The durable owner witness must not refer to an unsynchronized stage name.
    fsync(&target.parent).map_err(FsError::from)?;
    check_named(
        &target.parent,
        OsStr::new(&retained.name),
        descriptor,
        &after,
    )?;
    Ok(after)
}
