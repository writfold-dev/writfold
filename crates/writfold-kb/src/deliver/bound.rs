//! The built-in check `invocation.bound` (#149): a worker's report is a
//! delivery candidate that names its invocation (`deliver prepare
//! --invocation`). It passes only while the candidate names an invocation
//! of this run, prepared at the step that owes this mode (the step the
//! candidate recorded when it was prepared, #164, so the answer does not
//! change when the plan moves on), for the mode's
//! `agent`, under the closure as it is now, with no second report for it
//! ([`writfold_core::invoke::bound`]). An unreadable record is `unknown`.
//! It judges records, never what the report says.
//!
//! On every read a passed receipt is judged again for what can move after
//! it: the closure (an edited profile, skill, tool or instructions file)
//! and a second report. Either makes it `stale` ([`recheck`]).
//! Gathering the facts is tested IO; the decision is the proved kernel.
use super::candidate::{self, Candidate};
use super::checks::{Evaluated, Finding};
use super::status::Judged;
use super::withdraw;
use crate::agents::invocation::{self, Record};
use crate::index::query::Rec;
use crate::session::Session;
use writfold_core::delivery::rechecked;
use writfold_core::gate::{Outcome, State, standing};
use writfold_core::invoke::{Named, bound};

/// The run directory a candidate is filed under.
fn run_dir(c: &Candidate) -> &str {
    let up = c.dir.rsplit_once('/').map_or("", |(d, _)| d);
    up.rsplit_once('/').map_or("", |(d, _)| d)
}

fn record(session: &Session<'_>, c: &Candidate, id: &str) -> Result<Record, String> {
    let m = c.manifest.as_ref().map_err(Clone::clone)?;
    let path = invocation::path(run_dir(c), id);
    let text = std::fs::read_to_string(session.ws.root.join(&path))
        .map_err(|e| format!("{path} unreadable ({e})"))?;
    invocation::parse(&text, &m.run, id)
}

/// The closure digest of profile `agent` now, with the run's repository.
fn closure_now(session: &Session<'_>, rec: &Rec, agent: &str) -> Option<String> {
    let subject = crate::gate::locate(session, rec, None, &session.ws.root).ok()?;
    let top = subject.as_ref().map(|s| s.top.as_path());
    let t = crate::agents::inspect::table_at(&session.ws.root, top);
    crate::agents::inspect::decide(&t, agent).1.ok()
}

/// Whether every live candidate naming `id` holds the same bytes.
fn single(session: &Session<'_>, c: &Candidate, id: &str, run: &str) -> bool {
    let root = &session.ws.root;
    let cands = candidate::list(root, run_dir(c), run);
    let withdrawn = withdraw::ids(root, &cands, run);
    invocation::single(&invocation::naming(&cands, &withdrawn, id))
}

/// The step that owes `deliver:<mode>` for this candidate, when it does:
/// the step recorded in the candidate (#164), or, for a candidate written
/// before it recorded one, the run's current step.
fn owing(session: &Session<'_>, c: &Candidate, run: &str, mode: &str) -> Option<String> {
    let graph = crate::plan::fold::read(&session.ws.root, run_dir(c), run);
    let recorded = c.manifest.as_ref().ok().and_then(|m| m.step.as_deref());
    let step = match recorded {
        Some(at) => graph.steps.iter().find(|s| s.path == at),
        None => graph.current().and_then(|i| graph.steps.get(i)),
    }?;
    let word = format!("deliver:{mode}");
    step.gates.contains(&word).then(|| step.path.clone())
}

/// Every fact of `c`, and the record when it read.
fn named(
    session: &Session<'_>,
    rec: &Rec,
    c: &Candidate,
    agent: Option<&str>,
) -> (Named, Option<Record>) {
    let m = c.manifest.as_ref().ok();
    let id = m.and_then(|m| m.invocation.clone());
    let Some(id) = id else {
        return (
            Named {
                names: false,
                readable: false,
                this_run: false,
                this_step: false,
                this_agent: false,
                closure_current: false,
                single: false,
            },
            None,
        );
    };
    let found = record(session, c, &id).ok();
    let run = m.map_or("", |m| m.run.as_str());
    let mode = m.map_or("", |m| m.mode.as_str());
    let facts = match &found {
        None => Named {
            names: true,
            readable: false,
            this_run: false,
            this_step: false,
            this_agent: false,
            closure_current: false,
            single: false,
        },
        Some(r) => Named {
            names: true,
            readable: true,
            this_run: r.run == rec.id,
            this_step: owing(session, c, run, mode).as_deref() == Some(r.step.as_str()),
            this_agent: agent == Some(r.agent.as_str()),
            closure_current: closure_now(session, rec, &r.agent).as_deref()
                == Some(r.closure.as_str()),
            single: single(session, c, &id, run),
        },
    };
    (facts, found)
}

