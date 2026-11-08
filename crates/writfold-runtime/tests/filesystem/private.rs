use super::support::{Fixture, TestResult, limit};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use writfold_runtime::fs::{ContainedRoot, FsError, LockKey, PathError, RelativePath};

#[test]
fn child_creation_is_single_component_private_and_exclusive() -> TestResult {
    let fixture = Fixture::new()?;
    let root = ContainedRoot::open(&fixture.path)?;
    root.check_private()?;
    assert!(matches!(
        root.create_child(&RelativePath::new("missing/child")?),
        Err(FsError::InvalidPath(PathError::MultipleComponents))
    ));
    assert!(!fixture.path.join("missing").exists());
    let child = root.create_child(&RelativePath::new("child")?)?;
    child.check_private()?;
    assert_eq!(
        fs::metadata(fixture.path.join("child"))?
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    assert!(matches!(
        root.create_child(&RelativePath::new("child")?),
        Err(FsError::AlreadyExists)
    ));
    symlink("child", fixture.path.join("alias"))?;
    assert!(matches!(
        root.create_child(&RelativePath::new("alias")?),
        Err(FsError::AlreadyExists)
    ));
    assert!(fs::read_dir(fixture.path.join("child"))?.next().is_none());
    Ok(())
}

#[test]
fn child_and_parent_path_checks_refuse_observed_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let root = ContainedRoot::create(fixture.path.join("root"))?;
    let child = root.create_child(&RelativePath::new("child")?)?;
    fs::rename(
        fixture.path.join("root/child"),
        fixture.path.join("root/moved"),
    )?;
    fs::create_dir(fixture.path.join("root/child"))?;
    assert!(matches!(child.sync(), Err(FsError::ChangedSource)));
    fs::rename(fixture.path.join("root"), fixture.path.join("detached"))?;
    fs::create_dir(fixture.path.join("root"))?;
    assert!(matches!(
        root.create_child(&RelativePath::new("new")?),
        Err(FsError::ChangedSource)
    ));
    assert!(!fixture.path.join("detached/new").exists());
    assert!(!fixture.path.join("root/new").exists());
    Ok(())
}

#[test]
fn private_file_admission_rejects_shared_access_links_and_nonregulars() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    let target = fixture.target("record")?;
    for mode in [0o644, 0o640, 0o601, 0o700, 0o4600] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
        assert!(matches!(target.check_private(), Err(FsError::Permission)));
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    let identity = target.check_private()?;
    assert_eq!(target.read(limit(8)?)?.identity(), identity);
    fs::hard_link(&path, fixture.path.join("alias"))?;
    assert!(matches!(target.check_private(), Err(FsError::HardLinked)));
    assert_eq!(fs::read(&path)?, b"original");
    fs::create_dir(fixture.path.join("directory"))?;
    assert!(matches!(
        fixture.target("directory")?.check_private(),
        Err(FsError::NonRegular)
    ));
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        fixture.path.join("fifo"),
        rustix::fs::Mode::RWXU,
    )?;
    assert!(matches!(
        fixture.target("fifo")?.check_private(),
        Err(FsError::NonRegular)
    ));
    Ok(())
}

#[test]
fn private_locks_refuse_insecure_existing_entries_without_modifying_them() -> TestResult {
    let fixture = Fixture::new()?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let key = LockKey::new("record")?;
    let entry = fixture.path.join("managed-locks/record.lock");
    fs::write(&entry, b"foreign lock bytes")?;
    fs::set_permissions(&entry, fs::Permissions::from_mode(0o644))?;
    assert!(matches!(
        target.try_lock_private(&locks, &key),
        Err(FsError::Permission)
    ));
    fs::set_permissions(&entry, fs::Permissions::from_mode(0o600))?;
    fs::hard_link(&entry, fixture.path.join("alias"))?;
    assert!(matches!(
        target.try_lock_private(&locks, &key),
        Err(FsError::HardLinked)
    ));
    assert_eq!(fs::read(&entry)?, b"foreign lock bytes");
    assert!(!fixture.path.join("record").exists());
    fs::set_permissions(
        fixture.path.join("managed-locks"),
        fs::Permissions::from_mode(0o755),
    )?;
    assert!(matches!(locks.check_private(), Err(FsError::Permission)));
    Ok(())
}
