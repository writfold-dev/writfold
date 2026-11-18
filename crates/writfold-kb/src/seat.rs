//! A seat: the agent session and the work tree a verb or a hook runs for,
//! and the local current-run pointers a seat reads and writes.
//!
//! Under `<KB git dir>/writfold/`:
//!
//! ```text
//! current-run                  the shared pointer: the run last made current on this clone
//! current-run.d/session-<id>   one session's pointer
//! current-run.d/tree-<16 hex>  one work tree's pointer (line 2 names the tree)
//! ```
//!
//! Each holds a run ID on its first line. The session is the hook event's
//! `session_id`, else `$WRITFOLD_SESSION`; the work tree is the one
//! enclosing the working directory (for a guarded command, the directory
//! it acts in), unless that is the KB's own work tree, which has only the
//! shared pointer. Which pointer decides, and which are written, is
//! [`writfold_core::pointer`] (proved); this module reads and writes the
//! files. The pointers are local and untracked: deleting any of them only
//! moves a seat to the next, less specific one.
use crate::error::Result;
use crate::workspace::{Workspace, find_repo};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use writfold_core::pointer::{self, Level, Pointer};

/// The directory of the session and work-tree pointers.
pub const DIR: &str = "current-run.d";
/// The shared pointer's file.
pub const SHARED: &str = "current-run";
/// The variable a session's ID reaches verbs under.
pub const SESSION_ENV: &str = "WRITFOLD_SESSION";

/// Who is asking: a session, a work tree, both or neither.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seat {
    pub session: Option<String>,
    /// The canonical top of the work tree, when it is not the KB's own.
    pub tree: Option<PathBuf>,
}

/// The run a seat's pointers name, and the pointer that decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    /// `session`, `tree` or `shared`.
    pub level: &'static str,
}

/// A usable session ID, or `None`.
pub fn session_id(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|id| crate::draft::valid_session(id))
        .map(str::to_owned)
}

/// The session `$WRITFOLD_SESSION` names, when it is a session ID. Read
/// by the binary, never by the library: a test run inside a session is
/// not that session.
pub fn session_from_env() -> Option<String> {
    session_id(std::env::var(SESSION_ENV).ok().as_deref())
}

/// The work tree enclosing `dir`, unless it is the KB's own.
pub fn tree_of(kb_tree: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    let repo = find_repo(dir).ok().flatten()?;
    let top = std::fs::canonicalize(&repo.work_tree).unwrap_or(repo.work_tree);
    let own = kb_tree.map(|t| std::fs::canonicalize(t).unwrap_or_else(|_| t.to_path_buf()));
    (own.as_deref() != Some(top.as_path())).then_some(top)
}

impl Seat {
    /// The seat of `session` working in `dir`.
    pub fn at(ws: &Workspace, session: Option<String>, dir: &Path) -> Self {
        let kb_tree = ws.repo.as_ref().map(|r| r.work_tree.as_path());
        Self {
            session,
            tree: tree_of(kb_tree, dir),
        }
    }

    /// This seat's session, acting in `dir`.
    pub fn moved_to(&self, ws: &Workspace, dir: &Path) -> Self {
        Self::at(ws, self.session.clone(), dir)
    }

    /// `session s-1, work tree /w/repo, and the shared pointer`.
    pub fn describe(&self) -> String {
        let mut said = Vec::new();
        if let Some(id) = &self.session {
            said.push(format!("session {id}"));
        }
        if let Some(tree) = &self.tree {
            said.push(format!("work tree {}", tree.display()));
        }
        said.push("the shared pointer".into());
        said.join(", ")
    }

    fn session_file(&self, state: &Path) -> Option<PathBuf> {
        let id = self.session.as_deref()?;
        Some(state.join(DIR).join(format!("session-{id}")))
    }

    fn tree_file(&self, state: &Path) -> Option<PathBuf> {
        let tree = self.tree.as_deref()?;
        let digest = Sha256::digest(tree.to_string_lossy().as_bytes());
        let hex = crate::index::hex(&digest);
        Some(
            state
                .join(DIR)
                .join(format!("tree-{}", hex.get(..16).unwrap_or(&hex))),
        )
    }
}

/// The run ID a pointer file names, if it is set.
fn named(file: &Path) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let id = text.lines().next().unwrap_or_default().trim();
    (!id.is_empty()).then(|| id.to_owned())
}

/// How the run `id` reads from the KB's files: gone without a manifest,
/// closed when the sidecar (else the manifest) says so, else open.
fn standing(root: &Path, id: &str) -> Pointer {
    let dir = root.join("runs").join(id);
    if id.contains(['/', '\\']) || matches!(id, "." | "..") || !dir.join("manifest.json").is_file()
    {
        return Pointer::Gone;
    }
    let state = |file: &str| {
        let text = std::fs::read_to_string(dir.join(file)).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        value.get("state")?.as_str().map(str::to_owned)
    };
    match state(crate::runs::SIDECAR).or_else(|| state("manifest.json")) {
        Some(word) if word == "closed" => Pointer::Closed,
        _ => Pointer::Open,
    }
}

/// The seat's three pointers, most specific first.
fn pointers(state: &Path, seat: &Seat) -> [Option<String>; 3] {
    [
        seat.session_file(state).and_then(|f| named(&f)),
        seat.tree_file(state).and_then(|f| named(&f)),
        named(&state.join(SHARED)),
    ]
}

fn read(root: &Path, id: &Option<String>) -> Pointer {
    id.as_deref()
        .map_or(Pointer::Unset, |id| standing(root, id))
}

/// The seat's current run: the most specific pointer that is set, and
/// whether the run it names is open.
pub fn current(ws: &Workspace, seat: &Seat) -> Option<(Found, bool)> {
    let state = ws.repo.as_ref()?.git_dir.join("writfold");
    let [session, tree, shared] = pointers(&state, seat);
    let read = [
        read(&ws.root, &session),
        read(&ws.root, &tree),
        read(&ws.root, &shared),
    ];
    let open = pointer::open(&read[0], &read[1], &read[2]);
    let (id, level) = match pointer::level(&read[0], &read[1], &read[2])? {
        Level::Session => (session, "session"),
        Level::Tree => (tree, "tree"),
        Level::Shared => (shared, "shared"),
    };
    Some((Found { id: id?, level }, open))
}

fn write(state: &Path, file: &Path, text: &str) -> Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let temporary = state.join(format!(".{name}-{}.tmp", std::process::id()));
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, file)?;
    Ok(())
}

/// Make `id` the seat's current run: its session's pointer, its work
/// tree's, and the shared one ([`pointer::writes`]). The most specific is
/// written first, so a crash part-way leaves the seat itself reading the
/// new run and at worst a less specific pointer on the earlier one.
pub fn make_current(state: &Path, seat: &Seat, id: &str) -> Result<()> {
    std::fs::create_dir_all(state)?;
    let which = pointer::writes(seat.session.is_some(), seat.tree.is_some());
    if which.session
        && let Some(file) = seat.session_file(state)
    {
        write(state, &file, &format!("{id}\n"))?;
    }
    if which.tree
        && let (Some(file), Some(tree)) = (seat.tree_file(state), &seat.tree)
    {
        write(state, &file, &format!("{id}\n{}\n", tree.display()))?;
    }
    if which.shared {
        write(state, &state.join(SHARED), &format!("{id}\n"))?;
    }
    Ok(())
}
