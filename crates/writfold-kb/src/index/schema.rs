//! The derived index schema. No derived table holds a clock value or an
//! auto-assigned row ID, so equal inputs give equal rows.
//!
//! Phase A tables (`files`, `claims`, `claim_aliases`, `sections`,
//! `citations`, `refs`, `fts`, `tri`) are a pure function of one file and are
//! replaced per file on refresh. Phase B tables (`records`, `aliases`,
//! `dups`, `edges`) are a pure function of all Phase A rows and are
//! recomputed in full whenever any file changed. `meta` and `cite_checks`
//! are outside the equality claim, and so is `text_rows`, an access path
//! that records the full-text row of each file so a refresh deletes it by
//! row ID (an FTS5 delete by an unindexed column scans the whole table).

/// Bumped when the schema changes; a different stored value rebuilds.
pub const SCHEMA: &str = "3";
/// Bumped when derivation changes; a different stored value rebuilds.
pub const EXTRACTOR: &str = "writfold-kb-extract/2";

pub const DDL: &str = "
PRAGMA user_version = 3;
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files(path TEXT PRIMARY KEY, sha256 BLOB NOT NULL, size INTEGER NOT NULL,
                   kind TEXT NOT NULL, parse_status TEXT NOT NULL, parse_error TEXT,
                   heading TEXT);
CREATE TABLE claims(path TEXT PRIMARY KEY, id TEXT NOT NULL, kind TEXT NOT NULL,
                    line INTEGER NOT NULL, title TEXT, status TEXT, project TEXT,
                    created TEXT, updated TEXT, summary TEXT, extra TEXT NOT NULL);
CREATE INDEX claims_id ON claims(id, path);
CREATE TABLE claim_aliases(path TEXT NOT NULL, alias TEXT NOT NULL, source TEXT NOT NULL,
                           PRIMARY KEY(path, alias, source));
CREATE TABLE records(id TEXT PRIMARY KEY, kind TEXT NOT NULL, path TEXT NOT NULL,
                     line INTEGER NOT NULL, title TEXT, status TEXT, project TEXT,
                     created TEXT, updated TEXT, summary TEXT, extra TEXT NOT NULL);
CREATE INDEX records_path ON records(path);
CREATE TABLE aliases(alias TEXT NOT NULL, record_id TEXT NOT NULL, source TEXT NOT NULL,
                     PRIMARY KEY(alias, record_id, source)) WITHOUT ROWID;
CREATE TABLE dups(path TEXT PRIMARY KEY, id TEXT NOT NULL);
CREATE TABLE sections(path TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
                      line_first INTEGER NOT NULL, line_last INTEGER NOT NULL,
                      PRIMARY KEY(path, ord));
CREATE TABLE citations(path TEXT NOT NULL, ord INTEGER NOT NULL, line INTEGER NOT NULL,
                       kind TEXT, repo TEXT, rev TEXT, cpath TEXT, l_first INTEGER, l_last INTEGER,
                       blob TEXT, span TEXT, seen TEXT, ttl TEXT, state TEXT, marked TEXT,
                       illustrative INTEGER NOT NULL, complete INTEGER NOT NULL,
                       malformed TEXT, fields TEXT NOT NULL, raw TEXT NOT NULL,
                       PRIMARY KEY(path, ord));
CREATE TABLE refs(path TEXT NOT NULL, line INTEGER NOT NULL, ord INTEGER NOT NULL,
                  src TEXT NOT NULL, rel TEXT NOT NULL, token TEXT NOT NULL,
                  origin TEXT NOT NULL, status TEXT NOT NULL, cite_ord INTEGER,
                  PRIMARY KEY(path, ord));
CREATE TABLE edges(src TEXT NOT NULL, rel TEXT NOT NULL, dst TEXT NOT NULL,
                   origin TEXT NOT NULL, status TEXT NOT NULL,
                   prov_path TEXT NOT NULL, prov_line INTEGER NOT NULL, cite_ord INTEGER,
                   PRIMARY KEY(src, rel, dst, origin, prov_path, prov_line)) WITHOUT ROWID;
