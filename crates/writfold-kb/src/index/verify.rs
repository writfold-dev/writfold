//! The index equals a rebuild: every derived table, selected in primary-key
//! order, compared row by row with a rebuild from the same snapshot, plus
//! the FTS5 `integrity-check` of both full-text tables. Each table's
//! canonical digest is SHA-256 over its rows' typed values.
use super::schema::CANONICAL;
use super::{Snapshot, build_file, hex, inputs, listed, replace};
use crate::error::Result;
use crate::workspace::{Repo, Workspace};
use rusqlite::Connection;
use rusqlite::types::ValueRef;
use sha2::{Digest, Sha256};
use std::path::Path;
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

fn lines(conn: &Connection, select: &str, keys: usize) -> Result<Vec<Line>> {
    let mut statement = conn.prepare(select)?;
    let columns = statement.column_count();
    let mut rows = statement.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let mut bytes = Vec::new();
        let mut key = Vec::new();
        for index in 0..columns {
            let value = row.get_ref(index)?;
            encode(value, &mut bytes);
            if index < keys {
                key.push(text(value));
            }
        }
        out.push(Line {
            key: key.join(" "),
            bytes,
        });
    }
    Ok(out)
}

fn digest(lines: &[Line]) -> String {
    let mut hash = Sha256::new();
    for line in lines {
        hash.update(&line.bytes);
    }
    hex(&hash.finalize())
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
    for (name, select, keys) in CANONICAL {
        let (ours, theirs) = (lines(index, select, keys)?, lines(rebuilt, select, keys)?);
        let difference = ours
            .iter()
            .zip(&theirs)
            .find(|(a, b)| a.bytes != b.bytes)
            .map(|(a, b)| std::cmp::min(&a.key, &b.key).clone())
            .or_else(|| match ours.len().cmp(&theirs.len()) {
                std::cmp::Ordering::Greater => ours.get(theirs.len()).map(|l| l.key.clone()),
                std::cmp::Ordering::Less => theirs.get(ours.len()).map(|l| l.key.clone()),
                std::cmp::Ordering::Equal => None,
            });
        report.tables.push(Table {
            name,
            rows: ours.len(),
            digest: digest(&ours),
            rebuilt_digest: digest(&theirs),
            difference,
        });
    }
    integrity(index, "index", &mut report.integrity);
    integrity(rebuilt, "rebuild", &mut report.integrity);
    Ok(report)
}

/// Rebuild the snapshot into a temporary file beside `file` and compare;
/// on any difference the rebuild replaces the index.
pub fn against_rebuild(
    ws: &Workspace,
    repo: &Repo,
    git: &GitExecutable<'_>,
    index: &Connection,
    file: &Path,
    snapshot: &Snapshot,
) -> Result<Report> {
    let temporary = file.with_file_name(format!(".kb-verify-{}.tmp", std::process::id()));
    build_file(&temporary, inputs(&ws.root, listed(repo, git)?), snapshot)?;
    let report = compare(index, &Connection::open(&temporary)?)?;
    if report.first_difference().is_some() {
        replace(file, &temporary)?;
    } else {
        std::fs::remove_file(&temporary)?;
    }
    Ok(report)
}
