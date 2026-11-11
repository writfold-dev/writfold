//! The derived index schema. No derived table holds a clock value or an
//! auto-assigned row ID, so equal inputs give equal rows.

/// Bumped when the schema changes; a different stored value rebuilds.
pub const SCHEMA: &str = "1";
/// Bumped when derivation changes; a different stored value rebuilds.
pub const EXTRACTOR: &str = "writfold-kb-extract/1";

pub const DDL: &str = "
PRAGMA user_version = 1;
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files(path TEXT PRIMARY KEY, sha256 BLOB NOT NULL, size INTEGER NOT NULL,
                   kind TEXT NOT NULL, parse_status TEXT NOT NULL, parse_error TEXT);
CREATE TABLE records(id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                     path TEXT NOT NULL REFERENCES files(path), line INTEGER NOT NULL,
                     title TEXT, status TEXT, project TEXT, created TEXT, updated TEXT,
                     summary TEXT, extra TEXT NOT NULL);
CREATE INDEX records_path ON records(path);
CREATE TABLE aliases(alias TEXT NOT NULL, record_id TEXT NOT NULL, source TEXT NOT NULL,
                     PRIMARY KEY(alias, record_id, source));
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
                   PRIMARY KEY(src, rel, dst, origin, prov_path, prov_line));
CREATE INDEX edges_dst ON edges(dst, rel);
CREATE VIRTUAL TABLE fts USING fts5(id, title, summary, body, path UNINDEXED,
                                    tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE tri USING fts5(id, title, body, path UNINDEXED,
                                    tokenize = 'trigram case_sensitive 0', detail = none);
CREATE TABLE cite_checks(path TEXT NOT NULL, ord INTEGER NOT NULL, repo_head TEXT NOT NULL,
                         verdict TEXT NOT NULL, new_first INTEGER, new_last INTEGER,
                         new_path TEXT, PRIMARY KEY(path, ord));
";

/// Phase B: every edge, recomputed in full from Phase A. A token resolves to
/// a record ID exactly, else through an alias that names one record only.
pub const PHASE_B: &str = "
DELETE FROM edges;
INSERT OR IGNORE INTO edges
  SELECT r.src, r.rel, rec.id, r.origin, r.status, r.path, r.line, r.cite_ord
  FROM refs r JOIN records rec ON rec.id = r.token;
INSERT OR IGNORE INTO edges
  SELECT r.src, r.rel, a.record_id, r.origin, r.status, r.path, r.line, r.cite_ord
  FROM refs r JOIN aliases a ON a.alias = r.token
  WHERE NOT EXISTS (SELECT 1 FROM records WHERE id = r.token)
    AND (SELECT COUNT(DISTINCT record_id) FROM aliases WHERE alias = r.token) = 1;
";
