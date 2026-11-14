//! Typed reads over the index shared by the verbs.
use crate::error::Result;
use crate::parse::{list, ticket};
use rusqlite::{Connection, OptionalExtension, Row, ToSql};
use serde_json::Value;

/// One record row.
#[derive(Clone, Debug, Default)]
pub struct Rec {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub line: u32,
    pub title: Option<String>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub summary: Option<String>,
    pub extra: String,
}

impl Rec {
    /// One authored field, verbatim, if present and non-empty.
    pub fn field(&self, key: &str) -> Option<String> {
        let value: Value = serde_json::from_str(&self.extra).ok()?;
        match value.get(key)? {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
            Value::Array(items) => {
                let parts: Vec<String> = items
                    .iter()
                    .filter_map(|i| i.as_str().map(str::to_owned))
                    .collect();
                (!parts.is_empty()).then(|| format!("[{}]", parts.join(", ")))
            }
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    /// A list field (`[a, b]` in front matter or a JSON array).
    pub fn list(&self, key: &str) -> Vec<String> {
        self.field(key).map(|v| list(&v)).unwrap_or_default()
    }

    /// `title`, else the ID.
    pub fn label(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.id.clone())
    }
}

pub const COLUMNS: &str =
    "id, kind, path, line, title, status, project, created, updated, summary, extra";

pub fn rec(row: &Row<'_>) -> rusqlite::Result<Rec> {
    Ok(Rec {
        id: row.get(0)?,
        kind: row.get(1)?,
        path: row.get(2)?,
        line: row.get(3)?,
        title: row.get(4)?,
        status: row.get(5)?,
        project: row.get(6)?,
        created: row.get(7)?,
        updated: row.get(8)?,
        summary: row.get(9)?,
        extra: row.get(10)?,
    })
}

pub fn record(conn: &Connection, id: &str) -> Result<Option<Rec>> {
    let sql = format!("SELECT {COLUMNS} FROM records WHERE id = ?1");
    Ok(conn.query_row(&sql, [id], rec).optional()?)
}

/// Records matching a `WHERE` clause, in the given order.
pub fn records(conn: &Connection, clause: &str, params: &[&dyn ToSql]) -> Result<Vec<Rec>> {
    let sql = format!("SELECT {COLUMNS} FROM records WHERE {clause}");
    let mut statement = conn.prepare_cached(&sql)?;
    let found = statement
        .query_map(params, rec)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}

/// How a user's target named records.
pub enum Resolved {
    One(Box<Rec>),
    Many(Vec<Rec>),
    None,
}

const PREFIXES: [&str; 9] = [
    "proposal/",
    "project:",
    "sig:",
    "inv:",
    "ev:",
    "dossier:",
    "runbook:",
    "correction:",
    "rel:",
];

/// Exact ID first; then the normalised ticket, typed prefixes and aliases;
/// then an asset whose locator contains the text.
pub fn resolve(conn: &Connection, input: &str) -> Result<Resolved> {
    if let Some(found) = record(conn, input)? {
        return Ok(Resolved::One(Box::new(found)));
    }
    let mut ids: Vec<String> = ticket(input).into_iter().collect();
    ids.extend(PREFIXES.iter().map(|p| format!("{p}{input}")));
    let mut found: Vec<Rec> = Vec::new();
    for id in ids {
        found.extend(record(conn, &id)?);
    }
    let aliased = records(
        conn,
        "id IN (SELECT record_id FROM aliases WHERE alias = ?1) ORDER BY id",
        &[&input],
    )?;
    found.extend(aliased);
    if found.is_empty() {
        let lowered = input.to_lowercase();
        found = records(
            conn,
            "kind = 'asset' AND instr(lower(json_extract(extra, '$.locator')), ?1) > 0 ORDER BY id",
            &[&lowered],
        )?;
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found.dedup_by(|a, b| a.id == b.id);
    Ok(match found.len() {
        0 => Resolved::None,
        1 => found
            .pop()
            .map_or(Resolved::None, |r| Resolved::One(Box::new(r))),
        _ => Resolved::Many(found),
    })
}

/// The full-text match expression for free text: its words, OR-ed.
pub fn match_expression(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(|w| format!("\"{w}\""))
        .collect();
    (!words.is_empty()).then(|| words.join(" OR "))
}

/// Up to `limit` full-text hits (ID, title, path), best first, optionally
/// only records of the given kinds.
pub fn nearest(
    conn: &Connection,
    text: &str,
    limit: usize,
    kinds: &[&str],
) -> Result<Vec<(String, Option<String>, String)>> {
    let Some(expression) = match_expression(text) else {
        return Ok(Vec::new());
    };
    let filter = if kinds.is_empty() {
        String::new()
    } else {
        let quoted: Vec<String> = kinds.iter().map(|k| format!("'{k}'")).collect();
        format!(
            " AND id IN (SELECT id FROM records WHERE kind IN ({}))",
            quoted.join(",")
        )
    };
    let sql = format!(
        "SELECT id, title, path FROM fts WHERE fts MATCH ?1{filter} \
         ORDER BY bm25(fts, 10.0, 5.0, 2.0, 1.0), id LIMIT ?2"
    );
    let mut statement = conn.prepare(&sql)?;
    let found = statement
        .query_map((&expression, &limit), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}

/// One resolved edge with its provenance.
#[derive(Clone, Debug)]
pub struct Edge {
    pub src: String,
    pub rel: String,
    pub dst: String,
    pub origin: String,
    pub status: String,
    pub path: String,
    pub line: u32,
    /// The provenance citation's ordinal in `path`, for a `cite` edge.
    pub cite: Option<u32>,
}

/// Edges leaving (`outgoing`) or reaching `id`, in key order.
pub fn edges(conn: &Connection, id: &str, outgoing: bool) -> Result<Vec<Edge>> {
    let column = if outgoing { "src" } else { "dst" };
    let sql = format!(
        "SELECT src, rel, dst, origin, status, prov_path, prov_line, cite_ord FROM edges WHERE {column} = ?1 \
         ORDER BY src, rel, dst, origin, prov_path, prov_line"
    );
    let mut statement = conn.prepare_cached(&sql)?;
    let found = statement
        .query_map([id], |r| {
            Ok(Edge {
                src: r.get(0)?,
                rel: r.get(1)?,
                dst: r.get(2)?,
                origin: r.get(3)?,
                status: r.get(4)?,
                path: r.get(5)?,
                line: r.get(6)?,
                cite: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}

/// Destinations mentioned by files inside one run's directory.
pub fn run_destinations(conn: &Connection, id: &str) -> Result<Vec<String>> {
    let prefix = format!("doc:runs/{id}/");
    // '/' and '0' are adjacent in the source key's binary ordering.
    let end = format!("doc:runs/{id}0");
    let mut statement = conn.prepare_cached(
        "SELECT DISTINCT dst FROM edges WHERE src >= ?1 AND src < ?2 ORDER BY dst",
    )?;
    let found = statement
        .query_map((&prefix, &end), |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}

/// Readable input files, all input files, unreadable files (a file whose
/// record ID an earlier file already claims is unreadable).
pub fn scope(conn: &Connection) -> Result<(usize, usize, usize)> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FILTER (WHERE parse_status = 'ok' AND path NOT IN (SELECT path FROM dups)), \
         COUNT(*), \
         COUNT(*) FILTER (WHERE parse_status <> 'ok' OR path IN (SELECT path FROM dups)) FROM files",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?)
}
