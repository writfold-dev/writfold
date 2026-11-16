//! The guard log: `<KB Git directory>/writfold/guard-log.jsonl`, what this
//! machine's hook (and `edit apply`) decided for audit rules, refusals and
//! passes that rested on a waiver. Local, untracked, mode 0600; one line
//! per entry, appended with one `write`. At 4 MiB the file is renamed to
//! `guard-log.1.jsonl`. A log that cannot be written changes no decision.
use super::rules::Rule;
use crate::workspace::Workspace;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-guard-log/1";
pub const FILE: &str = "guard-log.jsonl";
pub const ROTATED: &str = "guard-log.1.jsonl";
/// The size at which the log is rotated.
pub const LIMIT: u64 = 4 * 1024 * 1024;
/// The longest command kept, in characters.
const COMMAND: usize = 200;

/// The log's path, when the KB is under Git.
pub fn path(ws: &Workspace) -> Option<PathBuf> {
    ws.repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold").join(FILE))
}

/// The rule file on disk: KB-relative for the KB root's rules, absolute
/// for a repository's.
pub fn file(ws: &Workspace, rule: &Rule) -> PathBuf {
    ws.root.join(&rule.path)
}

/// `sha256:<hex>` of the rule file's bytes (`sha256:unreadable` when it
/// cannot be read).
pub fn digest(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes))),
        Err(_) => "sha256:unreadable".into(),
    }
}

/// A command as logged: at most 200 characters.
pub fn command(words: &str) -> String {
    words.chars().take(COMMAND).collect()
}

/// Append each entry as one line; `Err` names why the log could not be
/// written (the caller says so on stderr and decides as before).
pub fn append(log: &Path, entries: &[Value]) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    if std::fs::metadata(log).is_ok_and(|m| m.is_file() && m.len() >= LIMIT) {
        let rotated = log.with_file_name(ROTATED);
        std::fs::rename(log, rotated).map_err(|e| format!("cannot rotate the guard log: {e}"))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(log)
        .map_err(|e| format!("cannot write the guard log {}: {e}", log.display()))?;
    for entry in entries {
        let line = format!("{entry}\n");
        file.write_all(line.as_bytes())
            .map_err(|e| format!("cannot write the guard log {}: {e}", log.display()))?;
    }
    Ok(())
}

/// Every entry of the rotated and current log, oldest first, and how
/// many lines did not parse (a torn last line, say).
pub fn read(log: &Path) -> (Vec<Value>, usize) {
    let mut entries = Vec::new();
    let mut unparsed = 0;
    for path in [log.with_file_name(ROTATED), log.to_path_buf()] {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<Value>(line) {
                Ok(v) if v["schema"] == SCHEMA => entries.push(v),
                _ => unparsed += 1,
            }
        }
    }
    (entries, unparsed)
}
