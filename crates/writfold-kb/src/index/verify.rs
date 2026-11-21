//! The index equals a rebuild: every derived table, selected in primary-key
//! order, compared row by row with a rebuild from the same snapshot, plus
//! the FTS5 `integrity-check` of both full-text tables. Each table's
//! canonical digest is SHA-256 over its rows' typed values. Both sides are
//! streamed: two ordered cursors, one row each at a time (#58).
use super::schema::CANONICAL;
use super::{Snapshot, build_file, hex, inputs, listed, replace};
use crate::error::Result;
use crate::workspace::{Repo, Workspace};
use rusqlite::Connection;
use rusqlite::types::ValueRef;
use sha2::{Digest, Sha256};
use std::path::Path;
use writfold_core::publish::{Publish, Repair, repair};
use writfold_runtime::git::GitExecutable;

/// One row: its key (the first columns, as text) and its encoded values.
struct Line {
    key: String,
    bytes: Vec<u8>,
}

/// One derived table in both indexes.
#[derive(Clone, Debug)]
pub struct Table {
    pub name: &'static str,
    pub rows: usize,
    pub digest: String,
    pub rebuilt_digest: String,
    /// The key of the first row that differs, if any.
    pub difference: Option<String>,
}

/// The comparison of an index with a rebuild.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub tables: Vec<Table>,
    /// FTS5 integrity-check failures, as `<index>.<table>: <error>`.
    pub integrity: Vec<String>,
    /// The rebuild replaced the index.
    pub replaced: bool,
    /// The KB or the live index moved during the comparison: nothing was
    /// replaced, and the comparison says nothing about the index.
    pub moved: bool,
}

impl Report {
    /// `<table> <key>` of the first difference, or an integrity failure.
    pub fn first_difference(&self) -> Option<String> {
        self.tables
            .iter()
            .find_map(|t| t.difference.as_ref().map(|key| format!("{} {key}", t.name)))
            .or_else(|| self.integrity.first().cloned())
    }
}

fn encode(value: ValueRef<'_>, out: &mut Vec<u8>) {
    let (tag, bytes): (u8, Vec<u8>) = match value {
        ValueRef::Null => (0, Vec::new()),
        ValueRef::Integer(n) => (1, n.to_be_bytes().to_vec()),
        ValueRef::Real(x) => (2, x.to_bits().to_be_bytes().to_vec()),
        ValueRef::Text(t) => (3, t.to_vec()),
        ValueRef::Blob(b) => (4, b.to_vec()),
    };
    out.push(tag);
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(&bytes);
}

fn text(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "null".into(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(x) => x.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => hex(b),
    }
}

/// One row of an ordered select, encoded; `None` past the last.
fn line(row: Option<&rusqlite::Row<'_>>, columns: usize, keys: usize) -> Result<Option<Line>> {
    let Some(row) = row else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    let mut key = Vec::new();
    for index in 0..columns {
        let value = row.get_ref(index)?;
        encode(value, &mut bytes);
        if index < keys {
            key.push(text(value));
        }
    }
    Ok(Some(Line {
        key: key.join(" "),
        bytes,
    }))
}

