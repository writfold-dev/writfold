//! Writing an edit into the worktree. Under the KB write lock (which
//! serialises Writfold's own writers; nothing stops another tool or the
//! agent's shell from editing the worktree meanwhile):
//!
//! 1. every file still holds the bytes the recipes were replayed against;
//! 2. the journal is made durable ([`super::journal`]);
//! 3. each file's after bytes go to a dot-named temporary beside it, with
//!    the file's permissions, synced;
//! 4. per file: the target is read again and must still hold the replayed
//!    bytes, then the temporary is renamed over it and the directory synced;
//! 5. the receipt (and a recorded `--worktree`) go to the KB through the
//!    write protocol; 6. the journal is removed.
//!
//! Each file is replaced atomically. When step 4 finds a file changed, the
//! files already renamed are restored to their before bytes and nothing is
//! left written. A crash between two renames leaves some files applied and
//! the journal in place; `edit recover` finishes the batch from it.
use super::journal::{Entry, Journal};
use crate::error::{KbError, Result};
use crate::write::atomic::{self, Lock, digest, sync_dir};
use crate::write::{Op, Probe, Step, no_probe};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A fresh dot-named temporary name for a worktree file.
pub fn temp_name() -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!(".writfold-edit-{}-{n}.tmp", std::process::id())
}

fn current(top: &Path, path: &str) -> Option<String> {
    std::fs::read(top.join(path)).ok().map(|b| digest(&b))
}

fn temp_path(top: &Path, entry: &Entry) -> PathBuf {
    let target = top.join(&entry.path);
    target.parent().unwrap_or(top).join(&entry.temp)
}

fn stage(top: &Path, entry: &Entry, probe: Probe) -> Result<()> {
    let temp = temp_path(top, entry);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(&entry.bytes)?;
    if let Ok(meta) = std::fs::metadata(top.join(&entry.path)) {
        file.set_permissions(meta.permissions())?;
    }
    probe(Step::TempWritten);
    file.sync_all()?;
    probe(Step::TempSynced);
    Ok(())
}

/// Rename the staged temporary over the file if it still holds `expected`;
/// `false` (temporary kept) when it does not.
fn publish(top: &Path, entry: &Entry, expected: &str, probe: Probe) -> Result<bool> {
    if current(top, &entry.path).as_deref() != Some(expected) {
        return Ok(false);
    }
    probe(Step::Rechecked);
    let target = top.join(&entry.path);
    std::fs::rename(temp_path(top, entry), &target)?;
    probe(Step::Published);
    sync_dir(target.parent().unwrap_or(top))?;
    probe(Step::DirSynced);
    Ok(true)
}

fn discard(top: &Path, entries: &[Entry]) {
    for entry in entries {
        let _ = std::fs::remove_file(temp_path(top, entry));
    }
}

/// Put back the before bytes of files already renamed, newest first; the
/// paths that could not be restored (changed again meanwhile).
fn restore(top: &Path, applied: &[Entry], befores: &[Vec<u8>]) -> Vec<String> {
    let mut stuck = Vec::new();
    for (entry, before) in applied.iter().zip(befores).rev() {
        let back = Entry {
            path: entry.path.clone(),
            before: entry.after.clone(),
            after: entry.before.clone(),
            temp: temp_name(),
            bytes: before.clone(),
        };
        let done =
            stage(top, &back, no_probe).and_then(|()| publish(top, &back, &back.before, no_probe));
        if !matches!(done, Ok(true)) {
            discard(top, std::slice::from_ref(&back));
            stuck.push(entry.path.clone());
        }
    }
    stuck
}

pub fn pending(run: &str) -> KbError {
    KbError::Refused(format!(
        "an edit in run {run} was interrupted; finish it with writfold kb edit recover --run {run} (or drop it with --abandon)"
    ))
}

