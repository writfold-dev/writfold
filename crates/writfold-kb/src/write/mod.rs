//! Tracked-file writes (M3): `checkpoint`, `record ticket|decision|link`,
//! `status --set` and `cite make`. Each verb first plans its writes from
//! the index at the current snapshot (validating IDs, vocabularies and
//! targets, refusing conflicts with exit 3), then applies them through
//! the atomic protocol in [`atomic`], then answers from the index at the
//! new snapshot. Files are written in the shapes the KB already uses, so
//! the earlier prototype keeps accepting them. Writes leave the work tree
//! dirty; committing is the user's (or agent's) step.
pub mod atomic;
pub mod cite;
pub mod json;
pub mod record;
pub mod run;
pub mod status;

use crate::emit::{Answer, Frame, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::verbs::record_row;
use crate::workspace::{GitTool, Workspace};
use crate::{Request, index::query};

/// A named point in the write protocol, after which a fault test may stop
/// the process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    Locked,
    TempWritten,
    TempSynced,
    Rechecked,
    Published,
    DirSynced,
    Unlocked,
}

impl Step {
    pub const ALL: [Step; 7] = [
        Step::Locked,
        Step::TempWritten,
        Step::TempSynced,
        Step::Rechecked,
        Step::Published,
        Step::DirSynced,
        Step::Unlocked,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Step::Locked => "locked",
            Step::TempWritten => "temp-written",
            Step::TempSynced => "temp-synced",
            Step::Rechecked => "rechecked",
            Step::Published => "published",
            Step::DirSynced => "dir-synced",
            Step::Unlocked => "unlocked",
        }
    }
}

/// Called after each step; production passes [`no_probe`].
pub type Probe = fn(Step);

pub fn no_probe(_: Step) {}

/// One planned change to the KB, with paths relative to the root.
#[derive(Clone, Debug)]
pub enum Op {
    /// A new file; refused if the path exists.
    Create { path: String, bytes: Vec<u8> },
    /// Replace a file whose current bytes still have this SHA-256 (hex).
    Replace {
        path: String,
        expected: String,
        bytes: Vec<u8>,
    },
    /// A new directory holding these files (a name ending in `/` is an
    /// empty subdirectory), staged and renamed into place.
    CreateDir {
        path: String,
        files: Vec<(String, Vec<u8>)>,
    },
    /// Refuse when an entry of `dir` starts with `prefix`.
    AbsentPrefix {
        dir: String,
        prefix: String,
        what: String,
    },
}

/// What a write verb will do, and what it shows afterwards.
pub struct Plan {
    pub ops: Vec<Op>,
    /// The record (or run) to show from the index after the write.
    pub show: Option<String>,
    pub notes: Vec<String>,
    /// The run to make current (`<git dir>/writfold/current-run`).
    pub current: Option<String>,
    pub verb: &'static str,
    pub args: Vec<String>,
}

/// A write verb with its arguments.
#[derive(Clone, Debug)]
pub enum Write {
    Checkpoint(run::Checkpoint),
    /// `checkpoint --from-draft`: confirm a hook's local draft.
    FromDraft(run::FromDraft),
    Ticket(record::Ticket),
    Decision(record::Decision),
    Link(record::Link),
    SetStatus(status::SetStatus),
    CiteMake(cite::CiteMake),
}

/// Plan, apply and answer one write.
pub fn execute(ws: &Workspace, request: &Request, write: &Write, now: i64) -> Result<Output> {
    let tool = GitTool::locate()?;
    if let Write::CiteMake(make) = write {
        return cite::make(ws, &tool, request, make, now);
    }
    let Some(repo) = &ws.repo else {
        return Err(KbError::Refused(
            "writes need a KB under Git (this one would be unpinned)".into(),
        ));
    };
    let state = repo.git_dir.join("writfold");
    let plan = {
        let session = Session::open(ws, Some(&tool), false, now)?;
        match write {
            Write::Checkpoint(c) => run::plan(&session, c, &request.cwd)?,
            Write::FromDraft(f) => {
                let checkpoint = run::from_draft(&session, &state, f)?;
                run::plan(&session, &checkpoint, &request.cwd)?
            }
            Write::Ticket(t) => record::ticket(&session, t)?,
            Write::Decision(d) => record::decision(&session, d)?,
            Write::Link(l) => record::link(&session, l)?,
            Write::SetStatus(s) => status::plan(&session, s)?,
            Write::CiteMake(_) => return Err(KbError::Usage("unreachable".into())),
        }
    };
    atomic::apply(&ws.root, &state, &plan.ops, request.probe)?;
    if let Some(id) = &plan.current {
        crate::runs::set_current(&state, id)?;
    }
    let mut notes = plan.notes.clone();
    if let Write::FromDraft(f) = write {
        crate::draft::remove(&state, &f.session)?;
        notes.push(format!("draft {} confirmed and removed", f.session));
    }
    let session = Session::open(ws, Some(&tool), false, now)?;
    let mut answer = Answer {
        notes,
        ..Answer::default()
    };
    if let Some(id) = &plan.show {
        match query::record(&session.conn, id)? {
            Some(rec) if rec.kind == "run" => {
                answer.rows.extend(crate::runs::rows(&session, &rec)?)
            }
            Some(rec) => answer.rows.push(record_row(&session, &rec)),
            None => answer
                .notes
                .push(format!("{id} was written but is not in the index")),
        }
    }
    answer
        .notes
        .push("written to the work tree; commit it to share it".into());
    let frame = Frame {
        verb: plan.verb.to_owned(),
        args: plan.args,
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    session.emit(&answer, &frame)
}

/// `[a, b]`, the list form of front matter.
pub fn list(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

/// Lower case, runs of other characters as one `-`, trimmed; `item` when
/// nothing is left.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "item".into()
    } else {
        out.to_owned()
    }
}

/// A lower-case slug as record IDs use: `[a-z0-9]`, inner `-`.
pub fn is_slug(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('-')
        && !text.ends_with('-')
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A single-line, non-empty value.
pub fn one_line(flag: &str, text: &str) -> Result<String> {
    let text = text.trim();
    if text.is_empty() || text.contains(['\n', '\r']) {
        return Err(KbError::Usage(format!("{flag} needs one non-empty line")));
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{is_slug, slug};

    #[test]
    fn slugs() {
        assert_eq!(slug("  Fix the Flush, now! "), "fix-the-flush-now");
        assert_eq!(slug("ÉÉ"), "item");
        assert!(is_slug("harbor-api") && !is_slug("Harbor") && !is_slug("a-"));
    }
}
