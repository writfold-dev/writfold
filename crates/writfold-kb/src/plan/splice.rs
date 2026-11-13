//! The plan's splices: `plan set` (a template at the run's root) and
//! `plan use` (a template before the current step, as its children; at
//! the root when the run has no plan yet). Each writes the snapshots of
//! the template and of every template it includes (`use=`), then one
//! `splice-` event per template, in one write: a splice never exists
//! without its snapshots. A new root plan also splices the required start
//! and end templates ([`super::required`]).
//!
//! Idempotence: a template at the same digest already spliced at the
//! current step or any of its ancestors (the root included) is "already
//! spliced", exit 0, nothing written. So a skill that splices itself on
//! load, loaded again (after compaction, or by a retry), adds its steps
//! once. `--again` splices another copy, at the next ordinal.
use super::event::{self, Event, Include, Splice};
use super::fold;
use super::place::{decided, place};
use super::recheck::{Decided, guard};
use super::required;
use super::template::{self, Closure};
use super::write::{answer, apply, open_run};
use super::{Loaded, PlanVerb};
use crate::Request;
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::Op;
use crate::{runs, time};

/// One splice to write: a template's closure under `parent`; `again`:
/// an intended repeat (`plan use --again`), recorded in the event.
pub(super) struct Planned {
    pub parent: String,
    pub ordinal: u64,
    pub closure: Closure,
    pub again: bool,
}

type Repo<'a> = Option<(&'a std::path::Path, &'a str)>;

fn repo_of(loaded: &Loaded) -> Repo<'_> {
    loaded
        .subject
        .as_ref()
        .map(|s| (s.top.as_path(), s.name.as_str()))
}

/// The template a `plan set` names, and where it came from.
fn chosen(loaded: &Loaded, ws: &Workspace, request: &Request, verb: &PlanVerb) -> Result<Closure> {
    let PlanVerb::Set { template, from, .. } = verb else {
        return Err(KbError::Usage("plan set".into()));
    };
    let repo = repo_of(loaded);
    let (t, source) = match (template, from) {
        (Some(name), None) => template::find(&ws.root, repo, name)?,
        (None, Some(file)) => {
            let bytes = std::fs::read(request.cwd.join(file))?;
            let shown = file
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            (template::parse(&bytes, None)?, format!("--from {shown}"))
        }
        _ => {
            return Err(KbError::Usage(
                "plan set takes a template name or --from FILE".into(),
            ));
        }
    };
    template::closure(&ws.root, repo, t, source)
}

/// Refuse a template that names a gate no definition provides.
fn defined(ws: &Workspace, loaded: &Loaded, closure: &Closure) -> Result<()> {
    let top = loaded.subject.as_ref().map(|s| s.top.as_path());
    let (found, problems) = crate::gate::def::all(&ws.root, top);
    for t in closure.all() {
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
    }
    Ok(())
}

pub(super) fn too_deep(name: &str, parent: &str) -> KbError {
    KbError::Refused(format!(
        "plan {name} under {} would nest deeper than {} levels; nothing written",
        shown(parent),
        writfold_core::plan::MAX_DEPTH
    ))
}

pub(super) fn shown(parent: &str) -> &str {
    match parent {
        "" => "the root",
        _ => parent,
    }
}

/// A new root plan: the required start templates, `main`, the required
/// end templates, at root ordinals 1, 2, …
fn root(ws: &Workspace, loaded: &Loaded, main: Closure) -> Result<Vec<Planned>> {
    let repo = repo_of(loaded);
    let required = required::find(&ws.root, repo)?;
    required::check(&ws.root, repo, &required)?;
    let main_name = main.top.name.clone();
    let mut main = Some(main);
    let mut out = Vec::new();
    for (name, is_required) in required::order(&required, &main_name) {
        let closure = match (is_required, main.take()) {
            (false, Some(m)) => m,
            (false, None) => return Err(KbError::Refused(format!("plan {name} is listed twice"))),
            (true, kept) => {
                main = kept;
                let (t, source) = template::find(&ws.root, repo, &name)?;
                template::closure(&ws.root, repo, t, source)?
            }
        };
        out.push(Planned {
            parent: String::new(),
            ordinal: out.len() as u64 + 1,
            closure,
            again: false,
        });
    }
    Ok(out)
}

