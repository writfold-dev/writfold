//! The derived SQLite index: opened as is when its stored pin equals the
//! observed snapshot, rebuilt in full otherwise. The snapshot is HEAD plus
//! the digest of every dirty or untracked KB input under the root, from one
//! `git status` spawn and a hash of those files only.
pub mod build;
pub mod query;
pub mod schema;

use crate::error::Result;
use crate::layout::classify;
use crate::workspace::{CANCELLED, Workspace, budgets};
use build::Input;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use writfold_runtime::git::{GitExecutable, kb_ls_files, kb_status};

/// The observed snapshot: HEAD (if any), the number of dirty KB inputs, and
/// a digest binding both (the index pin and the cursor binding).
#[derive(Clone, Debug)]
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

/// How this invocation came by its index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexState {
    Fresh,
    Rebuilt,
}

impl IndexState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Rebuilt => "rebuilt",
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

/// Open the index for the current snapshot, rebuilding it when the stored
/// pin, schema or extractor differ, or when `force` is set.
pub fn open(ws: &Workspace, git: Option<&GitExecutable<'_>>, force: bool) -> Result<Opened> {
    let (Some(repo), Some(git)) = (&ws.repo, git) else {
        let mut conn = Connection::open_in_memory()?;
        build::build(&mut conn, inputs(&ws.root, walk(&ws.root)?), &[])?;
        let pin = Pin {
            head: None,
            dirty: 0,
            digest: hex(&Sha256::digest(b"writfold-kb-pin/1\nnone")),
        };
        return Ok(Opened {
            conn,
            pin,
            state: IndexState::Rebuilt,
            file: None,
        });
    };
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
            dirty.push(format!("{path}\0{digest}\n"));
        }
    }
    dirty.sort();
    let head = status.head.map(|id| id.as_str().to_owned());
    let mut material = format!("writfold-kb-pin/1\n{}\n", head.as_deref().unwrap_or("none"));
    material.extend(dirty.iter().map(String::as_str));
    let pin = Pin {
        head,
        dirty: dirty.len(),
        digest: hex(&Sha256::digest(material.as_bytes())),
    };
    let directory = repo.git_dir.join("writfold");
    std::fs::create_dir_all(&directory)?;
    let name = hex(&Sha256::digest(repo.prefix.as_bytes()));
    let file = directory.join(format!("kb-index-{}.sqlite", &name[..12]));
    if !force && let Some(conn) = current(&file, &pin)? {
        return Ok(Opened {
            conn,
            pin,
            state: IndexState::Fresh,
            file: Some(file),
        });
    }
    let listed = kb_ls_files(git, repo.reads(), repo.pathspec(), budgets()?, &CANCELLED)?;
    let paths = listed
        .iter()
        .filter_map(|p| p.strip_prefix(repo.prefix.as_str()))
        .map(str::to_owned)
        .collect();
    rebuild(&file, inputs(&ws.root, paths), &pin)?;
    let conn = Connection::open(&file)?;
    Ok(Opened {
        conn,
        pin,
        state: IndexState::Rebuilt,
        file: Some(file),
    })
}

/// The stored index when it matches `pin` and this build's derivation.
fn current(file: &Path, pin: &Pin) -> Result<Option<Connection>> {
    if !file.exists() {
        return Ok(None);
    }
    let Ok(conn) = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_WRITE) else {
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
        && read("pin").as_deref() == Some(pin.digest.as_str());
    Ok(matches.then_some(conn))
}

/// Build into a temporary file beside the index, keep cached citation
/// verdicts whose token is unchanged, then rename it into place.
fn rebuild(file: &Path, inputs: Vec<Input>, pin: &Pin) -> Result<()> {
    let temporary = file.with_file_name(format!(".kb-rebuild-{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&temporary);
    let mut conn = Connection::open(&temporary)?;
    conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
    let meta = [
        ("pin", pin.digest.clone()),
        ("head", pin.head.clone().unwrap_or_else(|| "none".into())),
        ("dirty", pin.dirty.to_string()),
    ];
    build::build(&mut conn, inputs, &meta)?;
    if file.exists() {
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
    conn.execute_batch("PRAGMA journal_mode = DELETE;")?;
    drop(conn);
    std::fs::File::open(&temporary)?.sync_all()?;
    std::fs::rename(&temporary, file)?;
    Ok(())
}

fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// Read every classified path under `root` into build inputs, in path order.
fn inputs(root: &Path, mut paths: Vec<String>) -> Vec<Input> {
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .filter_map(|path| {
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
        })
        .collect()
}

/// Every regular file under `root` (outside Git), skipping dot names.
fn walk(root: &Path) -> Result<Vec<String>> {
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
