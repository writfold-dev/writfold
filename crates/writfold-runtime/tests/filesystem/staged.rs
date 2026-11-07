use super::support::{Fixture, TestResult, limit};
use rustix::fs::{XattrFlags, setxattr};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use writfold_runtime::fs::{FsError, InstallError, LockKey, StageError, StageReconciliation};

#[test]
fn staged_content_preserves_regular_modes_and_ownership() -> TestResult {
    for mode in [0o644, 0o755] {
        let fixture = Fixture::new()?;
        let path = fixture.path.join("record");
        fs::write(&path, b"original")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
        let old = fs::metadata(&path)?;
        let locks = fixture.locks()?;
        let target = fixture.target("record")?;
        let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
        let observed = guard.observe_replacement(limit(64)?)?;
        assert_eq!(observed.preimage().bytes, b"original");
        assert_eq!(observed.preimage().metadata.mode, old.mode());
        assert_eq!(observed.preimage().metadata.links, 1);
        assert_eq!(observed.preimage().metadata.modified_seconds, old.mtime());
        assert_eq!(observed.preimage().metadata.changed_seconds, old.ctime());
        let stage = guard.stage_replacement(observed, b"new\0\xff", limit(64)?)?;
        let witness = stage.witness().clone();
        assert_eq!(fs::read(&path)?, b"original");
        assert_eq!(
            fs::read(fixture.path.join(&witness.stage_name))?,
            b"new\0\xff"
        );
        let installed = stage.install()?;
        let new = fs::metadata(&path)?;
        assert_eq!(new.mode() & 0o777, mode);
        assert_eq!((new.uid(), new.gid()), (old.uid(), old.gid()));
        assert_eq!(new.ino(), installed.identity.inode);
        assert_eq!(fs::read(&path)?, b"new\0\xff");
        assert!(!fixture.path.join(&witness.stage_name).exists());
    }
    Ok(())
}

#[test]
fn changed_preimage_refuses_staging_even_if_observed_digest_was_valid() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let observed = guard.observe_replacement(limit(64)?)?;
    fs::write(&path, b"different")?;
    let Err(failure) = guard.stage_replacement(observed, b"candidate", limit(64)?) else {
        return Err("changed preimage was admitted for staging".into());
    };
    assert!(matches!(failure.source, StageError::Conflict));
    assert!(failure.retained.is_none());
    assert_eq!(fs::read(&path)?, b"different");
    Ok(())
}

#[test]
fn exact_metadata_and_preimage_are_rechecked_before_install() -> TestResult {
    for metadata_only in [true, false] {
        let fixture = Fixture::new()?;
        let path = fixture.path.join("record");
        fs::write(&path, b"original")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
        let locks = fixture.locks()?;
        let target = fixture.target("record")?;
        let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
        let observed = guard.observe_replacement(limit(64)?)?;
        let stage = guard.stage_replacement(observed, b"candidate", limit(64)?)?;
        let witness = stage.witness().clone();
        if metadata_only {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        } else {
            fs::write(&path, b"modified")?;
        }
        assert!(matches!(
            stage.install(),
            Err(InstallError::NotInstalled(StageError::Conflict))
        ));
        assert!(fixture.path.join(&witness.stage_name).exists());
        assert!(matches!(
            guard.reconcile_stage(&witness, limit(64)?),
            StageReconciliation::Unknown(_)
        ));
    }
    Ok(())
}

#[test]
fn identical_bytes_with_changed_timestamps_are_not_the_preimage() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let observed = guard.observe_replacement(limit(64)?)?;
    // Deterministically change mtime without relying on clock resolution or sleeps.
    let times = fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH);
    fs::File::options()
        .write(true)
        .open(&path)?
        .set_times(times)?;
    assert!(
        matches!(guard.stage_replacement(observed, b"new", limit(64)?),
        Err(failure) if matches!(failure.source, StageError::Conflict))
    );
    assert_eq!(fs::read(&path)?, b"original");
    Ok(())
}

