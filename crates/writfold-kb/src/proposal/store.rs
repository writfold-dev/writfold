//! The two places a proposal lives, read as one queue.
//!
//! - **Local:** `<git dir>/writfold/proposals/<id>.json`, untracked and
//!   lost with the clone.
//! - **Tracked:** `<KB root>/.writfold/proposals/<id>-<session8>.json`, in
//!   the work tree, left for the user (or agent) to commit like a `record`
//!   write, so it survives a fresh clone. It is a dot path, so it is never
//!   a record and the index does not change. The file name carries the
//!   first eight characters of the session (`none` without one): two
//!   sessions proposing the same call in two clones add two files, never
//!   one file with two contents to merge.
//!
//! A marker file `.writfold/proposals/.tracked` makes tracked the default
//! for `propose`; `--local` still forces the local queue.
//!
//! Readers ([`Stores::pending`]) deduplicate by ID: the same call is one
//! pending proposal, labelled `tracked` when any copy is tracked, holding
//! every copy's file so that accepting or dropping it removes them all.
use super::{Proposal, Saved, read, save_at, valid_id};
use crate::error::Result;
use crate::workspace::Workspace;
use crate::write::Probe;
use std::path::{Path, PathBuf};

/// The tracked store, relative to the KB root.
pub const TRACKED_DIR: &str = ".writfold/proposals";
/// The marker that makes tracked the default.
pub const MARKER: &str = ".tracked";
/// Characters of the session kept in a tracked file's name.
const SESSION_CHARS: usize = 8;

/// Which store a copy lives in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Store {
    Local,
    Tracked,
}

impl Store {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Tracked => "tracked",
        }
    }
}

/// One copy on disk.
#[derive(Clone, Debug)]
pub struct Copy {
    pub store: Store,
    pub file: PathBuf,
    pub proposal: Proposal,
}

/// A pending proposal: the oldest copy's content and every copy.
#[derive(Clone, Debug)]
pub struct Pending {
    pub proposal: Proposal,
    pub copies: Vec<Copy>,
}

impl Pending {
    /// `tracked` when any copy is tracked, else `local`.
    pub fn store(&self) -> Store {
        if self.copies.iter().any(|c| c.store == Store::Tracked) {
            Store::Tracked
        } else {
            Store::Local
        }
    }

    /// The file shown for the proposal: a tracked copy first (the one to
    /// commit), else the local one.
    pub fn file(&self) -> Option<&Path> {
        let store = self.store();
        self.copies
            .iter()
            .find(|c| c.store == store)
            .map(|c| c.file.as_path())
    }

    /// Whether any copy was made by `session` (`None`: with no session).
    pub fn of_session(&self, session: Option<&str>) -> bool {
        self.copies
            .iter()
            .any(|c| c.proposal.session.as_deref() == session)
    }
}

/// Both stores of one KB.
#[derive(Clone, Debug)]
pub struct Stores {
    /// The local state directory (`<git dir>/writfold`).
    pub state: PathBuf,
    /// `<KB root>/.writfold/proposals`.
    pub tracked: PathBuf,
}

impl Stores {
    /// The stores of a KB under Git; `None` outside Git.
    pub fn of(ws: &Workspace) -> Option<Self> {
        Some(Self {
            state: super::state_of(ws)?,
            tracked: ws.root.join(TRACKED_DIR),
        })
    }

    /// Whether `propose` writes to the tracked store without a flag.
    pub fn tracked_by_default(&self) -> bool {
        self.tracked.join(MARKER).is_file()
    }

    /// The tracked file of a proposal.
    pub fn tracked_file(&self, p: &Proposal) -> PathBuf {
        let session: String = p
            .session
            .as_deref()
            .unwrap_or("none")
            .chars()
            .take(SESSION_CHARS)
            .collect();
        self.tracked.join(format!("{}-{session}.json", p.id))
    }

    /// Every readable copy in both stores. Unreadable files, dot-named
    /// leftovers and files whose name does not match their ID are skipped.
    pub fn copies(&self) -> Vec<Copy> {
        let local = super::all(&self.state).into_iter().filter_map(|p| {
            let file = super::path(&self.state, &p.id).ok()?;
            Some(Copy {
                store: Store::Local,
                file,
                proposal: p,
            })
        });
        let mut out: Vec<Copy> = local.collect();
        out.extend(self.tracked_copies());
        out
    }

    fn tracked_copies(&self) -> Vec<Copy> {
        let Ok(entries) = std::fs::read_dir(&self.tracked) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| {
                let file = entry.ok()?.path();
                let name = file.file_name()?.to_str()?;
                let (id, rest) = name.strip_suffix(".json")?.split_once('-')?;
                (valid_id(id) && !rest.is_empty()).then_some(())?;
                let proposal = read(&file, id).ok()??;
                Some(Copy {
                    store: Store::Tracked,
                    file,
                    proposal,
                })
            })
            .collect()
    }

    /// Every pending proposal, deduplicated by ID, oldest first, then by
    /// ID.
    pub fn pending(&self) -> Vec<Pending> {
        let mut copies = self.copies();
        copies.sort_by(|a, b| {
            (
                &a.proposal.created,
                &a.proposal.id,
                a.store == Store::Tracked,
            )
                .cmp(&(
                    &b.proposal.created,
                    &b.proposal.id,
                    b.store == Store::Tracked,
                ))
        });
        let mut out: Vec<Pending> = Vec::new();
        for copy in copies {
            match out.iter_mut().find(|p| p.proposal.id == copy.proposal.id) {
                Some(found) => found.copies.push(copy),
                None => out.push(Pending {
                    proposal: copy.proposal.clone(),
                    copies: vec![copy],
                }),
            }
        }
        out
    }

    /// One pending proposal by ID (a malformed ID is a usage error).
    pub fn find(&self, id: &str) -> Result<Option<Pending>> {
        super::path(&self.state, id)?;
        Ok(self.pending().into_iter().find(|p| p.proposal.id == id))
    }

    /// Whether `store` already holds a copy of `id`.
    pub fn holds(&self, store: Store, id: &str) -> bool {
        self.copies()
            .iter()
            .any(|c| c.store == store && c.proposal.id == id)
    }

    /// Queue a proposal in `store`, create-new.
    pub fn save(&self, store: Store, p: &Proposal, probe: Probe) -> Result<(Saved, PathBuf)> {
        match store {
            Store::Local => {
                let file = super::path(&self.state, &p.id)?;
                Ok((super::save(&self.state, p, probe)?, file))
            }
            Store::Tracked => {
                let file = self.tracked_file(p);
                Ok((save_at(&self.tracked, &file, p, probe)?, file))
            }
        }
    }

    /// Remove every copy (accepted or dropped). A copy already gone is not
    /// an error.
    pub fn remove(&self, pending: &Pending) -> Result<()> {
        for copy in &pending.copies {
            match std::fs::remove_file(&copy.file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(error.into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}