/// Write the planned splices: every snapshot not yet present, then each
/// splice event, guarded by what was decided (re-checked under the
/// lock). Returns the events' paths.
fn write(
    ws: &Workspace,
    request: &Request,
    loaded: &Loaded,
    (planned, decided): (&[Planned], Decided),
    now: i64,
) -> Result<Vec<String>> {
    let dir = format!("{}/{}", runs::dir(&loaded.rec), event::DIR);
    let mut ops = vec![guard(&runs::dir(&loaded.rec), &loaded.rec.id, decided)];
    let mut seen: Vec<String> = Vec::new();
    for t in planned.iter().flat_map(|p| p.closure.all()) {
        let hex = t.digest.trim_start_matches("sha256:");
        let snapshot = format!("{dir}/{}/{hex}.toml", event::TEMPLATES);
        if seen.contains(&snapshot) {
            continue;
        }
        seen.push(snapshot.clone());
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
    }
    let mut paths = Vec::new();
    for p in planned {
        let splice = Event::Splice(Splice {
            parent: p.parent.clone(),
            template: p.closure.top.name.clone(),
            digest: p.closure.top.digest.clone(),
            ordinal: p.ordinal,
            again: p.again,
            source: p.closure.source.clone(),
            includes: p
                .closure
                .includes
                .iter()
                .map(|(t, source)| Include {
                    name: t.name.clone(),
                    digest: t.digest.clone(),
                    source: source.clone(),
                })
                .collect(),
            records: loaded.records.baseline(),
            recorded: time::stamp(now),
        });
        let path = format!("{dir}/{}", event::name(&splice));
        ops.push(Op::Create {
            path: path.clone(),
            bytes: event::bytes(&loaded.rec.id, &splice),
        });
        paths.push(path);
    }
    apply(ws, request, &ops)?;
    Ok(paths)
}

/// Check every planned splice (gates defined, depth bounded) and write.
fn checked_write(
    ws: &Workspace,
    request: &Request,
    loaded: &Loaded,
    (planned, decided): (&[Planned], Decided),
    now: i64,
) -> Result<Vec<String>> {
    for p in planned {
        if fold::depth(&p.parent) + p.closure.height > writfold_core::plan::MAX_DEPTH {
            return Err(too_deep(&p.closure.top.name, &p.parent));
        }
        defined(ws, loaded, &p.closure)?;
    }
    write(ws, request, loaded, (planned, decided), now)
}

fn recorded(paths: Vec<String>) -> Vec<String> {
    paths.into_iter().map(|p| format!("recorded {p}")).collect()
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
    let closure = chosen(&loaded, ws, request, verb)?;
    let run_id = loaded.rec.id.clone();
    let place = ("plan-set", run_id.as_str(), repo.as_deref());
    let t = &closure.top;
    if loaded.graph.planned() {
        let roots = loaded
            .graph
            .splices
            .iter()
            .filter(|(_, s, _)| s.parent.is_empty());
        if roots
            .clone()
            .any(|(_, s, _)| s.template == t.name && s.digest == t.digest)
        {
            drop(session);
            let note = format!(
                "plan {} is already set on this run; nothing written",
                t.name
            );
            return answer(ws, tool, request, place, vec![note], now);
        }
        let names: Vec<&str> = roots.map(|(_, s, _)| s.template.as_str()).collect();
        return Err(KbError::Refused(format!(
            "run {run_id} already has a plan ({}); a second plan at the root is not built: splice under the current step with plan use, skip the remaining steps with plan skip, or start another run",
            names.join(", ")
        )));
    }
    let planned = root(ws, &loaded, closure)?;
    drop(session);
    let paths = checked_write(ws, request, &loaded, (&planned, Decided::Unplanned), now)?;
    answer(ws, tool, request, place, recorded(paths), now)
}

/// `plan use <template> [--again]`.
pub(super) fn use_template(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    verb: &PlanVerb,
    now: i64,
) -> Result<Output> {
    let PlanVerb::Use {
        template: name,
        again,
        run,
        repo,
    } = verb
    else {
        return Err(KbError::Usage("plan use".into()));
    };
    let session = Session::open(ws, Some(tool), false, now)?;
    let loaded = Loaded::open(&session, run.as_deref(), repo.as_deref(), &request.cwd)?;
    open_run(&loaded, &session)?;
    let run_id = loaded.rec.id.clone();
    let frame = ("plan-use", run_id.as_str(), repo.as_deref());
    let r = repo_of(&loaded);
    let (t, source) = template::find(&ws.root, r, name)?;
    let closure = template::closure(&ws.root, r, t, source)?;
    let (planned, mut notes, decision) = match loaded.graph.planned() {
        false => {
            let note =
                format!("run {run_id} had no plan: {name} is spliced at the root, as plan set");
            (root(ws, &loaded, closure)?, vec![note], Decided::Unplanned)
        }
        true => {
            let required = required::find(&ws.root, r)?;
            match place(&loaded.graph, &required, closure, *again)? {
                Ok(p) => {
                    let decision = decided(&loaded.graph, &p);
                    (vec![p], Vec::new(), decision)
                }
                Err(note) => {
                    drop(session);
                    return answer(ws, tool, request, frame, vec![note], now);
                }
            }
        }
    };
    if let [one] = planned.as_slice() {
        notes.push(format!(
            "spliced {name} under {} as {}",
            shown(&one.parent),
            event::prefix(&one.parent, name, one.ordinal)
        ));
    }
    drop(session);
    let paths = checked_write(ws, request, &loaded, (&planned, decision), now)?;
    notes.extend(recorded(paths));
    answer(ws, tool, request, frame, notes, now)
}
