use super::support::{Fixture, TestResult, limit, temporary_names};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use writfold_runtime::fs::{BudgetKind, ContainedRoot, FsError, LockKey, RelativePath, WriteError};

#[test]
fn conditional_create_and_replace_return_observed_sync_receipts() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("logical-record")?)?;
    let created = guard.compare_and_write(None, b"abc", limit(64)?, limit(3)?)?;
    assert_eq!(created.bytes(), 3);
    assert_eq!(
        created.digest().as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let read = guard.read(limit(64)?)?;
    assert_eq!(read.bytes(), b"abc");
    assert_eq!(read.identity(), created.identity());
    let updated =
        guard.compare_and_write(Some(created.digest()), b"\0\r\n\xff", limit(64)?, limit(4)?)?;
    assert_eq!(fs::read(fixture.path.join("record"))?, b"\0\r\n\xff");
    assert_eq!(target.read(limit(64)?)?.digest(), updated.digest());
    assert_ne!(created.identity(), updated.identity());
    assert_eq!(
        fs::metadata(fixture.path.join("record"))?
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(temporary_names(&fixture.path)?.is_empty());
    Ok(())
}

#[test]
fn stale_expected_revision_cannot_publish_or_truncate() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let first = guard.create(b"first", limit(64)?)?;
    guard.compare_and_write(Some(first.digest()), b"second", limit(64)?, limit(64)?)?;
    let stale = guard.compare_and_write(Some(first.digest()), b"stale", limit(64)?, limit(64)?);
    assert!(matches!(
        stale,
        Err(WriteError::NotPublished(FsError::StaleRevision { .. }))
    ));
    assert!(matches!(
        guard.compare_and_write(None, b"bad", limit(64)?, limit(64)?),
        Err(WriteError::NotPublished(FsError::StaleRevision { .. }))
    ));
    assert_eq!(fs::read(fixture.path.join("record"))?, b"second");
    assert!(temporary_names(&fixture.path)?.is_empty());
    Ok(())
}

#[test]
fn missing_expected_revision_is_a_stale_predecessor() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("absent")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let expected = writfold_domain::ContentDigest::try_from("a".repeat(64))?;
    assert!(matches!(
        guard.compare_and_write(Some(&expected), b"new", limit(64)?, limit(64)?),
        Err(WriteError::NotPublished(FsError::StaleRevision {
            actual: None,
            ..
        }))
    ));
    assert!(!fixture.path.join("absent").exists());
    Ok(())
}

#[test]
fn read_and_write_budgets_fail_before_publication() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let initial = guard.create(b"abcdef", limit(6)?)?;
    assert!(matches!(
        guard.compare_and_write(Some(initial.digest()), b"new", limit(3)?, limit(3)?),
        Err(WriteError::NotPublished(FsError::Budget {
            kind: BudgetKind::Read,
            ..
        }))
    ));
    assert!(matches!(
        guard.compare_and_write(Some(initial.digest()), b"new", limit(6)?, limit(2)?),
        Err(WriteError::NotPublished(FsError::Budget {
            kind: BudgetKind::Write,
            ..
        }))
    ));
    assert!(matches!(
        guard.create(b"new", limit(0)?),
        Err(WriteError::NotPublished(FsError::Budget {
            kind: BudgetKind::Write,
            ..
        }))
    ));
    assert_eq!(fs::read(fixture.path.join("record"))?, b"abcdef");
    assert!(temporary_names(&fixture.path)?.is_empty());
    Ok(())
}

#[test]
fn atomic_create_collision_keeps_existing_and_unowned_temporary_entries() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("record"), b"existing")?;
    fs::write(fixture.path.join(".writfold-tmp-foreign"), b"not-ours")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    assert!(matches!(
        guard.create(b"replacement", limit(64)?),
        Err(WriteError::NotPublished(FsError::AlreadyExists))
    ));
    assert_eq!(fs::read(fixture.path.join("record"))?, b"existing");
    assert_eq!(
        fs::read(fixture.path.join(".writfold-tmp-foreign"))?,
        b"not-ours"
    );
    assert_eq!(
        temporary_names(&fixture.path)?,
        [OsString::from(".writfold-tmp-foreign")]
    );
    Ok(())
}

