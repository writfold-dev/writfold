//! Incremental refresh, keyed by the Git commit and the dirty set.
//!
//! The changed paths are those `diff-tree` reports between the stored and
//! the current HEAD, plus the union of the stored and current dirty sets.
//! Each is re-read; one whose digest still equals its `files` row is left
//! alone. The rest have their Phase A rows replaced (or removed, when the
//! path is no longer an input), then Phase B is recomputed in full, all in
//! one `BEGIN IMMEDIATE` transaction that also moves the stored snapshot.
use super::build::{delete_file, digest, insert_file};
use super::{Pin, Snapshot, derive, input, listed, snapshot_meta};
use crate::error::Result;
use crate::layout::classify;
use crate::workspace::{CANCELLED, Repo, Workspace, budgets};
use rusqlite::{Connection, ErrorCode, OptionalExtension, TransactionBehavior};
use std::collections::{BTreeMap, BTreeSet};
use writfold_runtime::git::{GitExecutable, kb_diff_tree};

pub enum Outcome {
    /// The stored index is at the snapshot already.
    Current,
    /// Refreshed; the number of files re-derived.
    Done(usize),
    /// The write lock was busy: the stored pin and the changed-path count.
    Behind(Pin, usize),
    /// The stored head is unknown here (or there is none): rebuild.
    Rebuild,
}

/// The snapshot an index was derived at, from its `meta` rows.
struct Stored {
    pin: Pin,
    dirty: Vec<String>,
}

fn stored(conn: &Connection) -> Result<Option<Stored>> {
    let read = |key: &str| -> Result<Option<String>> {
        Ok(conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    };
    let (Some(digest), Some(head), Some(set)) = (read("pin")?, read("head")?, read("dirty_set")?)
    else {
        return Ok(None);
    };
    let dirty: Vec<String> = set
        .lines()
        .filter_map(|line| line.split_once('\0').map(|(path, _)| path.to_owned()))
        .collect();
    Ok(Some(Stored {
        pin: Pin {
            head: (head != "none").then_some(head),
            dirty: dirty.len(),
            digest,
        },
        dirty,
    }))
}

fn busy(error: &rusqlite::Error) -> bool {
    matches!(error, rusqlite::Error::SqliteFailure(e, _)
        if matches!(e.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked))
}

/// Paths that may differ between the stored snapshot and `now`, or `None`
/// when the stored head cannot be compared with the current one.
fn changed(
    repo: &Repo,
    git: &GitExecutable<'_>,
    old: &Stored,
    now: &Snapshot,
) -> Result<Option<BTreeSet<String>>> {
    let mut paths = BTreeSet::new();
    if old.pin.head != now.pin.head {
        let (Some(from), Some(to)) = (&old.pin.head, &now.pin.head) else {
            return Ok(None);
        };
        let pathspec = repo.pathspec();
        let Some(diff) = kb_diff_tree(
            git,
            repo.reads(),
            from,
            to,
            pathspec,
            budgets()?,
            &CANCELLED,
        )?
        else {
            return Ok(None);
        };
        paths.extend(
            diff.iter()
                .filter_map(|p| p.strip_prefix(repo.prefix.as_str()))
                .filter(|p| classify(p).is_some())
                .map(str::to_owned),
        );
    }
    paths.extend(old.dirty.iter().cloned());
    paths.extend(now.dirty.iter().map(|(path, _)| path.clone()));
    Ok(Some(paths))
}

pub fn refresh(
    conn: &mut Connection,
    ws: &Workspace,
    repo: &Repo,
    git: &GitExecutable<'_>,
    now: &Snapshot,
) -> Result<Outcome> {
    let Some(old) = stored(conn)? else {
        return Ok(Outcome::Rebuild);
    };
    if old.pin.digest == now.pin.digest {
        return Ok(Outcome::Current);
    }
    let Some(paths) = changed(repo, git, &old, now)? else {
        return Ok(Outcome::Rebuild);
    };
    // Membership is decided by Git, exactly as a rebuild decides it.
    let members: BTreeSet<String> = if paths.is_empty() {
        BTreeSet::new()
    } else {
        listed(repo, git)?.into_iter().collect()
    };
    let tx = match conn.transaction_with_behavior(TransactionBehavior::Immediate) {
        Ok(tx) => tx,
        Err(error) if busy(&error) => return Ok(Outcome::Behind(old.pin, paths.len())),
        Err(error) => return Err(error.into()),
    };
    // Another process may have refreshed while this one waited.
    if stored(&tx)?.is_some_and(|s| s.pin.digest == now.pin.digest) {
        return Ok(Outcome::Current);
    }
    let mut count = 0;
    for path in &paths {
        let fresh = members
            .contains(path)
            .then(|| input(&ws.root, path.clone()))
            .flatten();
        let prior: Option<(Vec<u8>, String)> = tx
            .prepare_cached("SELECT sha256, parse_status FROM files WHERE path = ?1")?
            .query_row([path], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        let unchanged = match (&fresh, &prior) {
            (None, None) => true,
            (Some(i), Some((d, _))) => i.bytes.is_ok() && digest(&i.bytes) == *d,
            _ => false,
        };
        if unchanged {
            continue;
        }
        let before = raws(&tx, path)?;
        delete_file(&tx, path)?;
        if let Some(fresh) = fresh {
            insert_file(&tx, fresh)?;
        }
        forget_verdicts(&tx, path, &before)?;
        count += 1;
    }
    if count > 0 {
        derive::phase_b(&tx)?;
    }
    for (key, value) in snapshot_meta(now) {
        tx.prepare_cached("INSERT OR REPLACE INTO meta(key, value) VALUES (?1, ?2)")?
            .execute((&key, &value))?;
    }
    match tx.commit() {
        Ok(()) => Ok(Outcome::Done(count)),
        Err(error) if busy(&error) => Ok(Outcome::Behind(old.pin, paths.len())),
        Err(error) => Err(error.into()),
    }
}

/// Citation token texts of a file, by ordinal.
fn raws(tx: &rusqlite::Transaction<'_>, path: &str) -> Result<BTreeMap<u32, String>> {
    let mut statement = tx.prepare_cached("SELECT ord, raw FROM citations WHERE path = ?1")?;
    let found = statement
        .query_map([path], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(found)
}

/// Drop cached verdicts of a re-derived file whose token at the same
/// ordinal changed, as a rebuild would.
fn forget_verdicts(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
    before: &BTreeMap<u32, String>,
) -> Result<()> {
    let after = raws(tx, path)?;
    let mut statement = tx.prepare_cached("SELECT ord FROM cite_checks WHERE path = ?1")?;
    let cached = statement
        .query_map([path], |r| r.get::<_, u32>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for ord in cached {
        if before.get(&ord).is_none() || before.get(&ord) != after.get(&ord) {
            tx.prepare_cached("DELETE FROM cite_checks WHERE path = ?1 AND ord = ?2")?
                .execute((path, ord))?;
        }
    }
    Ok(())
}