/// One table in both indexes, two ordered cursors stepped together and
/// hashed as they go: one row of each in memory at a time. The first
/// difference is positional, as before: the smaller key of the first
/// differing pair, else the first row past the shorter side.
fn table(
    index: &Connection,
    rebuilt: &Connection,
    (name, select, keys): (&'static str, &str, usize),
) -> Result<Table> {
    let (mut ours, mut theirs) = (index.prepare(select)?, rebuilt.prepare(select)?);
    let columns = ours.column_count();
    let (mut a, mut b) = (ours.query([])?, theirs.query([])?);
    let (mut hash_a, mut hash_b) = (Sha256::new(), Sha256::new());
    let (mut rows, mut difference) = (0usize, None);
    loop {
        let left = line(a.next()?, columns, keys)?;
        let right = line(b.next()?, columns, keys)?;
        let first = match (&left, &right) {
            (None, None) => break,
            (Some(x), Some(y)) if x.bytes != y.bytes => Some(std::cmp::min(&x.key, &y.key).clone()),
            (Some(_), Some(_)) => None,
            (Some(x), None) => Some(x.key.clone()),
            (None, Some(y)) => Some(y.key.clone()),
        };
        if let Some(x) = &left {
            rows += 1;
            hash_a.update(&x.bytes);
        }
        if let Some(y) = &right {
            hash_b.update(&y.bytes);
        }
        difference = difference.or(first);
    }
    Ok(Table {
        name,
        rows,
        digest: hex(&hash_a.finalize()),
        rebuilt_digest: hex(&hash_b.finalize()),
        difference,
    })
}

fn integrity(conn: &Connection, label: &str, out: &mut Vec<String>) {
    for table in ["fts", "tri"] {
        let check = format!("INSERT INTO {table}({table}) VALUES ('integrity-check')");
        if let Err(error) = conn.execute_batch(&check) {
            out.push(format!("{label}.{table}: {error}"));
        }
    }
}

/// Compare the derived tables of `index` with those of `rebuilt`.
pub fn compare(index: &Connection, rebuilt: &Connection) -> Result<Report> {
    let mut report = Report::default();
    for spec in CANONICAL {
        report.tables.push(table(index, rebuilt, spec)?);
    }
    integrity(index, "index", &mut report.integrity);
    integrity(rebuilt, "rebuild", &mut report.integrity);
    Ok(report)
}

/// The live index's generation: its pin and SQLite's data version, which
/// changes when another connection commits.
fn generation(conn: &Connection) -> Result<(String, i64)> {
    let pin = conn.query_row("SELECT value FROM meta WHERE key = 'pin'", [], |r| r.get(0))?;
    let version = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
    Ok((pin, version))
}

/// Rebuild the snapshot into a temporary file beside `file` and compare it
/// with the live index, inside one read transaction, so every live table
/// is one generation; the live index must be at the snapshot's pin. On a
/// mismatch the rebuild replaces the index only as
/// [`writfold_core::publish::repair`] decides: the KB still at the pin and
/// the live index still the generation compared, confirmed under the write
/// lock (spec/KbRepair.tla). `after_build` runs once the shadow is built
/// and compared, before anything is re-checked (tests interleave a second
/// writer there: an edit to the KB, or a commit to the live index, which
/// the comparison's read transaction kept out until then).
pub fn against_rebuild(
    ws: &Workspace,
    repo: &Repo,
    git: &GitExecutable<'_>,
    file: &Path,
    snapshot: &Snapshot,
    after_build: &dyn Fn(),
) -> Result<Report> {
    let temporary = file.with_file_name(format!(".kb-verify-{}.tmp", std::process::id()));
    super::sweep(file);
    // Its own connection: the comparison's read transaction and the
    // repair's write lock end with it, whatever the caller's connection
    // holds; SQLite's data version is per connection, which is what the
    // generation check needs.
    let index = &super::connect(file)?;
    index.execute_batch("BEGIN DEFERRED")?;
    let compared = (|| -> Result<Option<(Report, (String, i64))>> {
        let live = generation(index)?;
        if live.0 != snapshot.pin.digest {
            return Ok(None);
        }
        build_file(&temporary, inputs(&ws.root, listed(repo, git)?), snapshot)?;
        Ok(Some((
            compare(index, &Connection::open(&temporary)?)?,
            live,
        )))
    })();
    index.execute_batch("COMMIT")?;
    after_build();
    let Some((mut report, live)) = compared? else {
        return Ok(Report {
            moved: true,
            ..Report::default()
        });
    };
    let mismatch = report.first_difference().is_some();
    let source_moved = super::observe(ws, repo, git)?.pin.digest != snapshot.pin.digest;
    let mut locked = false;
    let live_moved = match mismatch {
        false => false,
        true => {
            index.execute_batch("BEGIN IMMEDIATE")?;
            locked = true;
            generation(index)? != live
        }
    };
    let found = Repair {
        mismatch,
        source_moved,
        live_moved,
    };
    let done = match repair(&found) {
        Publish::Replace => replace(file, &temporary).map(|()| report.replaced = true),
        Publish::Keep => std::fs::remove_file(&temporary).map_err(Into::into),
        Publish::Retry => {
            report.moved = true;
            std::fs::remove_file(&temporary).map_err(Into::into)
        }
    };
    if locked {
        index.execute_batch("ROLLBACK")?;
    }
    done?;
    Ok(report)
}
