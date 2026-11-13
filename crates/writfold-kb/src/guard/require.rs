//! Gathering the facts a matched guard rule requires: the current run,
//! the gates it owes (judged as `gate status` judges them) and the plan's
//! current step. This is the only part of a guard rule that opens the
//! index or spawns Git, and it runs only after a rule matched. When the KB
//! cannot be read, every requirement is [`Fact::Unknown`], which takes the
//! rule's action ([`writfold_core::guard::decide`]).
use super::rules::{Require, Rule};
use crate::gate::status::{self, Filings};
use crate::gate::subject::{self, Subject};
use crate::index::query::{self, Rec};
use crate::plan::{self, Loaded};
use crate::runs;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use std::path::Path;
use writfold_core::guard::Fact;
use writfold_core::scope::in_scope;

/// One requirement's fact, and (when it is not met) what is owed and the
/// command that would meet it.
#[derive(Clone, Debug)]
pub struct Owed {
    pub fact: Fact,
    pub text: String,
}

impl Owed {
    fn met() -> Self {
        Self {
            fact: Fact::Met,
            text: String::new(),
        }
    }

    fn unmet(text: String) -> Self {
        Self {
            fact: Fact::Unmet,
            text,
        }
    }

    fn unknown(text: String) -> Self {
        Self {
            fact: Fact::Unknown,
            text,
        }
    }
}

/// For each rule: `None` when its `repos` leave this repository out,
/// else one [`Owed`] per requirement, in order.
pub type Evaluated = Vec<Option<Vec<Owed>>>;

const START: &str = "start one: writfold kb checkpoint --new-run --project P --task T --note TEXT";

/// One gate as it stands: whether it is met, and its line.
struct GateLine {
    id: String,
    met: bool,
    line: String,
}

/// Every judged gate, the gates the run owes (`None`: every one), and
/// the count of invalid filings and definitions.
type Judged = (Vec<GateLine>, Option<Vec<String>>, usize);

/// What every requirement is looked up in, gathered once.
struct Facts {
    run: Result<Rec, String>,
    /// The run's effective state (`open`, `closed`, …).
    state: String,
    subject: Result<Option<Subject>, String>,
    /// The judged gates; `None` until a rule asks.
    gates: Option<Result<Judged, String>>,
    step: Option<Owed>,
}

/// Evaluate every rule's requirements.
pub fn evaluate(ws: &Workspace, cwd: &Path, now: i64, rules: &[&Rule]) -> Evaluated {
    let unknown = |why: String| -> Evaluated {
        rules
            .iter()
            .map(|r| {
                Some(
                    r.require
                        .iter()
                        .map(|_| Owed::unknown(why.clone()))
                        .collect(),
                )
            })
            .collect()
    };
    let tool = match GitTool::locate() {
        Ok(tool) => tool,
        Err(e) => return unknown(format!("cannot read the KB ({e})")),
    };
    let session = match Session::open(ws, Some(&tool), false, now) {
        Ok(session) => session,
        Err(e) => return unknown(format!("cannot read the KB ({e})")),
    };
    let facts = gather(&session, cwd, rules);
    rules
        .iter()
        .map(|rule| {
            let name = match (&facts.subject, rule.repos.is_empty()) {
                (_, true) => None,
                (Ok(Some(s)), false) => Some(s.name.as_str()),
                (Ok(None), false) => return None,
                (Err(_), false) => None,
            };
            if let Some(name) = name {
                let patterns: Vec<&str> = rule.repos.iter().map(String::as_str).collect();
                if !in_scope(&patterns, name) {
                    return None;
                }
            }
            Some(rule.require.iter().map(|q| fact(&facts, q)).collect())
        })
        .collect()
}

fn gather(session: &Session<'_>, cwd: &Path, rules: &[&Rule]) -> Facts {
    let run = match runs::current_of(session) {
        None => Err(format!("no current KB run; {START}")),
        Some(id) => match query::record(&session.conn, &id) {
            Ok(Some(rec)) if rec.kind == "run" => Ok(rec),
            Ok(_) => Err(format!(
                "the current-run pointer names {id}, which is not a run in the KB; {START}"
            )),
            Err(e) => Err(format!("cannot read the KB ({e})")),
        },
    };
    let subject = match &session.git {
        None => Err("the KB is not under Git".to_owned()),
        Some(git) => {
            let recorded = run
                .as_ref()
                .ok()
                .and_then(|rec| crate::gate::recorded(session, rec));
            subject::locate(session.ws, git, None, cwd, recorded.as_deref())
                .map_err(|e| format!("cannot locate the repository ({e})"))
        }
    };
    let wants = |f: fn(&Require) -> bool| rules.iter().any(|r| r.require.iter().any(f));
    let state = run
        .as_ref()
        .map(|rec| runs::state(session, rec))
        .unwrap_or_default();
    let mut facts = Facts {
        run,
        state,
        subject,
        gates: None,
        step: None,
    };
    if wants(|q| matches!(q, Require::GatesMet | Require::Gate(_))) {
        facts.gates = Some(judge(session, &facts));
    }
    if wants(|q| matches!(q, Require::StepMet)) {
        facts.step = Some(step(session, &facts, cwd));
    }
    facts
}

