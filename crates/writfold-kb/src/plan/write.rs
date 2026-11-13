//! The plan's closing writes, each one create-new event through the write
//! protocol ([`crate::write::atomic`]): `plan advance` (a `done-` record naming what met each gate) and `plan
//! skip` (a `skip-` record with a reason); the splices are in
//! [`super::splice`]. An event's file name follows
//! from what it is about, so two sessions cannot both advance one step:
//! the second finds the file and is refused as a write conflict.
use super::event::{self, Done, Event, Met, Skip};
use super::recheck::{Decided, guard};
use super::{Loaded, PlanVerb, judge, required, show};
use crate::Request;
use crate::emit::{Answer, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic};
use crate::{runs, time};
use writfold_core::plan::{Refusal, advance_check};

pub(super) fn open_run(loaded: &Loaded, session: &Session<'_>) -> Result<()> {
    match runs::state(session, &loaded.rec) == "closed" {
        true => Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            loaded.rec.id, loaded.rec.id
        ))),
        false => Ok(()),
    }
}

pub(super) fn apply(ws: &Workspace, request: &Request, ops: &[Op]) -> Result<()> {
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("plans need a KB under Git".into()))?;
    atomic::apply(&ws.root, &state, ops, request.probe)
}

/// Answer after a write: the plan as it now stands.
pub(super) fn answer(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    (verb, run, repo): (&str, &str, Option<&std::path::Path>),
    notes: Vec<String>,
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let loaded = Loaded::open(&session, Some(run), repo, &request.cwd)?;
    let mut answer = super::status(&session, &loaded)?;
    answer.notes.splice(0..0, notes);
    answer
        .notes
        .push("written to the KB work tree; commit it to share it".into());
    let frame = crate::gate::frame(request, verb, vec![run.to_owned()]);
    session.emit(&answer, &frame)
}

/// `plan advance`: the current step's gates, judged now, each met, or a
/// refusal (exit 3) naming what it owes, first owed gate first.
pub(super) fn advance(
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
    if !graph.planned() {
        return Err(KbError::Refused(format!(
            "run {} has no plan: writfold kb plan set <template>",
            loaded.rec.id
        )));
    }
    let at = graph.current().unwrap_or(graph.steps.len());
    let step = graph.steps.get(at);
    let gates: Vec<judge::Gate> = match step {
        Some(s) if !s.gates.is_empty() => loaded
            .judged(&session)?
            .and_then(|mut all| (at < all.len()).then(|| all.swap_remove(at)))
            .ok_or_else(|| KbError::Refused("no subject repository to judge the step's gates: run inside one, or pass --repo DIR".into()))?,
        _ => Vec::new(),
    };
    let met: Vec<bool> = gates.iter().map(judge::Gate::met).collect();
    let step = match (advance_check(&graph.nodes(), at, &met), step) {
        (Ok(()), Some(step)) => step,
        (Err(Refusal::Owed(first)), Some(step)) => {
            let frame = crate::gate::frame(request, "plan-advance", vec![loaded.rec.id.clone()]);
            return session.emit(&refused(step, &gates, first), &frame);
        }
        _ => {
            return Err(KbError::Refused(format!(
                "run {} has no current step: every step is done or skipped",
                loaded.rec.id
            )));
        }
    };
    complete_guard(ws, &loaded, at)?;
    let done = done(step, &gates, &loaded, now);
    let path = format!(
        "{}/{}/{}",
        runs::dir(&loaded.rec),
        event::DIR,
        event::name(&done)
    );
    let decided = Decided::Current(step.path.clone());
    let ops = [
        guard(&runs::dir(&loaded.rec), &loaded.rec.id, decided),
        Op::Create {
            path: path.clone(),
            bytes: event::bytes(&loaded.rec.id, &done),
        },
    ];
    let run_id = loaded.rec.id.clone();
    drop(session);
    apply(ws, request, &ops)?;
    let notes = vec![format!("recorded {path}")];
    answer(
        ws,
        tool,
        request,
        ("plan-advance", &run_id, repo),
        notes,
        now,
    )
}

