//! Full rebuild: Phase A (facts per file) and Phase B (edges), in one
//! transaction, inserted in path order.
use super::schema::{DDL, EXTRACTOR, PHASE_B, SCHEMA};
use crate::layout::{Kind, first_segment, project_name};
use crate::model::{Facts, Record};
use crate::parse;
use rusqlite::{Connection, Transaction};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// One input file: its root-relative path, kind, and bytes or the reason
/// they could not be read.
pub struct Input {
    pub path: String,
    pub kind: Kind,
    pub bytes: Result<Vec<u8>, String>,
}

#[derive(Default)]
struct Projects {
    /// Project name to (README path, its title) or the first file seen.
    seen: BTreeMap<String, (String, Option<String>)>,
}

/// Create the schema and derive every table from `inputs` (sorted by path).
pub fn build(
    conn: &mut Connection,
    inputs: Vec<Input>,
    meta: &[(&str, String)],
) -> rusqlite::Result<()> {
    conn.execute_batch(DDL)?;
    let tx = conn.transaction()?;
    let mut projects = Projects::default();
    for input in inputs {
        if let Some(name) = first_segment(&input.path).and_then(project_name) {
            note_project(&mut projects, name, &input);
        }
        insert_file(&tx, input)?;
    }
    synthesize_projects(&tx, &projects)?;
    tx.execute_batch(PHASE_B)?;
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

fn note_project(projects: &mut Projects, name: &str, input: &Input) {
    let readme = input.path == format!("{name}/README.md");
    let title = match (&input.bytes, readme) {
        (Ok(bytes), true) => std::str::from_utf8(bytes)
            .ok()
            .and_then(parse::first_heading),
        _ => None,
    };
    let entry = projects
        .seen
        .entry(name.to_owned())
        .or_insert_with(|| (input.path.clone(), None));
    if readme {
        *entry = (input.path.clone(), title);
    }
}

/// A project directory without `STATUS.md` still names a project.
fn synthesize_projects(tx: &Transaction<'_>, projects: &Projects) -> rusqlite::Result<()> {
    for (name, (path, title)) in &projects.seen {
        let id = format!("project:{name}");
        let record = Record {
            id: id.clone(),
            kind: Some(Kind::Project),
            line: 1,
            title: title.clone(),
            project: Some(name.clone()),
            extra: "{}".into(),
            ..Record::default()
        };
        if insert_record(tx, path, &record)? {
            tx.prepare_cached("INSERT OR IGNORE INTO aliases VALUES (?1, ?2, 'id')")?
                .execute((&name, &id))?;
        } else if let Some(title) = title {
            tx.prepare_cached("UPDATE records SET title = ?2 WHERE id = ?1 AND title IS NULL")?
                .execute((&id, &title))?;
        }
    }
    Ok(())
}

fn insert_file(tx: &Transaction<'_>, input: Input) -> rusqlite::Result<()> {
    let Input { path, kind, bytes } = input;
    let (digest, size, parsed) = match &bytes {
        Ok(bytes) => (
            Sha256::digest(bytes).to_vec(),
            bytes.len(),
            parse::parse(&path, kind, bytes),
        ),
        Err(reason) => (Vec::new(), 0, Err(reason.clone())),
    };
    let (status, error, facts) = match parsed {
        Ok(facts) => ("ok", None, facts),
        Err(reason) => ("unreadable", Some(reason), Facts::default()),
    };
    tx.prepare_cached("INSERT INTO files VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?
        .execute((&path, &digest, &size, &kind.as_str(), &status, &error))?;
    if status == "ok" {
        insert_facts(tx, &path, facts)?;
    }
    Ok(())
}

fn insert_facts(tx: &Transaction<'_>, path: &str, facts: Facts) -> rusqlite::Result<()> {
    let mut source = format!("doc:{path}");
    let (mut title, mut summary) = (None, None);
    if let Some(record) = &facts.record {
        if insert_record(tx, path, record)? {
            source.clone_from(&record.id);
            title.clone_from(&record.title);
            summary.clone_from(&record.summary);
            insert_links(tx, path, &facts)?;
        } else {
            let reason = format!("duplicate id {}", record.id);
            tx.prepare_cached(
                "UPDATE files SET parse_status = 'unreadable', parse_error = ?2 WHERE path = ?1",
            )?
            .execute((&path, &reason))?;
        }
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
    tx.prepare_cached("INSERT INTO tri VALUES (?1, ?2, ?3, ?4)")?
        .execute((&source, &title, &facts.body, &path))?;
    Ok(())
}

/// Insert one record; `false` when its ID is already taken.
fn insert_record(tx: &Transaction<'_>, path: &str, r: &Record) -> rusqlite::Result<bool> {
    let kind = r.kind.map_or("doc", Kind::as_str);
    let changed = tx
        .prepare_cached(
            "INSERT OR IGNORE INTO records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?
        .execute((
            &r.id, &kind, &path, &r.line, &r.title, &r.status, &r.project, &r.created, &r.updated,
            &r.summary, &r.extra,
        ))?;
    Ok(changed == 1)
}

fn insert_links(tx: &Transaction<'_>, path: &str, facts: &Facts) -> rusqlite::Result<()> {
    if let Some(record) = &facts.record {
        for (alias, source) in &facts.aliases {
            tx.prepare_cached("INSERT OR IGNORE INTO aliases VALUES (?1, ?2, ?3)")?
                .execute((&alias, &record.id, &source))?;
        }
    }
    for (ord, r) in facts.references.iter().enumerate() {
        tx.prepare_cached("INSERT INTO refs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")?
            .execute((
                &path, &r.line, &ord, &r.src, &r.rel, &r.token, &r.origin, &r.status, &r.cite,
            ))?;
    }
    Ok(())
}
