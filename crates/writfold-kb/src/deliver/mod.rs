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
pub mod candidate;
pub mod checks;
pub mod config;
mod evaluate;
pub mod markdown;
pub mod receipt;
pub mod status;

use crate::Request;
use crate::emit::{Answer, Output, Row};
use crate::error::{KbError, Result};
use crate::gate::statement::{self, Kind, Statement};
use crate::index::query::Rec;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic, unique};
use crate::{runs, time};
use candidate::{Candidate, Prepare};

/// One `deliver` verb.
#[derive(Clone, Debug)]
pub enum DeliverVerb {
    Prepare(Prepare),
    Check {
        candidate: Option<String>,
        check: Option<String>,
        run: Option<String>,
    },
    /// `deliver attest` or `deliver waive`.
    Say {
        kind: Kind,
        candidate: Option<String>,
        check: String,
        text: String,
        by: String,
        run: Option<String>,
    },
    Status {
        candidate: Option<String>,
        run: Option<String>,
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
            let rec = open_run(&session, p.run.as_deref(), true)?;
            let (op, id) =
                candidate::plan(&ws.root, &request.cwd, p, (&rec.id, &runs::dir(&rec)), now)?;
            drop(session);
            atomic::apply(&ws.root, &state, &[op], request.probe)?;
            let notes = vec![format!(
                "prepared {}/{}/{id}/{}: the exact payload; nothing is sent",
                runs::dir(&rec),
                candidate::DIR,
                candidate::PAYLOAD
            )];
            (Some((rec, id)), notes, "deliver-prepare")
        }
        DeliverVerb::Check {
            candidate,
            check,
            run,
        } => {
            let rec = open_run(&session, run.as_deref(), true)?;
            let (id, notes) = evaluate::run(
                &session,
                &rec,
                candidate.as_deref(),
                check.as_deref(),
                &state,
                request.probe,
            )?;
            drop(session);
            (Some((rec, id)), notes, "deliver-check")
        }
        DeliverVerb::Say { .. } => {
            let (rec, id, path) = say(&session, verb, &state, request.probe, now)?;
            drop(session);
            let note = format!("recorded {path} (a statement, not a check)");
            (Some((rec, id)), vec![note], "deliver-say")
        }
        DeliverVerb::Status { candidate, run } => {
            let rec = open_run(&session, run.as_deref(), false)?;
            let answer = status_answer(&session, &rec, candidate.as_deref())?;
            let frame = crate::gate::frame(request, "deliver-status", vec![rec.id.clone()]);
            return session.emit(&answer, &frame);
        }
    };
    let session = Session::open(ws, Some(&tool), false, now)?;
    let Some((rec, id)) = shown else {
        return Err(KbError::Usage("nothing to show".into()));
    };
    let mut answer = status_answer(&session, &rec, Some(&id))?;
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

/// Record an attestation or a waiver about one obligation of a candidate.
fn say(
    session: &Session<'_>,
    verb: &DeliverVerb,
    state: &std::path::Path,
    probe: crate::write::Probe,
    now: i64,
) -> Result<(Rec, String, String)> {
    let DeliverVerb::Say {
        kind,
        candidate,
        check,
        text,
        by,
        run,
    } = verb
    else {
        return Err(KbError::Usage("not a statement".into()));
    };
    let flag = match kind {
        Kind::Attested => "--statement",
        Kind::Waived => "--reason",
    };
    let text = statement::check_text(flag, text)?;
    let by = statement::check_text("--by", by)?;
    let rec = open_run(session, run.as_deref(), true)?;
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(&rec), &rec.id);
    let c = candidate::pick(&cands, candidate.as_deref(), &rec.id)?;
    let obligations = status::obligations(root, c)?;
    let Some(ob) = obligations.iter().find(|o| o.name == *check) else {
        let names: Vec<&str> = obligations.iter().map(|o| o.name.as_str()).collect();
        return Err(KbError::Refused(format!(
            "{check} is not an obligation of candidate {} (its mode asks for: {})",
            c.id,
            names.join(", ")
        )));
    };
    if *kind == Kind::Attested && ob.class != "attested" {
        return Err(KbError::Refused(format!(
            "{check} is a {} check: only its own passing receipt satisfies it (deliver waive sets it aside, with a reason)",
            ob.class
        )));
    }
    let path = format!(
        "{}/{}/{check}-{}.json",
        c.dir,
        statement::DIR,
        unique(now, check)
    );
    let bytes = statement::bytes(&Statement {
        kind: *kind,
        obligation: check.clone(),
        run: rec.id.clone(),
        subject: c.id.clone(),
        commit: None,
        content: c.digest(),
        definition: ob.digest.clone(),
        text,
        by,
        recorded: time::stamp(now),
    });
    let ops = [Op::Create {
        path: path.clone(),
        bytes,
    }];
    atomic::apply(root, state, &ops, probe)?;
    let id = c.id.clone();
    Ok((rec, id, path))
}

