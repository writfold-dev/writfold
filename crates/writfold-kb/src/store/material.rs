//! Backlinks, the reading half (stores-next spec S3): what each declared
//! store's directories claim to serve. In every store that resolves
//! ([`super::resolve`], proved), every `README.md` in the tree of the
//! store's target commit (`origin/HEAD`, else `HEAD`, as `cite make` pins)
//! is read for a `subject:` front-matter field, one ID or a list. Whether a
//! claim lists the directory under a record is
//! [`writfold_core::store::claim`] (proved), applied in
//! [`crate::verbs::material`].
//!
//! Reads are cached in the index file beside `cite_checks`, outside the
//! rebuild equality claim and the index pin, keyed by the commit read:
//! `store_material` holds one row per claim (a README with no claim keeps
//! one row with no subject, so it is not read again), `store_material_at`
//! the commit and whether the read was complete. Rows are reused only for
//! the commit just read, and by blob ID across commits. A cache that cannot
//! be written (the index is busy) is skipped; the answer is the same.
//! Tested, not proved.
use super::{Store, Stores};
use crate::error::Result;
use crate::stale::repos;
use crate::workspace::{CANCELLED, Workspace, budgets};
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_commit, pinned_blobs, store_ls_tree};

/// At most this many `README.md` files are read per store.
pub const MAX_FILES: usize = 5_000;
/// A `README.md` larger than this is not read.
pub const MAX_BYTES: u64 = 64 * 1024;

/// One `subject:` claim of a store directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub dir: String,
    pub subject: String,
    pub heading: Option<String>,
}

/// What one declared store gave.
#[derive(Clone, Debug, Default)]
pub struct Read {
    pub name: String,
    /// `read`, `over-limit`, `missing`, `mismatch`, `partial-clone`,
    /// `no-commit` or `git-error`.
    pub state: &'static str,
    /// How the clone was found (`map`, `search-root`, `hint`), when it was.
    pub via: &'static str,
    pub commit: Option<String>,
    /// The URL a clone found through the map has instead of the declared one.
    pub other_origin: Option<String>,
    /// The clone's own `HEAD`, when it is not the commit read.
    pub head: Option<String>,
    pub url: String,
    pub entries: Vec<Entry>,
}

impl Read {
    /// Whether the directories were read from the store's clone.
    pub fn attached(&self) -> bool {
        matches!(self.state, "read" | "over-limit")
    }
}

const TABLES: &str = "\
CREATE TABLE IF NOT EXISTS store_material(store TEXT NOT NULL, dir TEXT NOT NULL, ord INTEGER NOT NULL,
    subject TEXT, heading TEXT, readme_blob TEXT NOT NULL, PRIMARY KEY(store, dir, ord));
CREATE TABLE IF NOT EXISTS store_material_at(store TEXT PRIMARY KEY, commit_id TEXT NOT NULL,
    complete INTEGER NOT NULL);";

/// One README's claims, `(subject, heading)`; no claim is one pair with
/// no subject.
type Claims = Vec<(Option<String>, Option<String>)>;
/// One README: `(dir, blob, claims)`.
type Parsed = (String, String, Claims);

/// Read every declared store. No declaration: no Git spawn, no table.
/// `WRITFOLD_KB_VERIFY=1` rereads every store and refuses when the cache
/// differs.
pub fn read_all(
    git: Option<&GitExecutable<'_>>,
    ws: &Workspace,
    conn: &Connection,
    stores: &Stores,
) -> Result<Vec<Read>> {
    stores.iter().map(|s| read_one(git, ws, conn, s)).collect()
}

