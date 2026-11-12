//! Phase A (facts per file) and the full build: every file's facts, then
//! Phase B, in one transaction, inserted in path order.
use super::derive;
use super::schema::{DDL, DELETE_FILE, EXTRACTOR, SCHEMA};
use crate::layout::Kind;
use crate::model::{Facts, Record};
use crate::parse;
use rusqlite::{Connection, Transaction};
use sha2::{Digest, Sha256};

/// One input file: its root-relative path, kind, and bytes or the reason
/// they could not be read.
pub struct Input {
    pub path: String,
    pub kind: Kind,
    pub bytes: Result<Vec<u8>, String>,
}

/// Create the schema and derive every table from `inputs` (sorted by path).
pub fn build(
    conn: &mut Connection,
    inputs: Vec<Input>,
    meta: &[(&str, String)],
) -> rusqlite::Result<()> {
    conn.execute_batch(DDL)?;
    let tx = conn.transaction()?;
    for input in inputs {
        insert_file(&tx, input)?;
    }
    derive::phase_b(&tx)?;
    let mut statement = tx.prepare("INSERT INTO meta(key, value) VALUES (?1, ?2)")?;
    for (key, value) in [("schema", SCHEMA), ("extractor", EXTRACTOR)] {
        statement.execute((&key, &value))?;
    }
    for (key, value) in meta {
        statement.execute((&key, &value))?;
    }
    drop(statement);
    tx.commit()
}

/// Remove every Phase A row of `path`.
pub fn delete_file(tx: &Transaction<'_>, path: &str) -> rusqlite::Result<()> {
    for sql in DELETE_FILE {
        tx.prepare_cached(sql)?.execute([path])?;
    }
    Ok(())
}

/// The content digest stored for a file (empty when it could not be read).
pub fn digest(bytes: &Result<Vec<u8>, String>) -> Vec<u8> {
    bytes
        .as_ref()
        .map(|b| Sha256::digest(b).to_vec())
        .unwrap_or_default()
}

/// Derive and insert every Phase A row of one file.
pub fn insert_file(tx: &Transaction<'_>, input: Input) -> rusqlite::Result<()> {
    let Input { path, kind, bytes } = input;
    let digest = digest(&bytes);
    let (size, parsed) = match &bytes {
        Ok(bytes) => (bytes.len(), parse::parse(&path, kind, bytes)),
        Err(reason) => (0, Err(reason.clone())),
    };
    let (status, error, facts) = match parsed {
        Ok(facts) => ("ok", None, facts),
        Err(reason) => ("unreadable", Some(reason), Facts::default()),
    };
    // A README's title names its project even when the file is a plain
    // document (or could not be parsed as one).
    let heading = facts.heading.clone().or_else(|| match &bytes {
        Ok(bytes) if path.ends_with(".md") => std::str::from_utf8(bytes)
            .ok()
            .and_then(parse::first_heading),
        _ => None,
    });
    tx.prepare_cached("INSERT INTO files VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?
        .execute((
            &path,
            &digest,
            &size,
            &kind.as_str(),
            &status,
            &error,
            &heading,
        ))?;
    if status == "ok" {
        insert_facts(tx, &path, facts)?;
    }
    Ok(())
}

fn insert_facts(tx: &Transaction<'_>, path: &str, facts: Facts) -> rusqlite::Result<()> {
    let mut source = format!("doc:{path}");
    let (mut title, mut summary) = (None, None);
    if let Some(record) = &facts.record {
        insert_claim(tx, path, record)?;
        source.clone_from(&record.id);
        title.clone_from(&record.title);
        summary.clone_from(&record.summary);
        for (alias, origin) in &facts.aliases {
            tx.prepare_cached("INSERT OR IGNORE INTO claim_aliases VALUES (?1, ?2, ?3)")?
                .execute((&path, &alias, &origin))?;
        }
    }
    for (ord, r) in facts.references.iter().enumerate() {
        tx.prepare_cached("INSERT INTO refs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")?
            .execute((
                &path, &r.line, &ord, &r.src, &r.rel, &r.token, &r.origin, &r.status, &r.cite,
            ))?;
    }
    for (ord, section) in facts.sections.iter().enumerate() {
        tx.prepare_cached("INSERT INTO sections VALUES (?1, ?2, ?3, ?4, ?5)")?
            .execute((&path, &ord, &section.name, &section.first, &section.last))?;
    }
    for (ord, c) in facts.citations.iter().enumerate() {
        tx.prepare_cached(
            "INSERT INTO citations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        )?
        .execute(&[&path as &dyn rusqlite::ToSql, &ord, &c.line, &c.kind, &c.repo, &c.rev, &c.path, &c.first, &c.last, &c.blob, &c.span, &c.seen, &c.ttl, &c.state, &c.marked, &c.illustrative, &c.complete, &c.malformed, &c.fields, &c.raw][..])?;
    }
    tx.prepare_cached("INSERT INTO fts VALUES (?1, ?2, ?3, ?4, ?5)")?
        .execute((&source, &title, &summary, &facts.body, &path))?;
    let row = tx.last_insert_rowid();
    tx.prepare_cached("INSERT INTO tri(rowid, id, title, body, path) VALUES (?1, ?2, ?3, ?4, ?5)")?
        .execute((&row, &source, &title, &facts.body, &path))?;
    tx.prepare_cached("INSERT INTO text_rows VALUES (?1, ?2)")?
        .execute((&path, &row))?;
    Ok(())
}

/// The record this file claims; Phase B decides whether it wins its ID.
fn insert_claim(tx: &Transaction<'_>, path: &str, r: &Record) -> rusqlite::Result<()> {
    let kind = r.kind.map_or("doc", Kind::as_str);
    tx.prepare_cached("INSERT INTO claims VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)")?
        .execute((
            &path, &r.id, &kind, &r.line, &r.title, &r.status, &r.project, &r.created, &r.updated,
            &r.summary, &r.extra,
        ))?;
    Ok(())
}
