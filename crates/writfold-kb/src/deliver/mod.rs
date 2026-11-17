//! G2: offline delivery candidates (`writfold kb deliver`).
//!
//! `deliver prepare` copies a work artifact into the run as a candidate:
//! the exact payload that would be posted as a PR comment or a Slack
//! summary, with a manifest naming its mode, audience and destination.
//! `deliver check` evaluates the mode's built-in checks against the
//! candidate's bytes and records one receipt per check; `deliver attest`
//! and `deliver waive` record statements; `deliver status` derives each
//! obligation's standing. Nothing is sent, ever: there is no network and
//! no adapter. The checks are mechanical; attestations are attestations.
mod answer;
pub mod candidate;
pub mod checks;
pub mod config;
mod config_check;
pub mod draw;
mod evaluate;
pub mod found;
pub mod guarded;
pub mod markdown;
pub mod mode;
mod mode_check;
mod mode_keys;
pub mod modes;
pub mod next;
mod pick;
pub mod policy;
pub mod post;
mod prepare;
mod read;
pub mod receipt;
pub mod recheck;
mod record;
pub mod render;
pub mod route;
mod said;
mod say;
pub mod script;
pub mod status;
mod status_line;
pub mod step;
pub mod withdraw;
pub mod word;

pub use answer::resume_rows;
use answer::status_answer;
use say::say;

use crate::Request;
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::gate::statement::Kind;
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use candidate::Prepare;

/// One `deliver` verb.
#[derive(Clone, Debug)]
pub enum DeliverVerb {
    Prepare(Prepare),
    Check {
        candidate: Option<String>,
        check: Option<String>,
        run: Option<String>,
        mode: Option<String>,
    },
    /// `deliver attest` or `deliver waive`.
    Say {
        kind: Kind,
        candidate: Option<String>,
        check: String,
        text: String,
        by: String,
        run: Option<String>,
        mode: Option<String>,
    },
    Status {
        candidate: Option<String>,
        run: Option<String>,
        /// `--all`: withdrawn candidates too.
        all: bool,
    },
    /// `deliver withdraw`: the candidate is not meant.
    Withdraw {
        candidate: Option<String>,
        reason: String,
        by: String,
        run: Option<String>,
        mode: Option<String>,
    },
    /// `deliver posted`: a statement that the payload was posted.
    Posted {
        candidate: Option<String>,
        reference: String,
        by: String,
        run: Option<String>,
        mode: Option<String>,
    },
    /// `deliver payload`: the bytes to post.
    Payload {
        candidate: Option<String>,
        to: Option<std::path::PathBuf>,
        run: Option<String>,
        mode: Option<String>,
    },
    /// `deliver match`: whether these bytes are a ready candidate's payload.
    Match {
        from: std::path::PathBuf,
        mode: Option<String>,
        run: Option<String>,
    },
    /// `deliver modes`: what each mode asks for.
    Modes {
        mode: Option<String>,
    },
}

fn state_dir(ws: &Workspace) -> Result<std::path::PathBuf> {
    ws.repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("deliveries need a KB under Git".into()))
}

/// The run named, else the current-run pointer; refused when closed and
/// `writing`.
fn open_run(session: &Session<'_>, named: Option<&str>, writing: bool) -> Result<Rec> {
    let rec = crate::write::run::target(session, named)?;
    if writing && runs::state(session, &rec) == "closed" {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    Ok(rec)
}

/// Answer one deliver verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &DeliverVerb, now: i64) -> Result<Output> {
    let state = state_dir(ws)?;
    let tool = GitTool::locate()?;
    let session = Session::open(ws, Some(&tool), false, now)?;
    let (shown, notes, verb_name) = match verb {
        DeliverVerb::Prepare(p) => {
            let (rec, id, notes) = prepare::prepare(session, request, p, &state)?;
            (Some((rec, id)), notes, "deliver-prepare")
        }
        DeliverVerb::Check {
            candidate,
            check,
            run,
            mode,
        } => {
            let rec = open_run(&session, run.as_deref(), true)?;
            let want = pick::Want {
                named: candidate.as_deref(),
                mode: mode.as_deref(),
                verb: Some("deliver check"),
            };
            let (id, notes) = evaluate::run(
                &session,
                &rec,
                &want,
                check.as_deref(),
                &state,
                request.probe,
            )?;
            drop(session);
            (Some((rec, id)), notes, "deliver-check")
        }
        DeliverVerb::Say { .. } => {
            let (rec, id, path, ahead) = say(&session, verb, &state, request.probe, now)?;
            drop(session);
            let note = format!("recorded {path} (a statement, not a check)");
            (
                Some((rec, id)),
                [note].into_iter().chain(ahead).collect(),
                "deliver-say",
            )
        }
        DeliverVerb::Status {
            candidate,
            run,
            all,
        } => {
            let rec = open_run(&session, run.as_deref(), false)?;
            let answer = status_answer(&session, &rec, candidate.as_deref(), *all)?;
            let frame = crate::gate::frame(request, "deliver-status", vec![rec.id.clone()]);
            return session.emit(&answer, &frame);
        }
        DeliverVerb::Withdraw { .. } | DeliverVerb::Posted { .. } => {
            let (rec, id, notes, name) = said::record(&session, verb, &state, request.probe)?;
            drop(session);
            (Some((rec, id)), notes, name)
        }
        DeliverVerb::Payload { .. } | DeliverVerb::Match { .. } | DeliverVerb::Modes { .. } => {
            return read::answer(&session, request, verb);
        }
    };
    let session = Session::open(ws, Some(&tool), false, now)?;
    let Some((rec, id)) = shown else {
        return Err(KbError::Usage("nothing to show".into()));
    };
    let mut answer = status_answer(&session, &rec, Some(&id), true)?;
    answer.notes.splice(0..0, notes);
    if verb_name == "deliver-prepare" {
        // A new candidate's obligations are open by design; preparing it
        // succeeded, and `deliver status` reports what is owed.
        answer.counts.retain(|(name, _)| *name == "candidates");
    }
    answer
        .notes
        .push("written to the KB work tree; commit it to share it".into());
    let frame = crate::gate::frame(request, verb_name, vec![rec.id.clone()]);
    session.emit(&answer, &frame)
}