fn read_one(
    git: Option<&GitExecutable<'_>>,
    ws: &Workspace,
    conn: &Connection,
    store: &Store,
) -> Result<Read> {
    let found = super::resolve(Some(ws), store);
    let mut read = Read {
        name: store.name.clone(),
        state: found.how,
        url: store.url.clone(),
        other_origin: found.other_origin.clone(),
        ..Read::default()
    };
    let Some(git_dir) = found.git_dir else {
        return Ok(read);
    };
    read.via = found.how;
    if repos::partial(&git_dir) {
        read.state = "partial-clone";
        return Ok(read);
    }
    let Some(git) = git else {
        read.state = "git-error";
        return Ok(read);
    };
    let commit = match repos::target(git, &git_dir) {
        Ok(Some((commit, _))) => commit,
        Ok(None) => {
            read.state = "no-commit";
            return Ok(read);
        }
        Err(_) => {
            read.state = "git-error";
            return Ok(read);
        }
    };
    read.commit = Some(commit.as_str().to_owned());
    read.head = head(git, &git_dir).filter(|h| h != commit.as_str());
    let verify = std::env::var("WRITFOLD_KB_VERIFY").is_ok_and(|v| v == "1");
    let cached = cached(conn, &store.name, commit.as_str());
    let fresh = match (&cached, verify) {
        (Some(_), false) => None,
        _ => Some(load(git, &git_dir, &commit, conn, &store.name)),
    };
    let (parsed, complete) = match (cached, fresh) {
        (Some(hit), Some(Ok(again))) if hit != again => {
            return Err(crate::error::KbError::Refused(format!(
                "WRITFOLD_KB_VERIFY: the material cached for store {} differs from a fresh read",
                store.name
            )));
        }
        (_, Some(Err(_))) => {
            read.state = "git-error";
            return Ok(read);
        }
        (Some(hit), _) => hit,
        (None, Some(Ok(again))) => {
            remember(conn, &store.name, commit.as_str(), &again);
            again
        }
        (None, None) => return Ok(read),
    };
    read.state = if complete { "read" } else { "over-limit" };
    read.entries = entries(&parsed);
    Ok(read)
}

fn entries(parsed: &[Parsed]) -> Vec<Entry> {
    let mut out = Vec::new();
    for (dir, _, claims) in parsed {
        for (subject, heading) in claims {
            if let Some(subject) = subject {
                out.push(Entry {
                    dir: dir.clone(),
                    subject: subject.clone(),
                    heading: heading.clone(),
                });
            }
        }
    }
    out
}

/// The clone's own `HEAD` commit.
fn head(git: &GitExecutable<'_>, git_dir: &std::path::Path) -> Option<String> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    let commit = kb_commit(git, reads, "HEAD", budgets().ok()?, &CANCELLED).ok()??;
    Some(commit.as_str().to_owned())
}

/// The README's claims: its `subject:` list and first heading.
pub fn claims(text: &str) -> Claims {
    let heading = crate::parse::first_heading(text);
    let subjects = writfold_sources::frontmatter::parse_frontmatter(text)
        .ok()
        .and_then(|block| block.get("subject").map(crate::parse::list))
        .unwrap_or_default();
    if subjects.is_empty() {
        return vec![(None, heading)];
    }
    subjects
        .into_iter()
        .map(|s| (Some(s), heading.clone()))
        .collect()
}

/// The directory a `README.md` path describes (`""` for the top).
fn dir_of(path: &str) -> Option<&str> {
    if path == "README.md" {
        return Some("");
    }
    path.strip_suffix("/README.md")
}

/// Read the store's tree at `commit`: claims of every `README.md`, reusing
/// what the cache holds for the same blob.
fn load(
    git: &GitExecutable<'_>,
    git_dir: &std::path::Path,
    commit: &GitObjectId,
    conn: &Connection,
    store: &str,
) -> Result<(Vec<Parsed>, bool)> {
    let tree = store_ls_tree(git, git_dir, commit, budgets()?, &CANCELLED)?;
    let mut complete = true;
    let mut readmes: Vec<_> = tree
        .into_iter()
        .filter(|e| dir_of(&e.path).is_some())
        .collect();
    if readmes.len() > MAX_FILES {
        readmes.truncate(MAX_FILES);
        complete = false;
    }
    let known = by_blob(conn, store);
    let mut wanted = Vec::new();
    for e in &readmes {
        if e.size <= MAX_BYTES && !known.contains_key(&e.id) {
            wanted.push(
                GitObjectId::new(commit.format(), e.id.clone()).map_err(|_| {
                    crate::error::KbError::Refused(format!("store {store}: bad blob ID {}", e.id))
                })?,
            );
        }
    }
    let bytes = pinned_blobs(git, git_dir, &wanted, budgets()?, &CANCELLED)?;
    let mut read: BTreeMap<String, Claims> = known;
    for (id, body) in wanted.iter().zip(bytes) {
        let parsed = std::str::from_utf8(&body).map(claims).unwrap_or_default();
        read.insert(id.as_str().to_owned(), parsed);
    }
    let mut out = Vec::new();
    for e in readmes {
        let Some(dir) = dir_of(&e.path) else { continue };
        match read.get(&e.id) {
            Some(claims) if e.size <= MAX_BYTES => {
                out.push((dir.to_owned(), e.id.clone(), claims.clone()))
            }
            _ => complete = false,
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((out, complete))
}

fn tables(conn: &Connection) -> bool {
    conn.execute_batch(TABLES).is_ok()
}

/// What the cache holds per README blob, for any commit.
fn by_blob(conn: &Connection, store: &str) -> BTreeMap<String, Claims> {
    let mut out: BTreeMap<String, Vec<_>> = BTreeMap::new();
    let Ok(mut statement) = conn.prepare(
        "SELECT readme_blob, dir, subject, heading FROM store_material WHERE store = ?1 ORDER BY dir, ord",
    ) else {
        return out;
    };
    let rows = statement.query_map([store], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get(2)?,
            r.get(3)?,
        ))
    });
    let mut first_dir: BTreeMap<String, String> = BTreeMap::new();
    for (blob, dir, subject, heading) in rows.into_iter().flatten().flatten() {
        let seen = first_dir.entry(blob.clone()).or_insert_with(|| dir.clone());
        if *seen == dir {
            out.entry(blob).or_default().push((subject, heading));
        }
    }
    out
}