/// Refuse to close the plan's last open step while a required end
/// template is missing from it (the required file changed after the plan
/// was set).
pub(super) fn complete_guard(ws: &Workspace, loaded: &Loaded, at: usize) -> Result<()> {
    let mut nodes = loaded.graph.nodes();
    if let Some(node) = nodes.get_mut(at) {
        node.done = true;
    }
    if writfold_core::plan::current(&nodes).is_some() {
        return Ok(());
    }
    let repo = loaded
        .subject
        .as_ref()
        .map(|s| (s.top.as_path(), s.name.as_str()));
    let required = required::find(&ws.root, repo)?;
    match required::missing_end(&required, &loaded.graph).first() {
        None => Ok(()),
        Some(name) => Err(KbError::Refused(format!(
            "not advanced: the plan cannot complete without its required end step {name} ({}); add it with writfold kb plan use {name}",
            required.source.as_deref().unwrap_or("required.toml")
        ))),
    }
}

/// The answer of a refused advance (exit 3): the step and what it owes.
fn refused(step: &super::fold::Step, gates: &[judge::Gate], first: usize) -> Answer {
    let mut answer = Answer {
        refused: true,
        rows: vec![show::step_row(step, true, gates, true)],
        ..Answer::default()
    };
    answer.notes.push(format!(
        "not advanced: {} owes {} (first owed: {}); nothing written",
        step.path,
        show::listed(&gates.iter().filter(|g| !g.met()).collect::<Vec<_>>()),
        gates.get(first).map_or("?", |g| g.id.as_str())
    ));
    answer
}

/// The `done-` event: what met each gate, and the run's records now.
fn done(step: &super::fold::Step, gates: &[judge::Gate], loaded: &Loaded, now: i64) -> Event {
    Event::Done(Done {
        node: step.path.clone(),
        met: gates
            .iter()
            .map(|g| Met {
                gate: g.id.clone(),
                standing: g.shown().into(),
                evidence: g.evidence.clone(),
                definition: g.definition.clone(),
                content: g.content.clone(),
            })
            .collect(),
        records: loaded.records.baseline(),
        recorded: time::stamp(now),
    })
}

/// `plan skip --reason --by`: close the current step without calling it done.
pub(super) fn skip(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    verb: &PlanVerb,
    now: i64,
) -> Result<Output> {
    let PlanVerb::Skip {
        reason,
        by,
        run,
        repo,
    } = verb
    else {
        return Err(KbError::Usage("plan skip".into()));
    };
    let reason = crate::gate::statement::check_text("--reason", reason)?;
    let by = crate::gate::statement::check_text("--by", by)?;
    let session = Session::open(ws, Some(tool), false, now)?;
    let loaded = Loaded::open(&session, run.as_deref(), repo.as_deref(), &request.cwd)?;
    open_run(&loaded, &session)?;
    let Some(at) = loaded.graph.current() else {
        return Err(KbError::Refused(format!(
            "run {} has no current step to skip",
            loaded.rec.id
        )));
    };
    complete_guard(ws, &loaded, at)?;
    let step = &loaded.graph.steps[at];
    let skip = Event::Skip(Skip {
        node: step.path.clone(),
        reason,
        by,
        records: loaded.records.baseline(),
        recorded: time::stamp(now),
    });
    let path = format!(
        "{}/{}/{}",
        runs::dir(&loaded.rec),
        event::DIR,
        event::name(&skip)
    );
    let decided = Decided::Current(step.path.clone());
    let ops = [
        guard(&runs::dir(&loaded.rec), &loaded.rec.id, decided),
        Op::Create {
            path: path.clone(),
            bytes: event::bytes(&loaded.rec.id, &skip),
        },
    ];
    let run_id = loaded.rec.id.clone();
    drop(session);
    apply(ws, request, &ops)?;
    let notes = vec![format!("recorded {path} (skipped, not done)")];
    answer(
        ws,
        tool,
        request,
        ("plan-skip", &run_id, repo.as_deref()),
        notes,
        now,
    )
}
