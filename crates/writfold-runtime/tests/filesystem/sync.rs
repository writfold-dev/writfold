use super::support::{Fixture, TestResult, limit};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use writfold_runtime::fs::{FsError, LockKey, WriteError};

#[test]
fn complete_observed_revision_is_synchronized_without_inode_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock_private(&locks, &LockKey::new("record")?)?;
    let created = guard.create(b"\0original\xff\r\n", limit(64)?)?;
    let retained = guard.read(limit(64)?)?;
    let synced = guard.sync_existing(&retained)?;
    assert_eq!(synced.identity(), created.identity());
    assert_eq!(synced.digest(), created.digest());
    assert_eq!(synced.bytes(), retained.bytes().len());
    assert_eq!(fs::read(fixture.path.join("record"))?, retained.bytes());
    Ok(())
}

#[test]
fn synchronization_refuses_prefix_and_other_inode_even_with_identical_bytes() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock_private(&locks, &LockKey::new("record")?)?;
    guard.create(b"complete", limit(8)?)?;
    let prefix = guard.read(limit(4)?)?;
    assert!(matches!(
        guard.sync_existing(&prefix),
        Err(WriteError::NotPublished(FsError::ChangedSource))
    ));
    fs::write(fixture.path.join("same-bytes"), b"complete")?;
    let other = fixture.target("same-bytes")?.read(limit(8)?)?;
    assert!(matches!(
        guard.sync_existing(&other),
        Err(WriteError::NotPublished(FsError::ChangedSource))
    ));
    assert_eq!(fs::read(fixture.path.join("record"))?, b"complete");
    Ok(())
}

#[test]
fn synchronization_refuses_changed_content_metadata_and_hardlinks() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock_private(&locks, &LockKey::new("record")?)?;
    guard.create(b"original", limit(8)?)?;
    let original = guard.read(limit(8)?)?;
    fs::write(fixture.path.join("record"), b"corruption")?;
    assert!(matches!(
        guard.sync_existing(&original),
        Err(WriteError::NotPublished(FsError::ChangedSource))
    ));
    let changed = guard.read(limit(16)?)?;
    fs::set_permissions(
        fixture.path.join("record"),
        fs::Permissions::from_mode(0o400),
    )?;
    assert!(matches!(
        guard.sync_existing(&changed),
        Err(WriteError::NotPublished(FsError::ChangedSource))
    ));
    let metadata_changed = guard.read(limit(16)?)?;
    fs::hard_link(fixture.path.join("record"), fixture.path.join("alias"))?;
    assert!(matches!(
        guard.sync_existing(&metadata_changed),
        Err(WriteError::NotPublished(FsError::HardLinked))
    ));
    assert_eq!(fs::read(fixture.path.join("record"))?, b"corruption");
    assert_eq!(fs::read(fixture.path.join("alias"))?, b"corruption");
    Ok(())
}