/// The cached read for exactly `commit`, if there is one.
fn cached(conn: &Connection, store: &str, commit: &str) -> Option<(Vec<Parsed>, bool)> {
    let at: (String, bool) = conn
        .query_row(
            "SELECT commit_id, complete FROM store_material_at WHERE store = ?1",
            [store],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()??;
    if at.0 != commit {
        return None;
    }
    let mut statement = conn
        .prepare("SELECT dir, readme_blob, subject, heading FROM store_material WHERE store = ?1 ORDER BY dir, ord")
        .ok()?;
    let rows = statement
        .query_map([store], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get(2)?,
                r.get(3)?,
            ))
        })
        .ok()?;
    let mut out: Vec<Parsed> = Vec::new();
    for row in rows {
        let (dir, blob, subject, heading) = row.ok()?;
        match out.last_mut() {
            Some((d, _, claims)) if *d == dir => claims.push((subject, heading)),
            _ => out.push((dir, blob, vec![(subject, heading)])),
        }
    }
    Some((out, at.1))
}

/// Store a read; any failure (a busy index) leaves the cache as it was.
fn remember(conn: &Connection, store: &str, commit: &str, read: &(Vec<Parsed>, bool)) {
    if !tables(conn) {
        return;
    }
    let write = || -> rusqlite::Result<()> {
        conn.execute_batch("SAVEPOINT store_material")?;
        conn.execute("DELETE FROM store_material WHERE store = ?1", [store])?;
        let mut insert =
            conn.prepare_cached("INSERT INTO store_material VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        for (dir, blob, claims) in &read.0 {
            for (ord, (subject, heading)) in claims.iter().enumerate() {
                insert.execute((store, dir, ord as i64, subject, heading, blob))?;
            }
        }
        conn.execute(
            "INSERT OR REPLACE INTO store_material_at VALUES (?1, ?2, ?3)",
            (store, commit, read.1),
        )?;
        conn.execute_batch("RELEASE store_material")
    };
    if write().is_err() {
        let _ = conn.execute_batch("ROLLBACK TO store_material; RELEASE store_material");
    }
}

#[cfg(test)]
mod tests {
    use super::{claims, dir_of};

    #[test]
    fn a_readme_claims_each_subject_with_its_heading() {
        let text = "---\nsubject: [T-204, run-7]\n---\n# Review of 1234\n";
        let got = claims(text);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[0],
            (Some("T-204".into()), Some("Review of 1234".into()))
        );
        assert_eq!(got[1].0.as_deref(), Some("run-7"));
        assert_eq!(
            claims("# Only a title\n"),
            [(None, Some("Only a title".into()))]
        );
        assert_eq!(dir_of("README.md"), Some(""));
        assert_eq!(dir_of("work/a/README.md"), Some("work/a"));
        assert_eq!(dir_of("work/a/NOTREADME.md"), None);
        assert_eq!(dir_of("work/xREADME.md"), None);
    }
}
