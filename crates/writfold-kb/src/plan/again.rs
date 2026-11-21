//! `plan again` (#76): record the next round of the current loop step, the
//! body read from the run's snapshot (never the live template), as one
//! more splice under the step. Admitted only as
//! [`writfold_core::plan::round::again_check`] decides; nothing is run.
use super::event;
use super::rounds::bounds;
use super::splice::{Planned, checked_write, recorded};
use super::until::Began;
use super::write::{answer, open_run};
use super::{Loaded, judge, template};
use crate::Request;
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use writfold_core::plan::round::{AgainRefusal, Bounds, again_check};
use writfold_core::plan::{GateFact, met_all};

/// The body of a loop step as the run spliced it: its snapshot.
fn body(ws: &Workspace, loaded: &Loaded, step: &super::fold::Step) -> Result<template::Closure> {
    let (name, _) = step
        .repeat
        .as_ref()
        .ok_or_else(|| KbError::Refused("not a loop".into()))?;
    let digest = (loaded.graph.uses.iter())
        .find(|u| u.parent == step.path && &u.template == name)
        .map(|u| u.digest.clone())
        .ok_or_else(|| {
            KbError::Refused(format!("loop {}: its first round is missing", step.path))
        })?;
    let hex = digest.strip_prefix("sha256:").unwrap_or(&digest);
    let file = ws
        .root
        .join(crate::runs::dir(&loaded.rec))
        .join(event::DIR)
        .join(event::TEMPLATES)
        .join(format!("{hex}.toml"));
    let bytes = std::fs::read(&file)?;
    let shown = format!("{name} (the run's snapshot {})", &hex[..hex.len().min(12)]);
    let t = template::parse_shown(&bytes, Some(name), &shown)?;
    let repo = loaded
        .subject
        .as_ref()
        .map(|s| (s.top.as_path(), s.name.as_str()));
    template::closure(&ws.root, repo, t, shown)
}

/// The refusal of `plan again`, in words.
fn refused(why: AgainRefusal, step: &str, rounds: usize, b: &Bounds) -> KbError {
    KbError::Refused(match why {
        AgainRefusal::Complete => "not repeated: every step is closed".into(),
        AgainRefusal::NotCurrent(_) | AgainRefusal::NotLoop => {
            format!("not repeated: {step} is the current step and is not a loop")
        }
        AgainRefusal::Met => format!(
            "not repeated: {step} owes nothing more (round {rounds} of at most {}): writfold kb plan advance",
            b.max
        ),
        AgainRefusal::Spent => format!("not repeated: {step} has used its {} rounds", b.max),
    })
}

pub(super) fn again(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    (run, repo): (Option<&str>, Option<&std::path::Path>),
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let loaded = Loaded::open(&session, run, repo, &request.cwd)?;
    open_run(&loaded, &session)?;
    let graph = &loaded.graph;
    let at = graph.current().unwrap_or(graph.steps.len());
    let step = graph.steps.get(at);
    let b = step.and_then(bounds);
    let gates: Vec<judge::Gate> = match step {
        Some(s) if !s.gates.is_empty() && b.is_some() => loaded
            .judged(&session)?
            .and_then(|mut all| (at < all.len()).then(|| all.swap_remove(at)))
            .ok_or_else(|| KbError::Refused("no subject repository to judge the loop's gates: run inside one, or pass --repo DIR".into()))?,
        _ => Vec::new(),
    };
    let facts: Vec<GateFact> = gates.iter().map(judge::Gate::fact).collect();
    let rounds = step.map_or(0, |s| graph.rounds(s));
    let looped = b.is_some();
    let bound = b.unwrap_or(Bounds { min: 1, max: 1 });
    let path = step.map_or_else(String::new, |s| s.path.clone());
    let n = again_check(&graph.nodes(), at, looped, &bound, rounds, &met_all(&facts))
        .map_err(|why| refused(why, &path, rounds, &bound))?;
    let Some(step) = step else {
        return Err(refused(AgainRefusal::Complete, &path, rounds, &bound));
    };
    let closure = body(ws, &loaded, step)?;
    let planned = Planned {
        parent: step.path.clone(),
        ordinal: graph.next_ordinal(&step.path),
        closure,
        again: true,
        until: (gates.iter())
            .map(|g| Began {
                gate: g.id.clone(),
                standing: g.display(),
                evidence: g.evidence.clone(),
            })
            .collect(),
    };
    let decision = super::place::decided(graph, &planned);
    let mut notes = vec![format!(
        "round {n} of at most {} under {}",
        bound.max, step.path
    )];
    let run_id = loaded.rec.id.clone();
    let planned = [planned];
    drop(session);
    let paths = checked_write(ws, request, &loaded, (&planned, decision), now)?;
    notes.extend(recorded(paths));
    answer(ws, tool, request, ("plan-again", &run_id, repo), notes, now)
}
