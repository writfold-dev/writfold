//! The plan's three writes, each one create-new event through the write
//! protocol ([`crate::write::atomic`]): `plan set` (the template snapshot,
//! then the root splice, so a splice never exists without its snapshot),
//! `plan advance` (a `done-` record naming what met each gate) and `plan
//! skip` (a `skip-` record with a reason). An event's file name follows
//! from what it is about, so two sessions cannot both advance one step:
//! the second finds the file and is refused as a write conflict.
use super::event::{self, Done, Event, Met, Skip, Splice};
use super::template::{self, Template};
use super::{Loaded, PlanVerb, judge, show};
use crate::Request;
use crate::emit::{Answer, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic};
use crate::{runs, time};
use writfold_core::plan::{Refusal, advance_check};

fn open_run(loaded: &Loaded, session: &Session<'_>) -> Result<()> {
    match runs::state(session, &loaded.rec) == "closed" {
        true => Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            loaded.rec.id, loaded.rec.id
        ))),
        false => Ok(()),
    }
}

fn apply(ws: &Workspace, request: &Request, ops: &[Op]) -> Result<()> {
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("plans need a KB under Git".into()))?;
    atomic::apply(&ws.root, &state, ops, request.probe)
}

/// Answer after a write: the plan as it now stands.
fn answer(
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

/// The template a `plan set` names, and where it came from.
fn chosen(
    loaded: &Loaded,
    ws: &Workspace,
    request: &Request,
    verb: &PlanVerb,
) -> Result<(Template, String)> {
    let PlanVerb::Set { template, from, .. } = verb else {
        return Err(KbError::Usage("plan set".into()));
    };
    let repo = loaded
        .subject
        .as_ref()
        .map(|s| (s.top.as_path(), s.name.as_str()));
    match (template, from) {
        (Some(name), None) => template::find(&ws.root, repo, name),
        (None, Some(file)) => {
            let bytes = std::fs::read(request.cwd.join(file))?;
            let shown = file
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            Ok((template::parse(&bytes, None)?, format!("--from {shown}")))
        }
        _ => Err(KbError::Usage(
            "plan set takes a template name or --from FILE".into(),
        )),
    }
}

/// Refuse a template that names a gate no definition provides.
fn defined(ws: &Workspace, loaded: &Loaded, t: &Template) -> Result<()> {
    let top = loaded.subject.as_ref().map(|s| s.top.as_path());
    let (found, problems) = crate::gate::def::all(&ws.root, top);
    for gate in t.steps.iter().flat_map(|s| &s.gates) {
        let file = format!("{}/{gate}.toml", crate::gate::def::DIR);
        if let Some((_, path, why)) = problems.iter().find(|(_, p, _)| *p == file) {
            return Err(KbError::Refused(format!("gate {gate}: {path}: {why}")));
        }
        match found.iter().find(|d| &d.id == gate) {
            Some(d) if d.on_candidate() => {
                return Err(KbError::Refused(format!(
                    "gate {gate} judges delivery candidates; a plan step owes repository and run-record gates"
                )));
            }
            Some(_) => {}
            None => {
                return Err(KbError::Refused(format!(
                    "plan {} names gate {gate}, which is not defined: add {file} to the KB root or the repository",
                    t.name
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn set(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    verb: &PlanVerb,
    now: i64,
) -> Result<Output> {
    let PlanVerb::Set { run, repo, .. } = verb else {
        return Err(KbError::Usage("plan set".into()));
    };
    let session = Session::open(ws, Some(tool), false, now)?;
    let loaded = Loaded::open(&session, run.as_deref(), repo.as_deref(), &request.cwd)?;
    open_run(&loaded, &session)?;
    let (t, source) = chosen(&loaded, ws, request, verb)?;
    let run_id = loaded.rec.id.clone();
    let place = (("plan-set"), run_id.as_str(), repo.as_deref());
    if let Some((_, s, _)) = loaded
        .graph
        .splices
        .iter()
        .find(|(_, s, _)| s.parent.is_empty())
    {
        if s.template == t.name && s.digest == t.digest {
            drop(session);
            let note = format!(
                "plan {} is already set on this run; nothing written",
                t.name
            );
            return answer(ws, tool, request, place, vec![note], now);
        }
        return Err(KbError::Refused(format!(
            "run {run_id} already has a plan ({}); a second plan at the root is not built: skip the remaining steps with plan skip, or start another run",
            s.template
        )));
    }
    defined(ws, &loaded, &t)?;
    let dir = format!("{}/{}", runs::dir(&loaded.rec), event::DIR);
    let hex = t.digest.trim_start_matches("sha256:");
    let snapshot = format!("{dir}/{}/{hex}.toml", event::TEMPLATES);
    let mut ops = Vec::new();
    match std::fs::read(ws.root.join(&snapshot)) {
        Ok(bytes) if bytes == t.bytes => {}
        Ok(_) => {
            return Err(KbError::Refused(format!(
                "{snapshot} exists with other bytes"
            )));
        }
        Err(_) => ops.push(Op::Create {
            path: snapshot,
            bytes: t.bytes.clone(),
        }),
    }
    let splice = Event::Splice(Splice {
        parent: String::new(),
        template: t.name.clone(),
        digest: t.digest.clone(),
        ordinal: 1,
        source,
        records: loaded.records.baseline(),
        recorded: time::stamp(now),
    });
    let path = format!("{dir}/{}", event::name(&splice));
    ops.push(Op::Create {
        path: path.clone(),
        bytes: event::bytes(&run_id, &splice),
    });
    drop(session);
    apply(ws, request, &ops)?;
    answer(
        ws,
        tool,
        request,
        place,
        vec![format!("recorded {path}")],
        now,
    )
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
    let done = done(step, &gates, &loaded, now);
    let path = format!(
        "{}/{}/{}",
        runs::dir(&loaded.rec),
        event::DIR,
        event::name(&done)
    );
    let ops = [Op::Create {
        path: path.clone(),
        bytes: event::bytes(&loaded.rec.id, &done),
    }];
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
    let Some(step) = loaded.graph.current().map(|i| &loaded.graph.steps[i]) else {
        return Err(KbError::Refused(format!(
            "run {} has no current step to skip",
            loaded.rec.id
        )));
    };
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
    let ops = [Op::Create {
        path: path.clone(),
        bytes: event::bytes(&loaded.rec.id, &skip),
    }];
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
