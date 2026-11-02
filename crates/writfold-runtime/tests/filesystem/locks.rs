use super::support::{Fixture, TestResult, limit};
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::sync::mpsc::sync_channel;
use writfold_runtime::fs::{FsError, LockKey, ManagedLocks, WriteError};

#[test]
fn managed_keys_are_validated_and_independent_of_paths() -> TestResult {
    for key in ["", ".", "..", "a/b", "a.b", "a\\b", "a b", "non-ascii-é"] {
        assert!(LockKey::new(key).is_err());
    }
    assert!(LockKey::new(&"a".repeat(129)).is_err());
    assert_eq!(LockKey::new("Record_1-stable")?.as_str(), "Record_1-stable");
    let fixture = Fixture::new()?;
    let namespace = fixture.locks()?;
    let reopened = ManagedLocks::open(fixture.path.join("managed-locks"))?;
    assert_eq!(fs::read_dir(fixture.path.join("managed-locks"))?.count(), 0);
    let before = fixture.target("before-rename")?;
    let after = fixture.target("after-rename")?;
    let key = LockKey::new("logical-record")?;
    let first = before.try_lock(&namespace, &key)?;
    let identity = first.lock_identity();
    assert!(matches!(
        after.try_lock(&reopened, &key),
        Err(FsError::LockBusy)
    ));
    drop(first);
    let second = after.try_lock(&reopened, &key)?;
    assert_eq!(second.lock_identity(), identity);
    drop(second);
    let metadata = fs::metadata(fixture.path.join("managed-locks/logical-record.lock"))?;
    assert_eq!(metadata.ino(), identity.inode());
    assert_eq!(metadata.dev(), identity.device());
    assert!(!fixture.path.join(".before-rename.kctx-lock").exists());
    assert!(!fixture.path.join(".after-rename.kctx-lock").exists());
    Ok(())
}

#[test]
fn managed_lock_entries_refuse_symlinks_nonregular_and_hardlinks() -> TestResult {
    let fixture = Fixture::new()?;
    let namespace = fixture.locks()?;
    let target = fixture.target("record")?;
    fs::write(fixture.path.join("outside"), b"untouched")?;
    symlink(
        fixture.path.join("outside"),
        fixture.path.join("managed-locks/symbolic.lock"),
    )?;
    assert!(matches!(
        target.try_lock(&namespace, &LockKey::new("symbolic")?),
        Err(FsError::Symlink)
    ));
    fs::create_dir(fixture.path.join("managed-locks/directory.lock"))?;
    assert!(matches!(
        target.try_lock(&namespace, &LockKey::new("directory")?),
        Err(FsError::NonRegular)
    ));
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        fixture.path.join("managed-locks/fifo.lock"),
        rustix::fs::Mode::RWXU,
    )?;
    assert!(matches!(
        target.try_lock(&namespace, &LockKey::new("fifo")?),
        Err(FsError::NonRegular)
    ));
    fs::hard_link(
        fixture.path.join("outside"),
        fixture.path.join("managed-locks/linked.lock"),
    )?;
    assert!(matches!(
        target.try_lock(&namespace, &LockKey::new("linked")?),
        Err(FsError::HardLinked)
    ));
    assert_eq!(fs::read(fixture.path.join("outside"))?, b"untouched");
    Ok(())
}

#[test]
fn changed_lock_inode_blocks_publication_and_is_never_unlinked() -> TestResult {
    let fixture = Fixture::new()?;
    let namespace = fixture.locks()?;
    let target = fixture.target("record")?;
    let mut guard = target.try_lock(&namespace, &LockKey::new("record")?)?;
    let original = fixture.path.join("managed-locks/record.lock");
    let moved = fixture.path.join("managed-locks/displaced.lock");
    fs::rename(&original, &moved)?;
    fs::write(&original, b"replacement")?;
    assert!(matches!(
        guard.create(b"new", limit(16)?),
        Err(WriteError::NotPublished(FsError::ChangedSource))
    ));
    drop(guard);
    assert!(moved.exists());
    assert_eq!(fs::read(original)?, b"replacement");
    assert!(!fixture.path.join("record").exists());
    Ok(())
}

#[test]
fn competing_writer_observes_busy_then_stale_revision() -> TestResult {
    let fixture = Fixture::new()?;
    let namespace = fixture.locks()?;
    let competitor_namespace = ManagedLocks::open(fixture.path.join("managed-locks"))?;
    let target = fixture.target("record")?;
    let competitor = fixture.target("record")?;
    let key = LockKey::new("record")?;
    let competitor_key = key.clone();
    let mut guard = target.try_lock(&namespace, &key)?;
    let budget = limit(64)?;
    let first = guard.create(b"first", budget)?;
    let stale = first.digest().clone();
    let (ready_tx, ready_rx) = sync_channel(0);
    let (continue_tx, continue_rx) = sync_channel(0);
    std::thread::scope(|scope| -> TestResult {
        let worker = scope.spawn(move || {
            let was_busy = matches!(
                competitor.try_lock(&competitor_namespace, &competitor_key),
                Err(FsError::LockBusy)
            );
            if ready_tx.send(()).is_err() || continue_rx.recv().is_err() {
                return false;
            }
            let Ok(mut acquired) = competitor.try_lock(&competitor_namespace, &competitor_key)
            else {
                return false;
            };
            was_busy
                && matches!(
                    acquired.compare_and_write(Some(&stale), b"loser", budget, budget),
                    Err(WriteError::NotPublished(FsError::StaleRevision { .. }))
                )
        });
        ready_rx.recv()?;
        let updated = guard.compare_and_write(Some(first.digest()), b"winner", budget, budget);
        drop(guard);
        continue_tx.send(())?;
        let observed = worker
            .join()
            .map_err(|_| std::io::Error::other("writer thread panicked"))?;
        updated?;
        assert!(observed);
        Ok(())
    })?;
    assert_eq!(fs::read(fixture.path.join("record"))?, b"winner");
    Ok(())
}
