//! The proposal queue: `record` calls not yet made.
//!
//! A proposal holds exactly the arguments of `record ticket|decision|link`
//! ([`codec`]). It lives in `<git dir>/writfold/proposals/<id>.json`,
//! beside drafts: local, untracked, never committed, and lost with an
//! ephemeral clone, deliberately (an evaluator's output is noisy). The ID
//! is a digest of the kind and the normalised arguments, so proposing the
//! same call twice leaves one file. Accepting one replays it through the
//! ordinary write path ([`crate::write::execute`]) after refusing a stale
//! citation; the proposal is removed only after the record is published.
//! There is no second write path and no new record kind.
//!
//! [`for_run`] is the public count a run-record check (`run.reflected`)
//! uses to ask "does this run hold a proposal?".
pub mod codec;
mod inbox;
mod verb;

use crate::error::{KbError, Result};
use crate::workspace::Workspace;
use crate::write::{Probe, Step};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::{Path, PathBuf};

pub use inbox::{inbox, rows};
pub use verb::{ProposalVerb, Propose, execute};

pub const SCHEMA: &str = "writfold-proposal/1";
/// The most pending proposals one session may hold.
pub const CAP: usize = 10;
/// Hex digits of the digest kept as the ID.
const ID_LEN: usize = 16;

/// One pending proposal.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: String,
    /// `ticket`, `decision` or `link`.
    pub kind: String,
    /// The `record` arguments, by flag name ([`codec`]).
    pub args: Map<String, Value>,
    pub session: Option<String>,
    pub run: Option<String>,
    /// UTC `YYYY-MM-DDTHH:MM:SSZ`.
    pub created: String,
}

impl Proposal {
    fn to_json(&self) -> Value {
        json!({"schema": SCHEMA, "id": self.id, "kind": self.kind, "args": self.args,
               "session": self.session, "run": self.run, "created": self.created})
    }

    fn from_json(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key)?.as_str().map(str::to_owned);
        (text("schema")? == SCHEMA).then_some(())?;
        Some(Self {
            id: text("id")?,
            kind: text("kind")?,
            args: value.get("args")?.as_object()?.clone(),
            session: text("session"),
            run: text("run"),
            created: text("created").unwrap_or_default(),
        })
    }
}

/// The ID of a call: the first [`ID_LEN`] hex digits of the SHA-256 of
/// its kind and its arguments in key order. Session, run and time are not
/// part of it, so the same call from two sessions is one proposal.
pub fn digest(kind: &str, args: &Map<String, Value>) -> String {
    let mut keys: Vec<&String> = args.keys().collect();
    keys.sort();
    let pairs: Vec<Value> = keys.iter().map(|k| json!([k, args[k.as_str()]])).collect();
    let material = format!("{SCHEMA}\n{kind}\n{}", Value::Array(pairs));
    let hex = crate::index::hex(&Sha256::digest(material.as_bytes()));
    hex.get(..ID_LEN).unwrap_or(&hex).to_owned()
}

/// The queue directory under a KB's local state (`<git dir>/writfold`).
pub fn dir(state: &Path) -> PathBuf {
    state.join("proposals")
}

/// A proposal ID: exactly [`ID_LEN`] lower-case hex digits.
pub fn valid_id(id: &str) -> bool {
    id.len() == ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The file of a proposal.
pub fn path(state: &Path, id: &str) -> Result<PathBuf> {
    if !valid_id(id) {
        return Err(KbError::Usage(format!(
            "{id:?} is not a proposal ID ({ID_LEN} hex digits; see `writfold kb proposals list`)"
        )));
    }
    Ok(dir(state).join(format!("{id}.json")))
}

/// A proposal, if present.
pub fn load(state: &Path, id: &str) -> Result<Option<Proposal>> {
    let file = path(state, id)?;
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| KbError::Usage(format!("{}: {e}", file.display())))?;
    match Proposal::from_json(&value) {
        Some(found) if found.id == id => Ok(Some(found)),
        _ => Err(KbError::Usage(format!(
            "{} is not a proposal",
            file.display()
        ))),
    }
}

