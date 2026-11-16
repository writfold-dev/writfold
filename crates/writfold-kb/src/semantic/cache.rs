//! Model-keyed derived vectors. No model work is on the lexical index path.
use super::{chunks, model::Model};
use crate::error::{KbError, Result};
use crate::index::Pin;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

const DDL: &str = "
CREATE TABLE IF NOT EXISTS semantic_vectors(
 hash BLOB PRIMARY KEY, model TEXT NOT NULL, vector BLOB NOT NULL);
CREATE TABLE IF NOT EXISTS semantic_chunks(
 path TEXT NOT NULL, first INTEGER NOT NULL, last INTEGER NOT NULL,
 id TEXT NOT NULL, hash BLOB NOT NULL, PRIMARY KEY(path, first));
";

/// Preserve content-addressed vectors across an explicit lexical rebuild.
/// Locations are always derived again, so moved/deleted records cannot leak.
pub(crate) fn carry_vectors(conn: &Connection, old: &Path) -> Result<()> {
    conn.execute(
        "ATTACH DATABASE ?1 AS semantic_old",
        [old.to_string_lossy().as_ref()],
    )?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM semantic_old.sqlite_master WHERE name = 'semantic_vectors' AND type = 'table')",
        [], |row| row.get(0),
    )?;
    if exists {
        conn.execute_batch(DDL)?;
        conn.execute_batch(
            "INSERT OR IGNORE INTO semantic_vectors SELECT * FROM semantic_old.semantic_vectors;",
        )?;
    }
    conn.execute_batch("DETACH DATABASE semantic_old;")?;
    Ok(())
}

/// Must run inside the caller's transaction, including its ensuing scan.
pub(super) fn sync(conn: &Connection, pin: &Pin, model: &Model) -> Result<usize> {
    conn.execute_batch(DDL)?;
    let actual: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'pin'", [], |r| r.get(0))
        .optional()?;
    if actual.as_ref().is_some_and(|p| p != &pin.digest) {
        return Err(KbError::Refused(
            "KB index moved before semantic search; retry the lookup".into(),
        ));
    }
    let state = format!("{}/{}", pin.digest, model.identity());
    let previous: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'semantic-state'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if previous.as_deref() == Some(&state) {
        return Ok(0);
    }
    conn.execute("DELETE FROM semantic_chunks", [])?;
    let count = populate(conn, model)?;
    conn.execute(
        "DELETE FROM semantic_vectors WHERE hash NOT IN (SELECT hash FROM semantic_chunks)",
        [],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES ('semantic-state', ?1)",
        [&state],
    )?;
    Ok(count)
}

fn populate(conn: &Connection, model: &Model) -> Result<usize> {
    let mut sources = conn.prepare(
        "SELECT coalesce(r.id, f.id), coalesce(r.title, f.title, files.heading, f.id), f.path, f.body
         FROM fts f JOIN files ON files.path = f.path LEFT JOIN records r ON r.path = f.path
         WHERE f.path NOT IN (SELECT path FROM dups) ORDER BY f.path",
    )?;
    let mut rows = sources.query([])?;
    let mut embedded = 0;
    while let Some(row) = rows.next()? {
        let id = row.get_ref(0)?.as_str().map_err(super::value_error)?;
        let title = row.get_ref(1)?.as_str().map_err(super::value_error)?;
        let path = row.get_ref(2)?.as_str().map_err(super::value_error)?;
        let body = row.get_ref(3)?.as_str().map_err(super::value_error)?;
        let sections = chunks::sections(conn, path)?;
        for chunk in chunks::split(title, body, &sections, model.identity()) {
            let present: bool = conn
                .prepare_cached(
                    "SELECT EXISTS(SELECT 1 FROM semantic_vectors WHERE hash = ?1 AND model = ?2)",
                )?
                .query_row((&chunk.hash, model.identity()), |r| r.get(0))?;
            if !present {
                let vector = model.embed(&chunk.text)?;
                let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
                conn.prepare_cached("INSERT INTO semantic_vectors VALUES (?1, ?2, ?3)")?
                    .execute((&chunk.hash, model.identity(), blob))?;
                embedded += 1;
            }
            conn.prepare_cached("INSERT INTO semantic_chunks VALUES (?1, ?2, ?3, ?4, ?5)")?
                .execute((path, chunk.first, chunk.last, id, &chunk.hash))?;
        }
    }
    Ok(embedded)
}
