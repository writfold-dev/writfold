//! The derived SQLite index. The observed snapshot is HEAD plus the digest
//! of every dirty or untracked KB input under the root, from one
//! `git status` spawn and a hash of those files only. A stored index at the
//! same snapshot is used as is; one at another snapshot is refreshed in
//! place (the changed files' Phase A rows replaced, Phase B recomputed); a
//! missing index, another schema or extractor, or an unknown stored head is
//! rebuilt in full.
pub mod build;
pub mod derive;
pub mod query;
mod refresh;
pub mod schema;
pub mod search;
pub mod verify;

use crate::error::{KbError, Result};
use crate::layout::classify;
use crate::workspace::{CANCELLED, Repo, Workspace, budgets};
use build::Input;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use writfold_runtime::git::{GitExecutable, kb_ls_files, kb_status};

/// The observed snapshot: HEAD (if any), the number of dirty KB inputs, and
/// a digest binding both (the index pin and the cursor binding).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pin {
    pub head: Option<String>,
    pub dirty: usize,
    pub digest: String,
}

impl Pin {
    /// `<commit12>[+dirty:<n>]`, or `none` outside Git or before a commit.
    pub fn label(&self) -> String {
        match &self.head {
            None => "none".into(),
            Some(head) if self.dirty == 0 => head.chars().take(12).collect(),
            Some(head) => format!("{}+dirty:{}", &head[..head.len().min(12)], self.dirty),
        }
    }
}

/// A pin and the dirty KB inputs it covers, as `(path, content digest)`
/// sorted by path.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub pin: Pin,
    pub dirty: Vec<(String, String)>,
}

/// How this invocation came by its index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexState {
    Fresh,
    /// Refreshed in place; the number of files re-derived.
    Refreshed(usize),
    Rebuilt,
    /// The write lock was not free within 500 ms: the answer comes from
    /// the stored index, this many changed paths behind.
    Behind(usize),
}

impl IndexState {
    pub fn label(self) -> String {
        match self {
            Self::Fresh => "fresh".into(),
            Self::Refreshed(n) => format!("refreshed:{n}"),
            Self::Rebuilt => "rebuilt".into(),
            Self::Behind(n) => format!("behind:{n}"),
        }
    }
}