/// Apply the journal's files and KB writes; `befores` are the replayed
/// bytes, in the journal's order.
pub fn apply(
    root: &Path,
    state: &Path,
    journal: &Journal,
    befores: &[Vec<u8>],
    probe: Probe,
) -> Result<()> {
    let lock = Lock::acquire(state)?;
    probe(Step::Locked);
    if Journal::load(state, &journal.run)?.is_some() {
        return Err(pending(&journal.run));
    }
    let top = &journal.worktree;
    for f in &journal.files {
        if current(top, &f.path).as_deref() != Some(f.before.as_str()) {
            return Err(KbError::Refused(format!(
                "{} changed since the recipe was replayed; nothing was written",
                f.path
            )));
        }
    }
    journal.store(state)?;
    probe(Step::Journaled);
    for f in &journal.files {
        if let Err(error) = stage(top, f, probe) {
            discard(top, &journal.files);
            Journal::clear(state, &journal.run)?;
            return Err(error);
        }
    }
    for (index, f) in journal.files.iter().enumerate() {
        if publish(top, f, &f.before, probe)? {
            continue;
        }
        discard(top, &journal.files[index..]);
        let stuck = restore(top, &journal.files[..index], befores);
        if stuck.is_empty() {
            Journal::clear(state, &journal.run)?;
            return Err(KbError::Refused(format!(
                "{} changed since the recipe was replayed; nothing was written ({index} file(s) already replaced were restored)",
                f.path
            )));
        }
        return Err(KbError::Refused(format!(
            "{} changed since the recipe was replayed, and {} changed again before it could be restored; the journal is kept: see writfold kb edit recover --run {}",
            f.path,
            stuck.join(", "),
            journal.run
        )));
    }
    atomic::apply_held(root, state, &journal.kb, probe, &lock)?;
    Journal::clear(state, &journal.run)?;
    probe(Step::Cleared);
    drop(lock);
    probe(Step::Unlocked);
    Ok(())
}

/// What `recover` found for one file.
pub struct Recovered {
    pub path: String,
    /// `applied` (it already held the after bytes), `completed` (written
    /// now), `unapplied` or `changed` (left as they are, with --abandon).
    pub state: &'static str,
}

/// KB writes of the journal not yet done (a crash may land between them).
fn remaining(root: &Path, ops: &[Op]) -> Vec<Op> {
    ops.iter()
        .filter(|op| match op {
            Op::Create { path, bytes } => {
                std::fs::read(root.join(path)).ok().as_ref() != Some(bytes)
            }
            Op::Replace { path, bytes, .. } => {
                std::fs::read(root.join(path)).ok().map(|b| digest(&b)) != Some(digest(bytes))
            }
            _ => true,
        })
        .cloned()
        .collect()
}

/// Finish (or, with `abandon`, drop) the interrupted edit of `run`.
pub fn recover(
    root: &Path,
    state: &Path,
    run: &str,
    abandon: bool,
    probe: Probe,
) -> Result<(Journal, Vec<Recovered>)> {
    let lock = Lock::acquire(state)?;
    probe(Step::Locked);
    let journal = Journal::load(state, run)?
        .ok_or_else(|| KbError::Refused(format!("no interrupted edit in run {run}")))?;
    let top = &journal.worktree;
    let mut found: Vec<Recovered> = journal
        .files
        .iter()
        .map(|f| {
            let now = current(top, &f.path);
            let state = match now.as_deref() {
                Some(d) if d == f.after => "applied",
                Some(d) if d == f.before => "unapplied",
                _ => "changed",
            };
            Recovered {
                path: f.path.clone(),
                state,
            }
        })
        .collect();
    discard(top, &journal.files);
    if abandon {
        Journal::clear(state, run)?;
        return Ok((journal, found));
    }
    let changed: Vec<&str> = found
        .iter()
        .filter(|r| r.state == "changed")
        .map(|r| r.path.as_str())
        .collect();
    if !changed.is_empty() {
        return Err(KbError::Refused(format!(
            "{} hold neither the replayed nor the edited bytes; resolve them by hand, then drop the journal with --abandon",
            changed.join(", ")
        )));
    }
    for (f, r) in journal.files.iter().zip(found.iter_mut()) {
        if r.state == "unapplied" {
            stage(top, f, probe)?;
            if !publish(top, f, &f.before, probe)? {
                discard(top, std::slice::from_ref(f));
                return Err(KbError::Refused(format!(
                    "{} changed during recovery",
                    f.path
                )));
            }
            r.state = "completed";
        }
    }
    atomic::apply_held(root, state, &remaining(root, &journal.kb), probe, &lock)?;
    Journal::clear(state, run)?;
    probe(Step::Cleared);
    drop(lock);
    probe(Step::Unlocked);
    Ok((journal, found))
}
