//! Cheap observation for a polling view: no index, writes or external checkers.
use crate::error::Result;
use crate::workspace::{CANCELLED, GitTool, Workspace, budgets};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::UNIX_EPOCH;
use writfold_runtime::git::{KbRepository, kb_status};

/// The gate subject's last observed HEAD and work-tree metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stamp {
    pub name: String,
    pub head: String,
    pub dirty: usize,
    pub digest: String,
}

fn name(hash: &mut Sha256, path: &Path) {
    let text = path.to_string_lossy();
    hash.update(text.as_bytes());
    hash.update([0]);
}

fn metadata(hash: &mut Sha256, info: std::io::Result<std::fs::Metadata>) {
    match info {
        Ok(meta) => {
            hash.update([1]);
            hash.update(meta.len().to_le_bytes());
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
            hash.update(modified.map_or(0, |t| t.as_nanos()).to_le_bytes());
        }
        Err(error) => {
            hash.update([0]);
            hash.update(error.raw_os_error().unwrap_or_default().to_le_bytes());
        }
    }
}

fn files(hash: &mut Sha256, root: &Path, dir: &Path) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            hash.update(error.raw_os_error().unwrap_or_default().to_le_bytes());
            return;
        }
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        name(hash, path.strip_prefix(root).unwrap_or(&path));
        let info = std::fs::symlink_metadata(&path);
        if info.as_ref().is_ok_and(|m| m.is_dir()) {
            files(hash, root, &path);
        } else {
            metadata(hash, info);
        }
    }
}

/// Current-run bytes and metadata under drafts and both proposal stores.
/// The tracked queue is not an index input. Neither the derived index nor
/// unrelated local state moves this stamp.
pub fn local(ws: &Workspace) -> String {
    let mut hash = Sha256::new();
    hash.update(b"writfold-watch-local/1\0");
    if let Some(stores) = crate::proposal::Stores::of(ws) {
        let state = &stores.state;
        match std::fs::read(state.join("current-run")) {
            Ok(bytes) => {
                hash.update([1]);
                hash.update(bytes);
            }
            Err(error) => {
                hash.update([0]);
                hash.update(error.raw_os_error().unwrap_or_default().to_le_bytes());
            }
        }
        for dir in [crate::draft::dir(state), crate::proposal::dir(state)] {
            name(&mut hash, dir.strip_prefix(state).unwrap_or(&dir));
            files(&mut hash, state, &dir);
        }
        name(
            &mut hash,
            stores
                .tracked
                .strip_prefix(&ws.root)
                .unwrap_or(&stores.tracked),
        );
        files(&mut hash, &ws.root, &stores.tracked);
    }
    crate::index::hex(&hash.finalize())
}

/// Locate exactly the verbs' subject, then observe it with one Git status.
/// Dirty contents are never read or hashed; a metadata change costs one re-ask.
pub fn subject(
    ws: &Workspace,
    tool: &GitTool,
    cwd: &Path,
    recorded: Option<&str>,
) -> Result<Option<Stamp>> {
    let git = tool.admit()?;
    let Some(subject) = crate::gate::subject::locate(ws, &git, None, cwd, recorded)? else {
        return Ok(None);
    };
    let repo = KbRepository {
        git_dir: &subject.git_dir,
        work_tree: Some(&subject.top),
    };
    let mut status = kb_status(&git, repo, ".", budgets()?, &CANCELLED)?;
    let head = status
        .head
        .map_or_else(|| "none".into(), |h| h.as_str().to_owned());
    let mut hash = Sha256::new();
    hash.update(b"writfold-watch-subject/1\0");
    hash.update(head.as_bytes());
    status.dirty.sort_by(|a, b| a.path.cmp(&b.path));
    for entry in &status.dirty {
        name(&mut hash, Path::new(&entry.path));
        metadata(
            &mut hash,
            std::fs::symlink_metadata(subject.top.join(&entry.path)),
        );
    }
    Ok(Some(Stamp {
        name: subject.name,
        head,
        dirty: status.dirty.len(),
        digest: crate::index::hex(&hash.finalize()),
    }))
}