#[test]
fn conditional_authored_updates_refuse_hardlinks() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("record"), b"authored")?;
    fs::hard_link(fixture.path.join("record"), fixture.path.join("alias"))?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let original = target.read(limit(64)?)?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    assert!(matches!(
        guard.compare_and_write(
            Some(original.digest()),
            b"replacement",
            limit(64)?,
            limit(64)?
        ),
        Err(WriteError::NotPublished(FsError::HardLinked))
    ));
    assert_eq!(fs::read(fixture.path.join("record"))?, b"authored");
    assert_eq!(fs::read(fixture.path.join("alias"))?, b"authored");
    Ok(())
}

#[test]
fn conditional_writes_refuse_symlink_leaves_without_touching_targets() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("outside"), b"outside")?;
    symlink("outside", fixture.path.join("record"))?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    assert!(matches!(
        guard.compare_and_write(None, b"new", limit(64)?, limit(64)?),
        Err(WriteError::NotPublished(FsError::Symlink))
    ));
    assert_eq!(fs::read(fixture.path.join("outside"))?, b"outside");
    assert!(
        fs::symlink_metadata(fixture.path.join("record"))?
            .file_type()
            .is_symlink()
    );
    Ok(())
}

#[test]
fn pinned_parent_is_used_for_reads_and_writes_after_directory_rename() -> TestResult {
    let fixture = Fixture::new()?;
    fs::create_dir(fixture.path.join("parent"))?;
    fs::create_dir(fixture.path.join("outside"))?;
    fs::write(fixture.path.join("parent/record"), b"inside")?;
    fs::write(fixture.path.join("outside/record"), b"outside")?;
    let root = ContainedRoot::open(&fixture.path)?;
    let target = root.pin(&RelativePath::new("parent/record")?)?;
    fs::rename(fixture.path.join("parent"), fixture.path.join("moved"))?;
    symlink("outside", fixture.path.join("parent"))?;
    let locks = fixture.locks()?;
    let mut guard = target.try_lock(&locks, &LockKey::new("stable-record")?)?;
    let observed = guard.read(limit(64)?)?;
    assert_eq!(observed.bytes(), b"inside");
    guard.compare_and_write(Some(observed.digest()), b"updated", limit(64)?, limit(64)?)?;
    assert_eq!(fs::read(fixture.path.join("moved/record"))?, b"updated");
    assert_eq!(fs::read(fixture.path.join("outside/record"))?, b"outside");
    assert!(temporary_names(&fixture.path.join("moved"))?.is_empty());
    Ok(())
}

#[test]
fn creation_never_uses_destination_as_staging() -> TestResult {
    const CHILD: &str = "WRITFOLD_FS_DESTINATION_STAGE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let fixture = Fixture::new()?;
        let destination = format!(".writfold-tmp-{}-0", std::process::id());
        let collision = format!(".writfold-tmp-{}-1", std::process::id());
        fs::write(fixture.path.join(&collision), b"foreign user data")?;
        let target = fixture.target(&destination)?;
        let locks = fixture.locks()?;
        let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
        guard.create(b"complete", limit(8)?)?;
        assert_eq!(fs::read(fixture.path.join(&destination))?, b"complete");
        assert_eq!(
            fs::read(fixture.path.join(&collision))?,
            b"foreign user data"
        );
        let mut expected = vec![OsString::from(destination), OsString::from(collision)];
        expected.sort();
        assert_eq!(temporary_names(&fixture.path)?, expected);
        return Ok(());
    }
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "writes::creation_never_uses_destination_as_staging",
        ])
        .env(CHILD, "1")
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
