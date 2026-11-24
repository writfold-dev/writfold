//! G1: executable checks with pinned receipts (`writfold kb gate`).
//!
//! A gate is a command named in a small TOML definition ([`def`]). `gate
//! run` executes it on a subject repository and records one receipt in the
//! KB run ([`receipt`]); `gate status` derives each gate's obligation state
//! from the definitions and the receipts ([`status`]); `gate verify`
//! re-runs a receipt's command at its commit in a temporary worktree; `gate
//! lines` prints the receipt lines for a commit message; `gate cover`
//! records one item of a checklist gate ([`checklist`]). State is never
//! stored. The claims are mistake resistance and Git tamper evidence: a
//! receipt is a tracked file whose history Git keeps, and anyone can
//! re-run it. Nothing here signs anything or stops a determined forger.
pub mod checklist;
pub mod def;
pub mod def_accept;
pub mod def_check;
mod def_class;
#[cfg(test)]
mod def_tests;
pub(crate) mod discover;
pub mod exec;
pub(crate) mod ground;
mod head;
mod lines;
pub(crate) mod own;
pub mod pins;
pub mod places;
pub mod policy;
pub mod receipt;
pub mod repos;
pub mod retain;
mod run;
mod say;
pub mod statement;
pub mod status;
pub mod subject;
pub mod toml;
mod verify;

use crate::emit::{Answer, Frame, Output, Row};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::{Request, runs};
use serde_json::Value;
use status::Filings;
use std::path::{Path, PathBuf};
use subject::Subject;
use writfold_core::gate::{Standing, State};

pub use say::Say;