/// Judge every defined gate of the run on the subject, as `gate status`.
fn judge(session: &Session<'_>, facts: &Facts) -> Result<Judged, String> {
    let rec = facts.run.as_ref().map_err(String::clone)?;
    let subject = match &facts.subject {
        Ok(Some(subject)) => subject,
        Ok(None) => return Err("no subject repository here".into()),
        Err(why) => return Err(why.clone()),
    };
    let git = session.git.as_ref().ok_or("gates need Git")?;
    let mut filings = Filings::read(&session.ws.root, &runs::dir(rec), &rec.id);
    let owed = plan::owed(session, rec);
    filings.owed.records = owed.records;
    let unread = |e: crate::error::KbError| format!("cannot judge the gates ({e})");
    let report = status::judge(session, subject, &filings).map_err(unread)?;
    let rows = status::rows(git, subject, &report, &filings).map_err(unread)?;
    let lines = report
        .gates
        .iter()
        .map(|j| {
            let id = j.definition.id.clone();
            let next = rows
                .iter()
                .find(|r| r.id == id && r.status.as_deref() != Some("invalid"))
                .and_then(|r| r.more.iter().find(|m| m.label == "next"))
                .map(|m| format!("; next: {}", m.text))
                .unwrap_or_default();
            GateLine {
                line: format!("gate {id} [{}]{next}", j.standing.name()),
                met: j.standing.met(),
                id,
            }
        })
        .collect();
    let invalid = filings.invalid() + report.problems.len();
    Ok((lines, owed.gates, invalid))
}

fn undefined(id: &str) -> String {
    format!(
        "gate {id} [undefined]; next: define {}/{id}.toml in the KB root or the repository",
        crate::gate::def::DIR
    )
}

/// The plan's current step: met when it owes nothing (or there is no
/// plan, or every step is closed).
fn step(session: &Session<'_>, facts: &Facts, cwd: &Path) -> Owed {
    let rec = match &facts.run {
        Ok(rec) => rec,
        Err(why) => return Owed::unmet(why.clone()),
    };
    let loaded = match Loaded::open(session, Some(&rec.id), None, cwd) {
        Ok(loaded) => loaded,
        Err(e) => return Owed::unknown(format!("cannot read the plan ({e})")),
    };
    let Some(at) = loaded.graph.current().filter(|_| loaded.graph.planned()) else {
        return Owed::met();
    };
    let judged = match loaded.judged(session) {
        Ok(Some(judged)) => judged,
        Ok(None) => return Owed::unknown("no subject repository here to judge the step".into()),
        Err(e) => return Owed::unknown(format!("cannot judge the step ({e})")),
    };
    let gates = judged.get(at).map(Vec::as_slice).unwrap_or_default();
    let owed: Vec<&plan::judge::Gate> = gates.iter().filter(|g| !g.met()).collect();
    let Some(first) = owed.first() else {
        return Owed::met();
    };
    let name = &loaded.graph.steps[at].id;
    Owed::unmet(format!(
        "step {name} owes {}; next: {}",
        plan::show::listed(&owed),
        plan::show::next(first)
    ))
}

fn fact(facts: &Facts, require: &Require) -> Owed {
    match require {
        Require::RunCurrent => match &facts.run {
            Err(why) => Owed::unmet(why.clone()),
            Ok(_) if facts.state == "open" => Owed::met(),
            Ok(rec) => Owed::unmet(format!("run {} is {}; {START}", rec.id, facts.state)),
        },
        Require::StepMet => facts
            .step
            .clone()
            .unwrap_or_else(|| Owed::unknown("the step was not judged".into())),
        Require::GatesMet => gates_met(facts),
        Require::Gate(id) => match &facts.gates {
            None => Owed::unknown("the gates were not judged".into()),
            Some(Err(why)) => Owed::unknown(why.clone()),
            Some(Ok((lines, _, _))) => match lines.iter().find(|g| &g.id == id) {
                Some(g) if g.met => Owed::met(),
                Some(g) => Owed::unmet(g.line.clone()),
                None => Owed::unmet(undefined(id)),
            },
        },
    }
}

/// Every owed gate met, and no invalid receipt, statement or definition.
fn gates_met(facts: &Facts) -> Owed {
    let (lines, owed, invalid) = match &facts.gates {
        None => return Owed::unknown("the gates were not judged".into()),
        Some(Err(why)) => return Owed::unknown(why.clone()),
        Some(Ok(judged)) => judged,
    };
    let mut unmet: Vec<String> = Vec::new();
    match owed {
        None => unmet.extend(lines.iter().filter(|g| !g.met).map(|g| g.line.clone())),
        Some(ids) => {
            for id in ids {
                match lines.iter().find(|g| &g.id == id) {
                    Some(g) if g.met => {}
                    Some(g) => unmet.push(g.line.clone()),
                    None => unmet.push(undefined(id)),
                }
            }
        }
    }
    if *invalid > 0 {
        unmet.push(format!(
            "invalid receipts or definitions: {invalid}; next: writfold kb gate status"
        ));
    }
    match unmet.is_empty() {
        true => Owed::met(),
        false => Owed::unmet(unmet.join("; ")),
    }
}
