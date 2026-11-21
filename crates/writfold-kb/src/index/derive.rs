//! Phase B: records, aliases, duplicates and edges, from the Phase A rows.
//! Because it reads only Phase A, an incremental refresh that replaces the
//! changed files' Phase A rows and reruns it yields exactly what a rebuild
//! yields. The keys (records, aliases, duplicates) are always recomputed;
//! the edges are rewritten as [`writfold_core::graph::plan`] decides: not at
//! all, or only for the affected files (#59, #65).
use super::schema::{PHASE_B_EDGES, PHASE_B_EDGES_OF, PHASE_B_RECORDS};
use crate::layout::{first_segment, project_name};
use rusqlite::Transaction;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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

/// How every token resolved before the keys are recomputed: record IDs
/// and paths, the unique aliases, and the files that lost a duplicate ID,
/// kept in temporary tables for [`moved`] (#65).
pub fn keys_before(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "DROP TABLE IF EXISTS temp.old_ids; DROP TABLE IF EXISTS temp.old_alias;
         DROP TABLE IF EXISTS temp.old_dups;
         CREATE TEMP TABLE old_ids AS SELECT id, path FROM main.records;
         CREATE TEMP TABLE old_alias AS SELECT alias, min(record_id) AS id FROM main.aliases
           GROUP BY alias HAVING COUNT(DISTINCT record_id) = 1;
         CREATE TEMP TABLE old_dups AS SELECT path FROM main.dups;",
    )
}

/// After the keys are recomputed: the files that gained or lost a
/// duplicate ID, and the files holding a token that now resolves
/// differently (an ID added or removed, a record's path moved, an alias
/// that became, stopped being or changed its unique record), found through
/// the `refs(token)` index. Every other file's edges are unchanged.
pub fn moved(tx: &Transaction<'_>) -> rusqlite::Result<(BTreeSet<String>, BTreeSet<String>)> {
    const NEW_ALIAS: &str = "SELECT alias, min(record_id) AS id FROM main.aliases GROUP BY alias HAVING COUNT(DISTINCT record_id) = 1";
    tx.execute_batch(&format!(
        "DROP TABLE IF EXISTS temp.moved_tokens;
         CREATE TEMP TABLE moved_tokens(token TEXT PRIMARY KEY);
         INSERT OR IGNORE INTO moved_tokens SELECT id FROM (SELECT id FROM temp.old_ids EXCEPT SELECT id FROM main.records);
         INSERT OR IGNORE INTO moved_tokens SELECT id FROM (SELECT id FROM main.records EXCEPT SELECT id FROM temp.old_ids);
         INSERT OR IGNORE INTO moved_tokens SELECT 'path:' || path FROM (SELECT id, path FROM temp.old_ids EXCEPT SELECT id, path FROM main.records);
         INSERT OR IGNORE INTO moved_tokens SELECT 'path:' || path FROM (SELECT id, path FROM main.records EXCEPT SELECT id, path FROM temp.old_ids);
         INSERT OR IGNORE INTO moved_tokens SELECT alias FROM (SELECT alias, id FROM temp.old_alias EXCEPT {NEW_ALIAS});
         INSERT OR IGNORE INTO moved_tokens SELECT alias FROM ({NEW_ALIAS} EXCEPT SELECT alias, id FROM temp.old_alias);"
    ))?;
    let paths = |sql: &str| -> rusqlite::Result<BTreeSet<String>> {
        let mut statement = tx.prepare(sql)?;
        let found = statement
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<BTreeSet<_>>>()?;
        Ok(found)
    };
    let dups = paths(
        "SELECT path FROM (SELECT path FROM temp.old_dups EXCEPT SELECT path FROM main.dups)
         UNION SELECT path FROM (SELECT path FROM main.dups EXCEPT SELECT path FROM temp.old_dups)",
    )?;
    let tokens = paths(
        "SELECT DISTINCT r.path FROM temp.moved_tokens m JOIN main.refs r ON r.token = m.token",
    )?;
    tx.execute_batch(
        "DROP TABLE temp.moved_tokens; DROP TABLE temp.old_ids; DROP TABLE temp.old_alias;
         DROP TABLE temp.old_dups;",
    )?;
    Ok((dups, tokens))
}

/// A digest of what one file gives the keys: whether it is an input and
/// its heading (a synthesized project's record), its claim and its
/// aliases. The keys are a function of these rows over every file, so when
/// no re-derived file's digest moved, the keys are unchanged (#65).
pub fn key_inputs_digest(tx: &Transaction<'_>, path: &str) -> rusqlite::Result<Vec<u8>> {
    let sqls = [
        "SELECT path, heading FROM files WHERE path = ?1",
        "SELECT * FROM claims WHERE path = ?1",
        "SELECT * FROM claim_aliases WHERE path = ?1 ORDER BY alias, source",
    ];
    digest_of(tx, &sqls, Some(path))
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
