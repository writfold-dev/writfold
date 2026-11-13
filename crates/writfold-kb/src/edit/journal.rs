//! The pre-apply journal: a local, untracked file,
//! `<KB git dir>/writfold/edits/<run>.json`, made durable before the first
//! worktree file is touched and removed once the edit and its receipt are
//! complete. It names the worktree, every file with the SHA-256 of its
//! before and after bytes, the after bytes themselves, the temporary name
//! each file is written through, and the KB writes (the receipt, and the
//! sidecar when `--worktree` is recorded). After a crash between two files,
//! `edit recover` reads it to tell applied files (after digest) from
//! unapplied ones (before digest) and finishes the batch.
use crate::error::{KbError, Result};
use crate::index::hex;
use crate::write::Op;
use crate::write::atomic::sync_dir;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "writfold-edit-journal/1";

/// One worktree file of the batch.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    pub before: String,
    pub after: String,
    /// The dot-named temporary beside the file.
    pub temp: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Journal {
    pub run: String,
    pub worktree: PathBuf,
    pub receipt: String,
    pub files: Vec<Entry>,
    /// The KB writes, after the files.
    pub kb: Vec<Op>,
}

/// Where the journal of `run` lives.
pub fn path(state: &Path, run: &str) -> PathBuf {
    state.join("edits").join(format!("{run}.json"))
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

fn op_json(op: &Op) -> Value {
    match op {
        Op::Create { path, bytes } => json!({"op": "create", "path": path, "bytes": hex(bytes)}),
        Op::Replace {
            path,
            expected,
            bytes,
        } => json!({"op": "replace", "path": path, "expected": expected, "bytes": hex(bytes)}),
        Op::CreateDir { path, .. }
        | Op::AbsentPrefix { dir: path, .. }
        | Op::Plan { run_dir: path, .. } => {
            json!({"op": "unsupported", "path": path})
        }
    }
}

impl Journal {
    fn to_json(&self) -> Value {
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                json!({"path": f.path, "before": f.before, "after": f.after,
                       "temp": f.temp, "bytes": hex(&f.bytes)})
            })
            .collect();
        json!({
            "schema": SCHEMA,
            "run": self.run,
            "worktree": self.worktree.to_string_lossy(),
            "receipt": self.receipt,
            "files": files,
            "kb": self.kb.iter().map(op_json).collect::<Vec<_>>(),
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        let text = |v: &Value, k: &str| v.get(k)?.as_str().map(str::to_owned);
        if text(value, "schema")? != SCHEMA {
            return None;
        }
        let mut files = Vec::new();
        for f in value.get("files")?.as_array()? {
            files.push(Entry {
                path: text(f, "path")?,
                before: text(f, "before")?,
                after: text(f, "after")?,
                temp: text(f, "temp")?,
                bytes: unhex(&text(f, "bytes")?)?,
            });
        }
        let mut kb = Vec::new();
        for o in value.get("kb")?.as_array()? {
            let (path, bytes) = (text(o, "path")?, unhex(&text(o, "bytes")?)?);
            kb.push(match text(o, "op")?.as_str() {
                "create" => Op::Create { path, bytes },
                "replace" => Op::Replace {
                    path,
                    expected: text(o, "expected")?,
                    bytes,
                },
                _ => return None,
            });
        }
        Some(Self {
            run: text(value, "run")?,
            worktree: PathBuf::from(text(value, "worktree")?),
            receipt: text(value, "receipt")?,
            files,
            kb,
        })
    }

    /// Write the journal durably: a synced temporary, renamed into place,
    /// then the directory synced.
    pub fn store(&self, state: &Path) -> Result<()> {
        let target = path(state, &self.run);
        let dir = target
            .parent()
            .ok_or_else(|| KbError::Usage("journal path".into()))?;
        std::fs::create_dir_all(dir)?;
        let temp = dir.join(format!(".{}.{}.tmp", self.run, std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)?;
        file.write_all(self.to_json().to_string().as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, &target)?;
        sync_dir(dir)
    }

    /// The journal of `run`, if one is pending.
    pub fn load(state: &Path, run: &str) -> Result<Option<Self>> {
        let file = path(state, run);
        let text = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| KbError::Usage(format!("{}: {e}", file.display())))?;
        Self::from_json(&value)
            .map(Some)
            .ok_or_else(|| KbError::Usage(format!("{} is not an edit journal", file.display())))
    }

    /// Remove the journal and sync its directory.
    pub fn clear(state: &Path, run: &str) -> Result<()> {
        let file = path(state, run);
        std::fs::remove_file(&file)?;
        match file.parent() {
            Some(dir) => sync_dir(dir),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, Journal, unhex};
    use crate::write::Op;

    #[test]
    fn round_trip() {
        let journal = Journal {
            run: "run-x".into(),
            worktree: "/w".into(),
            receipt: "runs/run-x/edits/a.md".into(),
            files: vec![Entry {
                path: "src/a.rs".into(),
                before: "b".into(),
                after: "a".into(),
                temp: ".t".into(),
                bytes: vec![0, 255, 10],
            }],
            kb: vec![Op::Create {
                path: "p".into(),
                bytes: b"x".to_vec(),
            }],
        };
        let back = Journal::from_json(&journal.to_json());
        assert!(back.is_some_and(|j| j.files[0].bytes == vec![0, 255, 10] && j.kb.len() == 1));
        assert_eq!(unhex("0aff"), Some(vec![10, 255]));
        assert_eq!(unhex("0"), None);
    }
}