/// The status answer for a run's candidates (or one of them).
fn status_answer(session: &Session<'_>, rec: &Rec, named: Option<&str>) -> Result<Answer> {
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(rec), &rec.id);
    let chosen: Vec<&Candidate> = match named {
        Some(_) => vec![candidate::pick(&cands, named, &rec.id)?],
        None => cands.iter().collect(),
    };
    let mut answer = Answer::default();
    let mut tally = Tally::default();
    for c in chosen {
        tally.add(&mut answer.rows, root, c, &rec.id)?;
    }
    if answer.rows.is_empty() {
        answer.unknown = Some("no-candidates");
        answer.notes.push(format!(
            "run {} has no delivery candidate: writfold kb deliver prepare --mode M --from FILE",
            rec.id
        ));
    }
    answer.counts = tally.counts();
    answer.notes.push(
        "standing is derived on every read; checks are mechanical, attestations are statements, and nothing is sent".into(),
    );
    Ok(answer)
}

/// Counts for the trailer.
#[derive(Default)]
struct Tally {
    candidates: usize,
    ready: usize,
    by: [usize; 7],
    invalid: usize,
}

const NAMES: [&str; 7] = [
    "satisfied",
    "attested",
    "waived",
    "open",
    "stale",
    "failed",
    "unknown",
];

impl Tally {
    fn add(
        &mut self,
        rows: &mut Vec<Row>,
        root: &std::path::Path,
        c: &Candidate,
        run: &str,
    ) -> Result<()> {
        let filings = status::Filings::read(root, c, run);
        self.candidates += 1;
        let invalid = status::invalid(&filings);
        self.invalid += invalid.len();
        if c.manifest.is_err() {
            self.invalid += 1;
            rows.push(status::row(c, &[]));
            rows.extend(invalid);
            return Ok(());
        }
        let judged = status::judge(status::obligations(root, c)?, c, &filings);
        if status::ready(&judged) {
            self.ready += 1;
        }
        for j in &judged {
            if let Some(i) = NAMES.iter().position(|n| *n == j.standing.name()) {
                self.by[i] += 1;
            }
        }
        rows.push(status::row(c, &judged));
        rows.extend(invalid);
        Ok(())
    }

    fn counts(&self) -> Vec<(&'static str, usize)> {
        let mut out = vec![("candidates", self.candidates), ("ready", self.ready)];
        out.extend(NAMES.iter().copied().zip(self.by));
        out.push(("invalid", self.invalid));
        out
    }
}

/// Each candidate of the run with what it still owes (`<check>
/// [<standing>]`); an empty list means it is ready.
pub fn readiness(root: &std::path::Path, rec: &Rec) -> Vec<(Candidate, Vec<String>)> {
    candidate::list(root, &runs::dir(rec), &rec.id)
        .into_iter()
        .map(|c| {
            let filings = status::Filings::read(root, &c, &rec.id);
            let owed = match status::obligations(root, &c) {
                Ok(obligations) => status::judge(obligations, &c, &filings)
                    .iter()
                    .filter(|j| !j.standing.met())
                    .map(|j| format!("{} [{}]", j.obligation.name, j.standing.name()))
                    .collect(),
                Err(error) => vec![error.to_string()],
            };
            (c, owed)
        })
        .collect()
}

/// The rows `resume` shows: the run's newest three candidates.
pub fn resume_rows(session: &Session<'_>, rec: &Rec) -> (Vec<Row>, Vec<String>) {
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(rec), &rec.id);
    let mut rows = Vec::new();
    let mut notes = Vec::new();
    let shown = cands.len().saturating_sub(3);
    for c in &cands[shown..] {
        let mut tally = Tally::default();
        if let Err(error) = tally.add(&mut rows, root, c, &rec.id) {
            notes.push(format!("deliveries: {error}"));
        }
    }
    if shown > 0 {
        notes.push(format!(
            "{} delivery candidates; the newest 3 are shown (writfold kb deliver status)",
            cands.len()
        ));
    }
    (rows, notes)
}