/// Every readable proposal, oldest first, then by ID. Unreadable files
/// and dot-named leftovers are skipped.
pub fn all(state: &Path) -> Vec<Proposal> {
    let Ok(entries) = std::fs::read_dir(dir(state)) else {
        return Vec::new();
    };
    let mut out: Vec<Proposal> = entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_str()?.to_owned();
            load(state, name.strip_suffix(".json")?).ok()?
        })
        .collect();
    out.sort_by(|a, b| a.created.cmp(&b.created).then_with(|| a.id.cmp(&b.id)));
    out
}

/// What [`save`] did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Saved {
    New,
    /// The same call was already queued; nothing was written.
    Existing,
}

/// Queue a proposal, create-new: a temporary beside it, synced, then
/// linked into place (an existing file is the same call, kept as it is).
/// After a crash the queue holds the proposal whole or not at all; a
/// leftover is a dot-named `.tmp` file, never read.
pub fn save(state: &Path, proposal: &Proposal, probe: Probe) -> Result<Saved> {
    let file = path(state, &proposal.id)?;
    std::fs::create_dir_all(dir(state))?;
    if file.exists() {
        return Ok(Saved::Existing);
    }
    let temporary = dir(state).join(format!(".{}-{}.tmp", proposal.id, std::process::id()));
    let text = serde_json::to_string_pretty(&proposal.to_json())
        .map_err(|e| KbError::Usage(format!("proposal: {e}")))?;
    let mut handle = std::fs::File::create(&temporary)?;
    handle.write_all(format!("{text}\n").as_bytes())?;
    probe(Step::TempWritten);
    handle.sync_all()?;
    probe(Step::TempSynced);
    let linked = std::fs::hard_link(&temporary, &file);
    let saved = match linked {
        Ok(()) => Saved::New,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Saved::Existing,
        Err(error) => return Err(error.into()),
    };
    probe(Step::Published);
    std::fs::remove_file(&temporary)?;
    Ok(saved)
}

/// Remove a proposal (accepted or dropped). Removing one that is already
/// gone is not an error.
pub fn remove(state: &Path, id: &str) -> Result<()> {
    match std::fs::remove_file(path(state, id)?) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// The local state directory of a KB under Git.
pub fn state_of(ws: &Workspace) -> Option<PathBuf> {
    Some(ws.repo.as_ref()?.git_dir.join("writfold"))
}

/// How many pending proposals record `run`: the public hook for a
/// run-record check such as `run.reflected` ("the run holds a proposal").
/// Reads the local queue only; 0 for a KB outside Git. A proposal stops
/// counting once it is accepted (it is then a record the check can see)
/// or dropped.
pub fn for_run(ws: &Workspace, run: &str) -> usize {
    state_of(ws).map_or(0, |state| {
        all(&state)
            .iter()
            .filter(|p| p.run.as_deref() == Some(run))
            .count()
    })
}

/// Pending proposals of one session (`None`: proposals made with no
/// session, which share one bucket).
pub fn of_session(state: &Path, session: Option<&str>) -> usize {
    all(state)
        .iter()
        .filter(|p| p.session.as_deref() == session)
        .count()
}

#[cfg(test)]
mod tests {
    use super::{digest, valid_id};
    use serde_json::{Map, Value, json};

    fn args(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn ids_ignore_key_order() {
        let a = digest("ticket", &args(json!({"id": "A-1", "title": "t"})));
        let b = digest("ticket", &args(json!({"title": "t", "id": "A-1"})));
        assert_eq!(a, b);
        assert!(valid_id(&a));
        assert_ne!(
            a,
            digest("decision", &args(json!({"id": "A-1", "title": "t"})))
        );
        assert!(!valid_id("../x") && !valid_id("ABCDEF0123456789"));
    }
}