/// One `gate` verb.
#[derive(Clone, Debug)]
pub enum GateVerb {
    Run {
        id: String,
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    Status {
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    Verify {
        receipt: String,
        repo: Option<PathBuf>,
    },
    Lines {
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    /// `gate attest` or `gate waive`: a statement, not a check.
    Say(Say),
    /// `gate cover`: one row about one item of a checklist gate.
    Cover(checklist::Cover),
    /// `gate cover --from`: several rows of one checklist gate, all or none.
    CoverBatch(checklist::Batch),
}

/// Answer one gate verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &GateVerb, now: i64) -> Result<Output> {
    if ws.repo.is_none() {
        return Err(KbError::Refused(
            "gates need a KB under Git (receipts are tracked files)".into(),
        ));
    }
    let named = match verb {
        GateVerb::Run { id, .. } | GateVerb::Say(Say { id, .. }) => Some(id),
        GateVerb::Cover(checklist::Cover { gate, .. })
        | GateVerb::CoverBatch(checklist::Batch { gate, .. }) => Some(gate),
        _ => None,
    };
    if let Some(word) = named.filter(|w| crate::deliver::step::mode_of(w).is_some()) {
        return Err(KbError::Refused(format!(
            "{word} is a delivery, not a gate: use writfold kb deliver check|attest|waive"
        )));
    }
    let tool = GitTool::locate()?;
    match verb {
        GateVerb::Run { id, run, repo } => {
            run::run(ws, &tool, request, id, run.as_deref(), repo.as_deref(), now)
        }
        GateVerb::Say(say) => say::record(ws, &tool, request, say, now),
        GateVerb::Cover(cover) => checklist::write::record(ws, &tool, request, cover, now),
        GateVerb::CoverBatch(b) => checklist::batch::record(ws, &tool, request, b, now),
        GateVerb::Verify { receipt, repo } => {
            verify::verify(ws, &tool, request, receipt, repo.as_deref(), now)
        }
        GateVerb::Status { run, repo } => {
            let session = Session::open(ws, Some(&tool), false, now)?;
            let rec = target(&session, run.as_deref())?;
            let (answer, subject) = report(&session, &rec, repo.as_deref(), &request.cwd)?;
            let frame = frame(request, "gate-status", vec![rec.id.clone(), subject]);
            session.emit(&answer, &frame)
        }
        GateVerb::Lines { run, repo } => {
            let session = Session::open(ws, Some(&tool), false, now)?;
            let rec = target(&session, run.as_deref())?;
            lines::lines(&session, &rec, repo.as_deref(), &request.cwd)
        }
    }
}

pub(crate) fn frame(request: &Request, verb: &str, args: Vec<String>) -> Frame {
    Frame {
        verb: verb.to_owned(),
        args,
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    }
}

/// The run named, else the current-run pointer.
pub(crate) fn target(session: &Session<'_>, named: Option<&str>) -> Result<Rec> {
    crate::write::run::target(session, named)
}

/// The run's recorded worktree (a local path from its sidecar), if any.
pub(crate) fn recorded(session: &Session<'_>, rec: &Rec) -> Option<String> {
    let (_, map) = runs::sidecar(session, rec)?;
    map.get("worktree")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

pub(crate) fn locate(
    session: &Session<'_>,
    rec: &Rec,
    repo: Option<&Path>,
    cwd: &Path,
) -> Result<Option<Subject>> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    subject::locate(
        session.ws,
        git,
        repo,
        cwd,
        recorded(session, rec).as_deref(),
    )
}

/// The status answer for a run, and the subject's name.
fn report(
    session: &Session<'_>,
    rec: &Rec,
    repo: Option<&Path>,
    cwd: &Path,
) -> Result<(Answer, String)> {
    let Some(subject) = locate(session, rec, repo, cwd)? else {
        return Err(KbError::Refused(
            "no subject repository: run inside one, or pass --repo DIR".into(),
        ));
    };
    let mut answer = Answer::default();
    let (rows, counts, note, diagnostics) = gate_rows(session, rec, &subject)?;
    answer.report = Some(diagnostics);
    let owed = crate::plan::owed(session, rec);
    let words: Vec<&str> = owed.deliveries.iter().map(|(w, _)| w.as_str()).collect();
    if rows.is_empty() {
        answer.unknown = Some("no-gates");
        answer.notes.push(match &owed.gates {
            Some(ids) if ids.is_empty() && !words.is_empty() => format!(
                "no gate is owed here: the run's plan names only deliveries ({}), judged at their steps: writfold kb plan status",
                words.join(", ")
            ),
            _ => none_owed(session, &owed, &subject),
        });
    } else if !words.is_empty() {
        let at: Vec<String> = owed
            .deliveries
            .iter()
            .map(|(w, s)| format!("{w} at {s}"))
            .collect();
        answer.notes.push(format!(
            "the plan also owes {}; a delivery is judged at its step: writfold kb plan status",
            at.join(", ")
        ));
    }
    answer.rows = rows;
    answer.counts = counts;
    answer.notes.extend(note);
    answer.notes.push(format!(
        "receipts of run {} for {}; state is derived on every read, never stored",
        rec.id, subject.name
    ));
    Ok((answer, subject.name))
}

/// Why `gate status` shows no gate: none is defined, or the run's plan
/// names none of those that are.
fn none_owed(session: &Session<'_>, owed: &status::Owed, subject: &Subject) -> String {
    let defined = def::all(&session.ws.root, Some(&subject.top)).0.len();
    let planned_none = owed.gates.as_ref().is_some_and(Vec::is_empty);
    match planned_none && defined > 0 {
        true => format!("no gate is owed: the run's plan names none ({defined} defined)"),
        false => format!(
            "no gates defined: add {}/<id>.toml to the KB root or {}",
            def::DIR,
            subject.name
        ),
    }
}

/// Trailer counts by state.
type Counts = Vec<(&'static str, usize)>;
type Rows = (Vec<Row>, Counts, Option<String>, crate::diag::Report);

fn gate_rows(session: &Session<'_>, rec: &Rec, subject: &Subject) -> Result<Rows> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let filings = Filings::for_run(session, rec);
    let report = status::judge(session, subject, &filings)?;
    let rows = status::rows(git, subject, &report, &filings)?;
    let diagnostics = report.diagnostics();
    Ok((
        rows,
        counts(&report, &filings),
        report.recorded_note(),
        diagnostics,
    ))
}

/// The gate rows `resume` shows for a run, or why there are none. Quiet
/// when no gate is defined anywhere.
pub fn resume_rows(session: &Session<'_>, rec: &Rec, cwd: &Path) -> (Vec<Row>, Vec<String>) {
    let subject = match locate(session, rec, None, cwd) {
        Ok(subject) => subject,
        Err(error) => return (Vec::new(), vec![format!("gates: {error}")]),
    };
    let Some(subject) = subject else {
        let (defined, _) = def::all(&session.ws.root, None);
        return match defined.iter().all(def::Definition::on_candidate) {
            true => (Vec::new(), Vec::new()),
            false => (
                Vec::new(),
                vec![
                    "gates: no subject repository here; see writfold kb gate status --repo DIR"
                        .into(),
                ],
            ),
        };
    };
    let (defined, problems) = def::all(&session.ws.root, Some(&subject.top));
    if defined.iter().all(def::Definition::on_candidate) && problems.is_empty() {
        return (Vec::new(), Vec::new());
    }
    match gate_rows(session, rec, &subject) {
        Ok((rows, ..)) => (rows, Vec::new()),
        Err(error) => (Vec::new(), vec![format!("gates: {error}")]),
    }
}

/// Counts by standing for the trailer, in a fixed order; invalid
/// receipts, statements and definitions are counted together.
fn counts(report: &status::Report<'_>, filings: &Filings) -> Vec<(&'static str, usize)> {
    let count = |s: Standing| report.gates.iter().filter(|j| j.standing == s).count();
    // A met word the gate's `accept` leaves out is owed; counted only when
    // there is one, so trailers without a policy are unchanged.
    let refused = report.gates.iter().filter(|j| j.not_accepted()).count();
    let mut out = vec![
        ("gates", report.gates.len()),
        ("satisfied", count(Standing::Checked(State::Satisfied))),
        ("open", count(Standing::Checked(State::Open))),
        ("stale", count(Standing::Checked(State::Stale))),
        ("failed", count(Standing::Checked(State::Failed))),
        ("unknown", count(Standing::Checked(State::Unknown))),
        ("attested", count(Standing::Attested)),
        ("waived", count(Standing::Waived)),
        ("invalid", filings.invalid() + report.problems.len()),
    ];
    if refused > 0 {
        out.push(("not-accepted", refused));
    }
    out
}
