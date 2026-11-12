//! The write protocol (design section 8.4). Under one per-KB lock, each
//! operation writes its bytes to a dot-named temporary file beside the
//! target (or a staged directory in the Git directory), syncs it, re-checks
//! what it replaces, publishes by `link` (create-new), `rename` (replace)
//! or a directory `rename`, and syncs the parent directory. A reader, or a
//! process killed at any step, sees the old complete bytes or the new
//! complete bytes, never a part. Leftover temporary files are dot-named,
//! end in `.tmp` and are never inputs.
use super::{Op, Probe, Step};
use crate::error::{KbError, Result};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
/// How long a writer waits for another to release the KB write lock.
const LOCK_WAIT: Duration = Duration::from_secs(2);

/// The per-KB write lock, an advisory lock on a local file in the Git
/// directory. Released when dropped (and by the kernel when the process
/// dies).
pub struct Lock {
    file: File,
}

impl Lock {
    pub fn acquire(state: &Path) -> Result<Self> {
        std::fs::create_dir_all(state)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(state.join("kb-write.lock"))?;
        let deadline = Instant::now() + LOCK_WAIT;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    return Err(KbError::Refused(
                        "another writer holds the KB write lock; try again".into(),
                    ));
                }
                Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
            }
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// SHA-256 of some bytes, as lower-case hex.
pub fn digest(bytes: &[u8]) -> String {
    crate::index::hex(&Sha256::digest(bytes))
}

fn temporary_name() -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!(".writfold-kb-{}-{n}.tmp", std::process::id())
}

pub fn sync_dir(dir: &Path) -> Result<()> {
    File::open(dir)?.sync_all()?;
    Ok(())
}

fn conflict(what: String) -> KbError {
    KbError::Refused(format!("write conflict: {what}"))
}

/// A synced temporary file in `dir`.
fn temporary(dir: &Path, bytes: &[u8], probe: Probe) -> Result<PathBuf> {
    let path = dir.join(temporary_name());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(bytes)?;
    probe(Step::TempWritten);
    file.sync_all()?;
    probe(Step::TempSynced);
    Ok(path)
}

fn parent(path: &Path) -> Result<&Path> {
    path.parent()
        .ok_or_else(|| KbError::Usage(format!("{} has no parent", path.display())))
}

/// Make the missing directories above `path`, syncing each new one's parent.
fn directories(root: &Path, path: &Path) -> Result<()> {
    let dir = parent(path)?;
    if dir.is_dir() {
        return Ok(());
    }
    if dir == root {
        return Err(KbError::Usage("the KB root is missing".into()));
    }
    directories(root, dir)?;
    std::fs::create_dir(dir)?;
    sync_dir(parent(dir)?)
}

/// Check every guard before any byte is written.
fn check(root: &Path, op: &Op) -> Result<()> {
    match op {
        Op::Create { path, .. } | Op::CreateDir { path, .. } => {
            if root.join(path).symlink_metadata().is_ok() {
                return Err(conflict(format!("{path} already exists")));
            }
        }
        Op::Replace { path, expected, .. } => {
            let now =
                std::fs::read(root.join(path)).map_err(|_| conflict(format!("{path} is gone")))?;
            if digest(&now) != *expected {
                return Err(conflict(format!("{path} changed since it was read")));
            }
        }
        Op::AbsentPrefix { dir, prefix, what } => {
            let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
                return Ok(());
            };
            for entry in entries {
                let name = entry?.file_name().to_string_lossy().into_owned();
                if name.starts_with(prefix.as_str()) {
                    return Err(conflict(format!("{what} already exists as {dir}/{name}")));
                }
            }
        }
    }
    Ok(())
}

/// Publish one operation.
fn publish(root: &Path, staging: &Path, op: &Op, probe: Probe) -> Result<()> {
    match op {
        Op::AbsentPrefix { .. } => Ok(()),
        Op::Create { path, bytes } => {
            let target = root.join(path);
            directories(root, &target)?;
            let dir = parent(&target)?;
            let temp = temporary(dir, bytes, probe)?;
            probe(Step::Rechecked);
            let linked = std::fs::hard_link(&temp, &target);
            let _ = std::fs::remove_file(&temp);
            match linked {
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    return Err(conflict(format!("{path} already exists")));
                }
                other => other?,
            }
            probe(Step::Published);
            sync_dir(dir)?;
            probe(Step::DirSynced);
            Ok(())
        }
        Op::Replace {
            path,
            expected,
            bytes,
        } => {
            let target = root.join(path);
            let dir = parent(&target)?;
            let temp = temporary(dir, bytes, probe)?;
            let now = std::fs::read(&target).unwrap_or_default();
            if digest(&now) != *expected {
                let _ = std::fs::remove_file(&temp);
                return Err(conflict(format!("{path} changed since it was read")));
            }
            probe(Step::Rechecked);
            std::fs::rename(&temp, &target)?;
            probe(Step::Published);
            sync_dir(dir)?;
            probe(Step::DirSynced);
            Ok(())
        }
        Op::CreateDir { path, files } => create_dir(root, staging, path, files, probe),
    }
}

/// Stage a directory with its files in the Git directory, then rename it
/// into place: the KB sees no directory, or all of it.
fn create_dir(
    root: &Path,
    staging: &Path,
    path: &str,
    files: &[(String, Vec<u8>)],
    probe: Probe,
) -> Result<()> {
    std::fs::create_dir_all(staging)?;
    let stage = staging.join(temporary_name());
    std::fs::create_dir(&stage)?;
    for (name, bytes) in files {
        if let Some(dir) = name.strip_suffix('/') {
            std::fs::create_dir_all(stage.join(dir))?;
            continue;
        }
        let file = stage.join(name);
        std::fs::create_dir_all(parent(&file)?)?;
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)?;
        out.write_all(bytes)?;
        out.sync_all()?;
        sync_dir(parent(&file)?)?;
    }
    probe(Step::TempWritten);
    sync_dir(&stage)?;
    probe(Step::TempSynced);
    let target = root.join(path);
    directories(root, &target)?;
    if target.symlink_metadata().is_ok() {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(conflict(format!("{path} already exists")));
    }
    probe(Step::Rechecked);
    if let Err(error) = std::fs::rename(&stage, &target) {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(error.into());
    }
    probe(Step::Published);
    sync_dir(parent(&target)?)?;
    probe(Step::DirSynced);
    Ok(())
}

/// Apply `ops` under the KB write lock held in `state`.
pub fn apply(root: &Path, state: &Path, ops: &[Op], probe: Probe) -> Result<()> {
    let lock = Lock::acquire(state)?;
    probe(Step::Locked);
    apply_held(root, state, ops, probe, &lock)?;
    drop(lock);
    probe(Step::Unlocked);
    Ok(())
}

/// Apply `ops` while the caller holds the KB write lock (the edit adapter
/// holds it across its worktree writes and its receipt).
pub fn apply_held(root: &Path, state: &Path, ops: &[Op], probe: Probe, _held: &Lock) -> Result<()> {
    for op in ops {
        check(root, op)?;
    }
    let staging = state.join("staging");
    for op in ops {
        publish(root, &staging, op, probe)?;
    }
    Ok(())
}