fn finding(rule: &str, detail: String) -> Finding {
    Finding {
        line: 1,
        rule: rule.into(),
        detail,
    }
}

/// What `another-step` says: both steps when the candidate recorded one.
fn other_step(r: Option<&Record>, step: Option<&str>) -> String {
    let at = r.map_or("", |r| r.step.as_str());
    match step {
        Some(s) if s != at => format!(
            "the invocation was prepared at {at}, and this report at {s}; a report binds only at its invocation's step"
        ),
        Some(s) => format!("the report was prepared at {s}, which does not owe this mode"),
        None => format!("the invocation was prepared at {at}, not at the step that owes this mode"),
    }
}

/// One finding per fact that does not hold.
fn findings(n: &Named, r: Option<&Record>, step: Option<&str>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (holds, rule, detail) in [
        (
            n.names,
            "no-invocation",
            "the candidate names no invocation; prepare it with --invocation ID".to_owned(),
        ),
        (
            !n.names || n.readable,
            "unreadable",
            "the invocation record does not read".to_owned(),
        ),
        (
            !n.readable || n.this_run,
            "another-run",
            "the invocation is of another run".to_owned(),
        ),
        (
            !n.readable || n.this_step,
            "another-step",
            other_step(r, step),
        ),
        (
            !n.readable || n.this_agent,
            "another-agent",
            format!(
                "the invocation is of agent {}, not this mode's agent",
                r.map_or("", |r| r.agent.as_str())
            ),
        ),
        (
            !n.readable || n.closure_current,
            "closure-moved",
            "the profile's closure moved since the invocation was prepared".to_owned(),
        ),
        (
            !n.readable || n.single,
            "two-reports",
            "another live candidate names this invocation with other bytes; withdraw one"
                .to_owned(),
        ),
    ] {
        if !holds {
            findings.push(finding(rule, detail));
        }
    }
    findings
}

/// `invocation.bound` for candidate `c` under a mode whose `agent` is `agent`.
pub fn evaluate(session: &Session<'_>, rec: &Rec, c: &Candidate, agent: Option<&str>) -> Evaluated {
    let (n, found) = named(session, rec, c, agent);
    let r = found.as_ref();
    let step = c.manifest.as_ref().ok().and_then(|m| m.step.as_deref());
    let findings = findings(&n, r, step);
    let outcome = bound(&n);
    let note = r.map(|r| format!("invocation {} of {} at {}", r.id, r.agent, r.step));
    let (findings, note) = match outcome {
        Outcome::Unknown => (
            Vec::new(),
            Some(
                findings
                    .first()
                    .map_or_else(String::new, |f| f.detail.clone()),
            ),
        ),
        _ => (findings, note),
    };
    Evaluated {
        outcome,
        findings,
        advice: Vec::new(),
        note,
    }
}

/// Judge a passed `invocation.bound` receipt again: the closure as it is
/// now, and no second report.
pub fn recheck(session: &Session<'_>, c: &Candidate, judged: &mut [Judged<'_>]) {
    let Some(j) = (judged.iter_mut())
        .find(|j| j.obligation.name == super::config::BOUND && j.obligation.gate.is_none())
        .filter(|j| j.checked == State::Satisfied)
    else {
        return;
    };
    let Ok(m) = &c.manifest else {
        return;
    };
    let Some(id) = &m.invocation else {
        return;
    };
    let Ok(rec) = crate::write::run::target(session, Some(&m.run)) else {
        return;
    };
    let why = match record(session, c, id) {
        Err(_) => Some("the invocation record no longer reads"),
        Ok(r) if closure_now(session, &rec, &r.agent).as_deref() != Some(r.closure.as_str()) => {
            Some("the profile's closure moved")
        }
        Ok(_) if !single(session, c, id, &m.run) => Some("a second report names the invocation"),
        Ok(_) => None,
    };
    if let Some(why) = why {
        j.checked = rechecked(j.checked, 1);
        j.standing = standing(false, j.checked, j.statements.statements);
        j.unbound = Some(why);
    }
}