pub struct Opened {
    pub conn: Connection,
    pub pin: Pin,
    pub state: IndexState,
    /// The index file; `None` when the KB is not under Git (in memory).
    pub file: Option<PathBuf>,
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The pin of a KB outside Git: always rebuilt in memory.
fn unpinned() -> Pin {
    Pin {
        head: None,
        dirty: 0,
        digest: hex(&Sha256::digest(b"writfold-kb-pin/1\nnone")),
    }
}

/// HEAD and the digest of every dirty KB input, from one `status` spawn.
pub fn observe(ws: &Workspace, repo: &Repo, git: &GitExecutable<'_>) -> Result<Snapshot> {
    let status = kb_status(git, repo.reads(), repo.pathspec(), budgets()?, &CANCELLED)?;
    let mut dirty = Vec::new();
    for entry in &status.dirty {
        let Some(path) = entry.path.strip_prefix(repo.prefix.as_str()) else {
            continue;
        };
        if classify(path).is_some() {
            let digest = match std::fs::read(ws.root.join(path)) {
                Ok(bytes) => hex(&Sha256::digest(&bytes)),
                Err(_) => "absent".into(),
            };
            dirty.push((path.to_owned(), digest));
        }
    }
    dirty.sort();
    dirty.dedup();
    let head = status.head.map(|id| id.as_str().to_owned());
    let material = pin_material(head.as_deref().unwrap_or("none"), &dirty);
    let pin = Pin {
        head,
        dirty: dirty.len(),
        digest: hex(&Sha256::digest(material.as_bytes())),
    };
    Ok(Snapshot { pin, dirty })
}

/// The bytes the pin digest covers: HEAD, then each dirty input.
pub fn pin_material(head: &str, dirty: &[(String, String)]) -> String {
    let mut material = format!("writfold-kb-pin/1\n{head}\n");
    for (path, digest) in dirty {
        material.push_str(&format!("{path}\0{digest}\n"));
    }
    material
}

/// The index file of a KB root inside its repository's Git directory.
pub fn index_file(repo: &Repo) -> Result<PathBuf> {
    let directory = repo.git_dir.join("writfold");
    std::fs::create_dir_all(&directory)?;
    let name = hex(&Sha256::digest(repo.prefix.as_bytes()));
    Ok(directory.join(format!("kb-index-{}.sqlite", &name[..12])))
}

/// Open the index for the current snapshot: as is, refreshed, or rebuilt
/// (always rebuilt when `force` is set).
pub fn open(ws: &Workspace, git: Option<&GitExecutable<'_>>, force: bool) -> Result<Opened> {
    let (Some(repo), Some(git)) = (&ws.repo, git) else {
        let mut conn = Connection::open_in_memory()?;
        build::build(&mut conn, inputs(&ws.root, walk(&ws.root)?), &[])?;
        return Ok(Opened {
            conn,
            pin: unpinned(),
            state: IndexState::Rebuilt,
            file: None,
        });
    };
    let file = index_file(repo)?;
    for _ in 0..ATTEMPTS {
        if let Some(opened) = attempt(ws, repo, git, &file, force)? {
            return Ok(opened);
        }
    }
    Err(KbError::Refused(
        "the KB kept changing while the index was refreshed; try again".into(),
    ))
}

/// Observations before an invocation gives up on a moving KB.
const ATTEMPTS: usize = 3;

/// One observe-derive-recheck round; `None` when the KB moved meanwhile.
fn attempt(
    ws: &Workspace,
    repo: &Repo,
    git: &GitExecutable<'_>,
    file: &Path,
    force: bool,
) -> Result<Option<Opened>> {
    let snapshot = observe(ws, repo, git)?;
    if !force && let Some(mut conn) = existing(file)? {
        let opened = |conn, pin, state| Opened {
            conn,
            pin,
            state,
            file: Some(file.to_path_buf()),
        };
        match refresh::refresh(&mut conn, ws, repo, git, &snapshot)? {
            refresh::Outcome::Current => {
                return Ok(Some(opened(conn, snapshot.pin, IndexState::Fresh)));
            }
            refresh::Outcome::Done(n) => {
                if std::env::var_os("WRITFOLD_KB_VERIFY").is_some_and(|v| v == "1") {
                    shadow(ws, repo, git, &conn, file, &snapshot)?;
                }
                return Ok(Some(opened(conn, snapshot.pin, IndexState::Refreshed(n))));
            }
            refresh::Outcome::Behind(pin, n) => {
                return Ok(Some(opened(conn, pin, IndexState::Behind(n))));
            }
            refresh::Outcome::Moved => return Ok(None),
            refresh::Outcome::Rebuild => {}
        }
    }
    let temporary = file.with_file_name(format!(".kb-rebuild-{}.tmp", std::process::id()));
    build_file(&temporary, inputs(&ws.root, listed(repo, git)?), &snapshot)?;
    if observe(ws, repo, git)?.pin.digest != snapshot.pin.digest {
        let _ = std::fs::remove_file(&temporary);
        return Ok(None);
    }
    replace(file, &temporary)?;
    Ok(Some(Opened {
        conn: connect(file)?,
        pin: snapshot.pin,
        state: IndexState::Rebuilt,
        file: Some(file.to_path_buf()),
    }))
}

/// `WRITFOLD_KB_VERIFY=1`: compare every refresh with a rebuild; on a
/// mismatch the rebuild replaces the index and the call fails.
fn shadow(
    ws: &Workspace,
    repo: &Repo,
    git: &GitExecutable<'_>,
    conn: &Connection,
    file: &Path,
    snapshot: &Snapshot,
) -> Result<()> {
    let report = verify::against_rebuild(ws, repo, git, conn, file, snapshot)?;
    match report.first_difference() {
        None => Ok(()),
        Some(difference) => Err(KbError::Mismatch(format!(
            "refresh differs from a rebuild at {difference}; the index was rebuilt"
        ))),
    }
}

/// A read-write connection that waits up to 500 ms for another writer.
pub fn connect(file: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.busy_timeout(Duration::from_millis(500))?;
    Ok(conn)
}

/// The stored index when it was built by this schema and extractor.
fn existing(file: &Path) -> Result<Option<Connection>> {
    if !file.exists() {
        return Ok(None);
    }
    let Ok(conn) = connect(file) else {
        return Ok(None);
    };
    let read = |key: &str| -> Option<String> {
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    };
    let matches = read("schema").as_deref() == Some(schema::SCHEMA)
        && read("extractor").as_deref() == Some(schema::EXTRACTOR)
        && read("dirty_set").is_some();
    Ok(matches.then_some(conn))
}

/// The `meta` rows that record the snapshot an index was derived at.
pub fn snapshot_meta(snapshot: &Snapshot) -> [(&'static str, String); 4] {
    let dirty_set: String = snapshot
        .dirty
        .iter()
        .map(|(path, digest)| format!("{path}\0{digest}\n"))
        .collect();
    [
        ("pin", snapshot.pin.digest.clone()),
        (
            "head",
            snapshot.pin.head.clone().unwrap_or_else(|| "none".into()),
        ),
        ("dirty", snapshot.pin.dirty.to_string()),
        ("dirty_set", dirty_set),
    ]
}

/// Every KB input path Git lists (tracked, or untracked and not ignored),
/// relative to the root.
pub fn listed(repo: &Repo, git: &GitExecutable<'_>) -> Result<Vec<String>> {
    let listed = kb_ls_files(git, repo.reads(), repo.pathspec(), budgets()?, &CANCELLED)?;
    Ok(listed
        .iter()
        .filter_map(|p| p.strip_prefix(repo.prefix.as_str()))
        .filter(|p| classify(p).is_some())
        .map(str::to_owned)
        .collect())
}

/// Build `inputs` into the file `target` (replaced if present).
pub fn build_file(target: &Path, inputs: Vec<Input>, snapshot: &Snapshot) -> Result<()> {
    let _ = std::fs::remove_file(target);
    let mut conn = Connection::open(target)?;
    conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
    build::build(&mut conn, inputs, &snapshot_meta(snapshot))?;
    conn.execute_batch("PRAGMA journal_mode = DELETE;")?;
    Ok(())
}

/// Move a freshly built index at `temporary` over `file`, carrying over
/// cached verdicts of citations whose token text is unchanged.
pub fn replace(file: &Path, temporary: &Path) -> Result<()> {
    if file.exists() {
        let conn = Connection::open(temporary)?;
        let keep = format!(
            "ATTACH DATABASE {} AS old;
             INSERT OR IGNORE INTO cite_checks SELECT c.* FROM old.cite_checks c
               JOIN old.citations o ON o.path = c.path AND o.ord = c.ord
               JOIN main.citations n ON n.path = c.path AND n.ord = c.ord
               WHERE o.raw = n.raw;
             DETACH DATABASE old;",
            quote(&file.to_string_lossy())
        );
        let _ = conn.execute_batch(&keep);
    }
    std::fs::File::open(temporary)?.sync_all()?;
    std::fs::rename(temporary, file)?;
    Ok(())
}

fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// One input read from the work tree; `None` when the file is gone.
pub fn input(root: &Path, path: String) -> Option<Input> {
    let kind = classify(&path)?;
    if path.contains('\u{fffd}') {
        let bytes = Err("path is not UTF-8".to_owned());
        return Some(Input { path, kind, bytes });
    }
    let bytes = match std::fs::read(root.join(&path)) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => Err(format!("cannot read: {error}")),
    };
    Some(Input { path, kind, bytes })
}

/// Read every classified path under `root` into build inputs, in path order.
pub fn inputs(root: &Path, mut paths: Vec<String>) -> Vec<Input> {
    paths.sort();
    paths.dedup();
    paths.into_iter().filter_map(|p| input(root, p)).collect()
}

/// Every regular file under `root` (outside Git), skipping dot names.
pub fn walk(root: &Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push((entry.path(), format!("{prefix}{name}/")));
            } else if kind.is_file() {
                found.push(format!("{prefix}{name}"));
            }
        }
    }
    Ok(found)
}
