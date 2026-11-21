//! Phase B: records, aliases, duplicates and edges, from the Phase A rows.
//! Because it reads only Phase A, an incremental refresh that replaces the
//! changed files' Phase A rows and reruns it yields exactly what a rebuild
//! yields. The keys (records, aliases, duplicates) are always recomputed;
//! the edges are rewritten as [`writfold_core::graph::plan`] decides: not at
//! all, only for the files whose references changed, or in full (#59).
use super::schema::{PHASE_B_EDGES, PHASE_B_EDGES_OF, PHASE_B_RECORDS};
use crate::layout::{first_segment, project_name};
use rusqlite::Transaction;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Phase B in full: the keys, then every edge.
pub fn phase_b(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    keys(tx)?;
    whole(tx).map(|_| ())
}

/// Records, aliases, duplicates and synthesized projects: what every
/// reference resolves against.
pub fn keys(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(PHASE_B_RECORDS)?;
    synthesize_projects(tx)
}

/// Every edge resolved again: the rows deleted and written.
pub fn whole(tx: &Transaction<'_>) -> rusqlite::Result<(usize, usize)> {
    let deleted = tx.execute("DELETE FROM edges", [])?;
    let mut written = 0;
    for sql in PHASE_B_EDGES {
        written += tx.execute(sql, [])?;
    }
    Ok((deleted, written))
}

/// The edges of `paths` only, replaced (#59): the rows deleted and
/// written. Every other file's edges are untouched.
pub fn of(tx: &Transaction<'_>, paths: &[String]) -> rusqlite::Result<(usize, usize)> {
    tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS affected(path TEXT PRIMARY KEY); DELETE FROM temp.affected;")?;
    for path in paths {
        tx.prepare_cached("INSERT OR IGNORE INTO temp.affected VALUES (?1)")?
            .execute([path])?;
    }
    let deleted = tx.execute(
        "DELETE FROM edges WHERE prov_path IN (SELECT path FROM temp.affected)",
        [],
    )?;
    let mut written = 0;
    for sql in PHASE_B_EDGES_OF {
        written += tx.execute(sql, [])?;
    }
    tx.execute_batch("DELETE FROM temp.affected;")?;
    Ok((deleted, written))
}

fn digest_of(
    tx: &Transaction<'_>,
    sqls: &[&str],
    param: Option<&str>,
) -> rusqlite::Result<Vec<u8>> {
    let mut hash = Sha256::new();
    for sql in sqls {
        let mut statement = tx.prepare_cached(sql)?;
        let columns = statement.column_count();
        let mut rows = match param {
            Some(p) => statement.query([p])?,
            None => statement.query([])?,
        };
        while let Some(row) = rows.next()? {
            for index in 0..columns {
                let value: Option<String> =
                    row.get::<_, rusqlite::types::Value>(index)
                        .map(|v| match v {
                            rusqlite::types::Value::Null => None,
                            rusqlite::types::Value::Integer(n) => Some(n.to_string()),
                            rusqlite::types::Value::Text(s) => Some(s),
                            other => Some(format!("{other:?}")),
                        })?;
                hash.update(value.as_deref().map_or(b"\x00N".as_slice(), str::as_bytes));
                hash.update(b"\x1f");
            }
            hash.update(b"\x1e");
        }
        hash.update(b"\x1d");
    }
    Ok(hash.finalize().to_vec())
}

/// A digest of everything a reference resolves against: record IDs and
/// paths, every alias pair, and the files that lost a duplicate ID.
pub fn keys_digest(tx: &Transaction<'_>) -> rusqlite::Result<Vec<u8>> {
    let sqls = [
        "SELECT id, path FROM records ORDER BY id",
        "SELECT alias, record_id FROM aliases ORDER BY alias, record_id",
        "SELECT path FROM dups ORDER BY path",
    ];
    digest_of(tx, &sqls, None)
}

/// A digest of one file's references, in order.
pub fn refs_digest(tx: &Transaction<'_>, path: &str) -> rusqlite::Result<Vec<u8>> {
    let sql = "SELECT line, ord, src, rel, token, origin, status, cite_ord FROM refs WHERE path = ?1 ORDER BY ord";
    digest_of(tx, &[sql], Some(path))
}

/// A project directory without `STATUS.md` still names a project: its
/// record sits at the README (else the first file in path order) and takes
/// the README's first heading as its title.
fn synthesize_projects(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    let mut seen: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    {
        let mut statement = tx.prepare_cached("SELECT path, heading FROM files ORDER BY path")?;
        let rows = statement.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        for row in rows {
            let (path, heading) = row?;
            let Some(name) = first_segment(&path).and_then(project_name) else {
                continue;
            };
            let readme = path == format!("{name}/README.md");
            let entry = seen
                .entry(name.to_owned())
                .or_insert_with(|| (path.clone(), None));
            if readme {
                *entry = (path, heading);
            }
        }
    }
    for (name, (path, title)) in &seen {
        let id = format!("project:{name}");
        let inserted = tx
            .prepare_cached(
                "INSERT OR IGNORE INTO records VALUES (?1, 'project', ?2, 1, ?3, NULL, ?4, NULL, NULL, NULL, '{}')",
            )?
            .execute((&id, &path, &title, &name))?;
        if inserted == 1 {
            tx.prepare_cached("INSERT OR IGNORE INTO aliases VALUES (?1, ?2, 'id')")?
                .execute((&name, &id))?;
        } else if let Some(title) = title {
            tx.prepare_cached("UPDATE records SET title = ?2 WHERE id = ?1 AND title IS NULL")?
                .execute((&id, &title))?;
        }
    }
    Ok(())
}