#[test]
fn symlinks_hardlinks_xattrs_and_special_modes_are_refused() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    symlink("record", fixture.path.join("symbolic"))?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let symbolic = fixture.target("symbolic")?;
    let guard = symbolic.try_lock(&locks, &LockKey::new("symbolic")?)?;
    assert!(matches!(
        guard.observe_replacement(limit(64)?),
        Err(StageError::Filesystem(FsError::Symlink))
    ));
    drop(guard);
    fs::hard_link(&path, fixture.path.join("alias"))?;
    let guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    assert!(matches!(
        guard.observe_replacement(limit(64)?),
        Err(StageError::Filesystem(FsError::HardLinked))
    ));
    fs::remove_file(fixture.path.join("alias"))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o2644))?;
    assert!(matches!(
        guard.observe_replacement(limit(64)?),
        Err(StageError::UnsupportedMetadata)
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
    setxattr(&path, "user.writfold-test", b"present", XattrFlags::empty())?;
    assert!(matches!(
        guard.observe_replacement(limit(64)?),
        Err(StageError::UnsupportedMetadata)
    ));
    Ok(())
}

#[test]
fn interrupted_stage_retains_witness_until_qualified_cleanup() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    fs::write(fixture.path.join(".writfold-stage-foreign"), b"unowned")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let key = LockKey::new("record")?;
    let mut guard = target.try_lock(&locks, &key)?;
    let stage =
        guard.stage_replacement(guard.observe_replacement(limit(64)?)?, b"new", limit(64)?)?;
    let witness = stage.witness().clone();
    drop(stage);
    drop(guard);
    assert!(fixture.path.join(&witness.stage_name).exists());
    let mut recovery = target.try_lock_existing(&locks, &key)?;
    assert!(matches!(
        recovery.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::NotInstalledCleaned
    ));
    assert!(!fixture.path.join(&witness.stage_name).exists());
    assert_eq!(fs::read(&path)?, b"original");
    assert_eq!(
        fs::read(fixture.path.join(".writfold-stage-foreign"))?,
        b"unowned"
    );
    // A lost cleanup receipt is not reconstructed as no-effect from absence alone.
    assert!(matches!(
        recovery.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::Unknown(_)
    ));
    Ok(())
}

#[test]
fn installed_then_lost_receipt_recovers_only_the_exact_stage_inode() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let key = LockKey::new("record")?;
    let mut guard = target.try_lock(&locks, &key)?;
    let stage =
        guard.stage_replacement(guard.observe_replacement(limit(64)?)?, b"new", limit(64)?)?;
    let witness = stage.witness().clone();
    let _lost_receipt = stage.install()?;
    drop(guard);
    let mut recovery = target.try_lock_existing(&locks, &key)?;
    assert!(matches!(recovery.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::InstalledObservedSync(receipt) if receipt.identity == witness.candidate.metadata.identity));
    assert_eq!(
        fs::metadata(&path)?.ino(),
        witness.candidate.metadata.identity.inode
    );
    // Reconciliation is repeatable observation, not a second replacement.
    assert!(matches!(
        recovery.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::InstalledObservedSync(_)
    ));
    assert_eq!(
        fs::metadata(&path)?.ino(),
        witness.candidate.metadata.identity.inode
    );
    Ok(())
}

#[test]
fn equal_candidate_bytes_on_foreign_inode_never_confirm() -> TestResult {
    let fixture = Fixture::new()?;
    let path = fixture.path.join("record");
    fs::write(&path, b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let stage =
        guard.stage_replacement(guard.observe_replacement(limit(64)?)?, b"new", limit(64)?)?;
    let witness = stage.witness().clone();
    fs::write(fixture.path.join("foreign"), b"new")?;
    fs::rename(fixture.path.join("foreign"), &path)?;
    assert_ne!(
        fs::metadata(&path)?.ino(),
        witness.candidate.metadata.identity.inode
    );
    drop(stage);
    assert!(matches!(
        guard.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::Unknown(_)
    ));
    assert!(fixture.path.join(&witness.stage_name).exists());
    Ok(())
}

#[test]
fn stage_substitution_is_neither_installed_nor_cleaned() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("record"), b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let stage =
        guard.stage_replacement(guard.observe_replacement(limit(64)?)?, b"new", limit(64)?)?;
    let witness = stage.witness().clone();
    let path = fixture.path.join(&witness.stage_name);
    fs::rename(&path, fixture.path.join("retained-original-stage"))?;
    fs::write(&path, b"new")?;
    assert!(matches!(
        stage.install(),
        Err(InstallError::NotInstalled(_))
    ));
    assert!(matches!(
        guard.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::Unknown(_)
    ));
    assert_eq!(fs::read(&path)?, b"new");
    assert_eq!(fs::read(fixture.path.join("record"))?, b"original");
    Ok(())
}

