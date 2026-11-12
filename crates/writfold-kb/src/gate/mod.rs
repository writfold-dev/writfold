//! G1: executable checks with pinned receipts (`writfold kb gate`).
//!
//! A gate is a command named in a small TOML definition ([`def`]). `gate
//! run` executes it on a subject repository and records one receipt in the
//! KB run ([`receipt`]); `gate status` derives each gate's obligation state
//! from the definitions and the receipts ([`status`]); `gate verify`
//! re-runs a receipt's command at its commit in a temporary worktree; `gate
//! lines` prints the receipt lines for a commit message. State is never
//! stored. The claims are mistake resistance and Git tamper evidence: a
//! receipt is a tracked file whose history Git keeps, and anyone can
//! re-run it. Nothing here signs anything or stops a determined forger.
pub mod def;
pub mod exec;
pub mod receipt;
mod run;
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
use std::path::{Path, PathBuf};
use subject::Subject;

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
}

/// Answer one gate verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &GateVerb, now: i64) -> Result<Output> {
    if ws.repo.is_none() {
        return Err(KbError::Refused(
            "gates need a KB under Git (receipts are tracked files)".into(),
        ));
    }
    let tool = GitTool::locate()?;
    match verb {
        GateVerb::Run { id, run, repo } => {
            run::run(ws, &tool, request, id, run.as_deref(), repo.as_deref(), now)
        }
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
            lines(&session, &rec, repo.as_deref(), &request.cwd)
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
    let (rows, counts) = gate_rows(session, rec, &subject)?;
    if rows.is_empty() {
        answer.unknown = Some("no-gates");
        answer.notes.push(format!(
            "no gates defined: add {}/<id>.toml to the KB root or {}",
            def::DIR,
            subject.name
        ));
    }
    answer.rows = rows;
    answer.counts = counts;
    answer.notes.push(format!(
        "receipts of run {} for {}; state is derived on every read, never stored",
        rec.id, subject.name
    ));
    Ok((answer, subject.name))
}

/// Trailer counts by state.
type Counts = Vec<(&'static str, usize)>;

fn gate_rows(session: &Session<'_>, rec: &Rec, subject: &Subject) -> Result<(Vec<Row>, Counts)> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let filed = status::filed(&session.ws.root, &runs::dir(rec), &rec.id);
    let report = status::judge(git, &session.ws.root, subject, &filed)?;
    let rows = status::rows(git, subject, &report, &filed)?;
    Ok((rows, status::counts(&report, &filed)))
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
        return match defined.is_empty() {
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
    if defined.is_empty() && problems.is_empty() {
        return (Vec::new(), Vec::new());
    }
    match gate_rows(session, rec, &subject) {
        Ok((rows, _)) => (rows, Vec::new()),
        Err(error) => (Vec::new(), vec![format!("gates: {error}")]),
    }
}

/// `gate lines`: plain lines to paste into a commit message, exit 0 only
/// when every gate is satisfied.
fn lines(session: &Session<'_>, rec: &Rec, repo: Option<&Path>, cwd: &Path) -> Result<Output> {
    let Some(subject) = locate(session, rec, repo, cwd)? else {
        return Err(KbError::Refused(
            "no subject repository: run inside one, or pass --repo DIR".into(),
        ));
    };
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let filed = status::filed(&session.ws.root, &runs::dir(rec), &rec.id);
    let report = status::judge(git, &session.ws.root, &subject, &filed)?;
    let satisfied = report
        .gates
        .iter()
        .filter(|j| j.state == writfold_core::gate::State::Satisfied)
        .count();
    let head = report.snapshot.commit.as_str();
    let mut text = format!(
        "Gates for {}@{} (writfold kb gate, run {}): {satisfied} of {} satisfied\n",
        subject.name,
        head.get(..12).unwrap_or(head),
        rec.id,
        report.gates.len()
    );
    let width = report
        .gates
        .iter()
        .map(|j| j.definition.id.len())
        .max()
        .unwrap_or(0);
    for judged in &report.gates {
        let id = format!("{}:", judged.definition.id);
        let state = judged.state.name();
        match judged.receipt {
            Some((f, r)) => text.push_str(&format!(
                "{id:<w$} {state} {} {} {}\n",
                r.outcome.name(),
                receipt::ended(r),
                f.path,
                w = width + 1
            )),
            None => text.push_str(&format!("{id:<w$} {state}\n", w = width + 1)),
        }
    }
    let invalid = filed.iter().filter(|f| f.parsed.is_err()).count() + report.problems.len();
    if invalid > 0 {
        text.push_str(&format!(
            "invalid receipts or definitions: {invalid} (see writfold kb gate status)\n"
        ));
    }
    let all = satisfied == report.gates.len() && !report.gates.is_empty() && invalid == 0;
    Ok(Output {
        text,
        exit: if all { 0 } else { 2 },
    })
}