CREATE INDEX edges_dst ON edges(dst, rel);
CREATE VIRTUAL TABLE fts USING fts5(id, title, summary, body, path UNINDEXED,
                                    tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE tri USING fts5(id, title, body, path UNINDEXED,
                                    tokenize = 'trigram case_sensitive 0', detail = none);
CREATE TABLE text_rows(path TEXT PRIMARY KEY, row INTEGER NOT NULL);
CREATE TABLE cite_checks(path TEXT NOT NULL, ord INTEGER NOT NULL, repo_head TEXT NOT NULL,
                         verdict TEXT NOT NULL, new_first INTEGER, new_last INTEGER,
                         new_path TEXT, detail TEXT, checked_at TEXT NOT NULL,
                         PRIMARY KEY(path, ord));
";

/// Phase A rows of one file, deleted before it is re-derived.
pub const DELETE_FILE: [&str; 9] = [
    "DELETE FROM fts WHERE rowid = (SELECT row FROM text_rows WHERE path = ?1)",
    "DELETE FROM tri WHERE rowid = (SELECT row FROM text_rows WHERE path = ?1)",
    "DELETE FROM text_rows WHERE path = ?1",
    "DELETE FROM files WHERE path = ?1",
    "DELETE FROM claims WHERE path = ?1",
    "DELETE FROM claim_aliases WHERE path = ?1",
    "DELETE FROM sections WHERE path = ?1",
    "DELETE FROM citations WHERE path = ?1",
    "DELETE FROM refs WHERE path = ?1",
];

/// Phase B, before project synthesis: each claimed ID goes to the first
/// claiming file in path order; later claimants are duplicates (counted as
/// unreadable). Aliases come from the winning claims.
pub const PHASE_B_RECORDS: &str = "
DELETE FROM records; DELETE FROM aliases; DELETE FROM dups; DELETE FROM edges;
INSERT INTO records SELECT id, kind, path, line, title, status, project, created, updated, summary, extra
  FROM claims c WHERE c.path = (SELECT min(path) FROM claims WHERE id = c.id);
INSERT INTO dups SELECT path, id FROM claims c
  WHERE c.path <> (SELECT min(path) FROM claims WHERE id = c.id);
INSERT OR IGNORE INTO aliases SELECT a.alias, c.id, a.source
  FROM claim_aliases a JOIN claims c ON c.path = a.path
  WHERE a.path NOT IN (SELECT path FROM dups);
";

/// Phase B, edges: a token resolves to a record ID exactly; else through an
/// alias that names one record only; `path:<file>` names the records of a
/// file; `code:`, `commit:` and `pr:` tokens are nodes outside the KB. A
/// mention of the file's own record is not an edge. Rows are inserted in
/// key order, so which duplicate provenance wins does not depend on the
/// order files were indexed in.
pub const PHASE_B_EDGES: &str = "
INSERT OR IGNORE INTO edges
  SELECT r.src, r.rel, rec.id, r.origin, r.status, r.path, r.line, r.cite_ord
  FROM refs r JOIN records rec ON rec.id = r.token
  WHERE r.path NOT IN (SELECT path FROM dups) AND (r.origin <> 'mention' OR r.src <> rec.id)
  ORDER BY r.path, r.ord, rec.id;
INSERT OR IGNORE INTO edges
  SELECT r.src, r.rel, a.id, r.origin, r.status, r.path, r.line, r.cite_ord
  FROM refs r
  JOIN (SELECT alias, min(record_id) AS id FROM aliases GROUP BY alias
        HAVING COUNT(DISTINCT record_id) = 1) a ON a.alias = r.token
  WHERE r.path NOT IN (SELECT path FROM dups)
    AND NOT EXISTS (SELECT 1 FROM records WHERE id = r.token)
    AND (r.origin <> 'mention' OR r.src <> a.id)
  ORDER BY r.path, r.ord;
INSERT OR IGNORE INTO edges
  SELECT r.src, r.rel, rec.id, r.origin, r.status, r.path, r.line, r.cite_ord
  FROM refs r JOIN records rec ON rec.path = substr(r.token, 6)
  WHERE r.token GLOB 'path:*' AND r.path NOT IN (SELECT path FROM dups)
  ORDER BY r.path, r.ord, rec.id;
INSERT OR IGNORE INTO edges
  SELECT r.src, r.rel, r.token, r.origin, r.status, r.path, r.line, r.cite_ord
  FROM refs r
  WHERE (r.token GLOB 'code:*' OR r.token GLOB 'commit:*' OR r.token GLOB 'pr:*')
    AND r.path NOT IN (SELECT path FROM dups)
  ORDER BY r.path, r.ord;
";

/// Every derived table in canonical order, as `(name, select, key
/// columns)`: the index equals a rebuild when these selects return equal
/// rows. The key columns name a row in a mismatch report.
pub const CANONICAL: [(&str, &str, usize); 12] = [
    ("files", "SELECT * FROM files ORDER BY path", 1),
    ("claims", "SELECT * FROM claims ORDER BY path", 1),
    (
        "claim_aliases",
        "SELECT * FROM claim_aliases ORDER BY path, alias, source",
        3,
    ),
    ("records", "SELECT * FROM records ORDER BY id", 1),
    (
        "aliases",
        "SELECT * FROM aliases ORDER BY alias, record_id, source",
        3,
    ),
    ("dups", "SELECT * FROM dups ORDER BY path", 1),
    ("sections", "SELECT * FROM sections ORDER BY path, ord", 2),
    ("citations", "SELECT * FROM citations ORDER BY path, ord", 2),
    ("refs", "SELECT * FROM refs ORDER BY path, ord", 3),
    (
        "edges",
        "SELECT * FROM edges ORDER BY src, rel, dst, origin, prov_path, prov_line",
        6,
    ),
    (
        "fts",
        "SELECT path, id, title, summary, body FROM fts ORDER BY path, id",
        2,
    ),
    (
        "tri",
        "SELECT path, id, title, body FROM tri ORDER BY path, id",
        2,
    ),
];