#[test]
fn stale_observation_cannot_stage_again_after_an_install() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("record"), b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let first_observation = guard.observe_replacement(limit(64)?)?;
    let stale = guard.observe_replacement(limit(64)?)?;
    guard
        .stage_replacement(first_observation, b"first", limit(64)?)?
        .install()?;
    let Err(failure) = guard.stage_replacement(stale, b"second", limit(64)?) else {
        return Err("stale observation was admitted after installation".into());
    };
    assert!(matches!(failure.source, StageError::Conflict));
    assert!(failure.retained.is_none());
    assert_eq!(fs::read(fixture.path.join("record"))?, b"first");
    Ok(())
}

#[test]
fn persisted_witness_is_bounded_and_cannot_replace_native_evidence() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("record"), b"original")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
    let stage =
        guard.stage_replacement(guard.observe_replacement(limit(64)?)?, b"new", limit(64)?)?;
    let mut witness = stage.witness().clone();
    drop(stage);
    assert!(matches!(
        guard.reconcile_stage(&witness, limit(1)?),
        StageReconciliation::Unknown(_)
    ));
    witness.candidate.bytes = b"bad".to_vec();
    assert!(matches!(
        guard.reconcile_stage(&witness, limit(64)?),
        StageReconciliation::Unknown(StageError::InvalidWitness)
    ));
    assert!(fixture.path.join(&witness.stage_name).exists());
    Ok(())
}

#[test]
fn an_observation_of_another_target_cannot_authorize_staging() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("record"), b"same")?;
    fs::write(fixture.path.join("other"), b"same")?;
    let locks = fixture.locks()?;
    let target = fixture.target("record")?;
    let other = fixture.target("other")?;
    let first = target.try_lock(&locks, &LockKey::new("record")?)?;
    let mut second = other.try_lock(&locks, &LockKey::new("other")?)?;
    let observed = first.observe_replacement(limit(64)?)?;
    assert!(
        matches!(second.stage_replacement(observed, b"new", limit(64)?),
        Err(failure) if matches!(failure.source, StageError::Conflict) && failure.retained.is_none())
    );
    assert_eq!(fs::read(fixture.path.join("other"))?, b"same");
    Ok(())
}

#[test]
fn modified_stage_attributes_and_contents_are_never_installed_or_cleaned() -> TestResult {
    for attributes in [true, false] {
        let fixture = Fixture::new()?;
        fs::write(fixture.path.join("record"), b"original")?;
        let locks = fixture.locks()?;
        let target = fixture.target("record")?;
        let mut guard = target.try_lock(&locks, &LockKey::new("record")?)?;
        let stage =
            guard.stage_replacement(guard.observe_replacement(limit(64)?)?, b"new", limit(64)?)?;
        let witness = stage.witness().clone();
        let path = fixture.path.join(&witness.stage_name);
        if attributes {
            setxattr(&path, "user.writfold-test", b"present", XattrFlags::empty())?;
        } else {
            fs::write(&path, b"bad")?;
        }
        assert!(matches!(
            stage.install(),
            Err(InstallError::NotInstalled(_))
        ));
        assert!(matches!(
            guard.reconcile_stage(&witness, limit(64)?),
            StageReconciliation::Unknown(_)
        ));
        assert!(path.exists());
        assert_eq!(fs::read(fixture.path.join("record"))?, b"original");
    }
    Ok(())
}
