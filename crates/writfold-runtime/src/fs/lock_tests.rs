use super::{FsError, LockKey, ManagedLocks};
use crate::fs::{ContainedRoot, RelativePath};
use rustix::io::dup;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn dropping_logical_guard_releases_lock_despite_an_inherited_description()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "writfold-lock-release-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let root = ContainedRoot::create(&path)?;
    std::fs::write(path.join("record"), b"original")?;
    let locks = ManagedLocks::create(path.join("coordination"))?;
    let target = root.pin(&RelativePath::new("record")?)?;
    let key = LockKey::new("record")?;
    let first = target.try_lock(&locks, &key)?;
    // dup shares the same open file description, exactly as an unrelated fork
    // does before exec closes CLOEXEC descriptors. It does not acquire a new lock.
    let inherited = dup(&first.descriptor)?;
    drop(first);
    let next = target.try_lock_existing(&locks, &key)?;
    drop(inherited);
    // Closing that old description must not release the successor's own lock.
    assert!(matches!(
        target.try_lock_existing(&locks, &key),
        Err(FsError::LockBusy)
    ));
    drop(next);
    drop(target);
    drop(locks);
    drop(root);
    std::fs::remove_dir_all(path)?;
    Ok(())
}
