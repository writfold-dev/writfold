//! `writfold kb agent prepare <id> --input FILE --out DIR [--retry-of ID]
//! [--by WHO] [--run ID] [--repo DIR]` (#147, #148): record one invocation
//! of agent profile `<id>` in the run at its current step, then write a
//! package an external harness can run. Writfold starts no model.
//!
//! The decision is [`writfold_core::invoke::prepare_check`] over six
//! facts gathered here (tested IO): the run is open; it has a current step;
//! the profile's closure is admitted; every plan template a listed skill
//! names is in the run at the current step or an ancestor; the input was
//! read; a required context packet is complete. A refusal writes nothing
//! (exit 3) and names its `next:` step.
//!
//! The record is one create-new file, published with a guard that the
//! step is still current under the KB write lock; the package is written
//! after it, outside the KB. A crash between the two leaves a readable
//! `prepared` invocation, and preparing again records another.
use super::invocation;
use super::refs::Table;
use super::{Def, DefKind, inspect};
use crate::error::{KbError, Result};
use crate::plan::Loaded;
use crate::plan::recheck::{Decided, guard};
use crate::session::Session;
use crate::write::{Op, atomic};
use crate::{Request, runs};
use std::path::{Path, PathBuf};
use writfold_core::invoke::{self as kernel, PrepareRefusal};

/// The largest input accepted.
const MAX_INPUT: usize = 1 << 20;

/// What `agent prepare` was asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prepare {
    pub id: String,
    pub input: PathBuf,
    pub out: PathBuf,
    pub retry_of: Option<String>,
    pub by: Option<String>,
    pub run: Option<String>,
    pub repo: Option<PathBuf>,
    /// `--for GATE.ITEM`: the checklist item the report will answer (#156).
    pub for_item: Option<String>,
}

/// Everything the facts were gathered from.
pub(super) struct Gathered<'a> {
    pub profile: Option<&'a Def>,
    pub closure: std::result::Result<String, String>,
    pub step: Option<String>,
    /// The first skill (and its plan) whose template is not in the run.
    pub missing_plan: Option<(String, String)>,
    pub input: std::result::Result<Vec<u8>, String>,
    /// The packet's text and whether it is complete, when required.
    pub packet: Option<(Vec<u8>, bool)>,
    /// Each needed gate (#154): who named it, its id, its fact and what is
    /// owed, in order (the profile's, then each skill's).
    pub needs: Vec<(String, String, writfold_core::guard::Fact, String)>,
}

/// The template names spliced or included at `step` or an ancestor.
fn in_run(loaded: &Loaded, step: &str) -> Vec<Vec<u8>> {
    let under = crate::plan::ancestors(step);
    (loaded.graph.uses.iter())
        .filter(|u| under.contains(&u.parent.as_str()))
        .map(|u| u.template.as_bytes().to_vec())
        .collect()
}

/// The first listed skill whose plan template is not in the run
/// ([`writfold_core::compose::covers`] over the template names).
fn missing_plan(t: &Table, p: &Def, spliced: &[Vec<u8>]) -> Option<(String, String)> {
    let named: Vec<(String, String)> = (p.skills.iter())
        .filter_map(|s| t.find(DefKind::Skill, s))
        .filter_map(|s| Some((s.id.clone(), s.plan.clone()?)))
        .collect();
    let wanted: Vec<Vec<u8>> = named
        .iter()
        .map(|(_, plan)| plan.as_bytes().to_vec())
        .collect();
    let at = writfold_core::compose::covers(&wanted, spliced)?;
    named.get(at).cloned()
}

fn read_input(cwd: &Path, input: &Path) -> std::result::Result<Vec<u8>, String> {
    let bytes = std::fs::read(cwd.join(input)).map_err(|e| format!("{}: {e}", input.display()))?;
    match bytes.len() {
        0 => Err(format!("{} is empty", input.display())),
        n if n > MAX_INPUT => Err(format!(
            "{} is larger than {MAX_INPUT} bytes",
            input.display()
        )),
        _ => Ok(bytes),
    }
}

/// The run's context packet as text, and whether it is complete.
fn packet(session: &Session<'_>, request: &Request, run: &str) -> Result<(Vec<u8>, bool)> {
    let (answer, id) =
        crate::verbs::runs::packet(session, Some(run), &request.cwd, request.budget)?;
    let whole = (answer.words.iter()).any(|(k, v)| *k == "packet" && v == "complete");
    let frame = crate::emit::Frame {
        verb: "resume".into(),
        args: vec![id, "--packet".into()],
        budget: request.budget,
        cursor: None,
        json: false,
    };
    Ok((session.emit(&answer, &frame)?.text.into_bytes(), whole))
}

/// The gates the profile and its skills need, each once, with who named it.
fn wanted(t: &Table, p: &Def) -> Vec<(String, String)> {
    let skills = p.skills.iter().filter_map(|s| t.find(DefKind::Skill, s));
    let mut out: Vec<(String, String)> = Vec::new();
    for d in std::iter::once(p).chain(skills) {
        for need in &d.needs {
            if !out.iter().any(|(_, g)| g == need) {
                out.push((format!("{} {}", d.kind.word(), d.id), need.clone()));
            }
        }
    }
    out
}

/// Each needed gate judged for the run and its subject (#154).
fn needs(
    session: &Session<'_>,
    loaded: &Loaded,
    wanted: Vec<(String, String)>,
) -> Vec<(String, String, writfold_core::guard::Fact, String)> {
    let ids: Vec<String> = wanted.iter().map(|(_, g)| g.clone()).collect();
    let judged = crate::guard::require::needs::judge_needs(
        session,
        &loaded.rec,
        loaded.subject.clone(),
        &ids,
    );
    (wanted.into_iter().zip(judged))
        .map(|((who, gate), (fact, owed))| (who, gate, fact, owed))
        .collect()
}

/// Gather the facts for profile `a.id` on the loaded run.
pub(super) fn gather<'a>(
    session: &Session<'_>,
    request: &Request,
    a: &Prepare,
    (loaded, t): (&Loaded, &'a Table),
) -> Result<Gathered<'a>> {
    let step = (loaded.graph.current())
        .and_then(|i| loaded.graph.steps.get(i))
        .map(|s| s.path.clone());
    let (found, closure) = inspect::decide(t, &a.id);
    let profile = found.map(|(p, _)| p);
    let spliced = step.as_deref().map_or_else(Vec::new, |s| in_run(loaded, s));
    let packet = match profile.is_some_and(|p| p.packet) {
        true => Some(packet(session, request, &loaded.rec.id)?),
        false => None,
    };
    let wanted = profile.map_or_else(Vec::new, |p| wanted(t, p));
    Ok(Gathered {
        needs: needs(session, loaded, wanted),
        profile,
        closure,
        missing_plan: profile.and_then(|p| missing_plan(t, p, &spliced)),
        step,
        input: read_input(&request.cwd, &a.input),
        packet,
    })
}

/// Why prepare was refused, with its `next:` step.
fn refusal(r: PrepareRefusal, g: &Gathered<'_>, a: &Prepare, run: &str) -> String {
    let id = &a.id;
    match r {
        PrepareRefusal::RunClosed => {
            format!("run {run} is closed; next: writfold kb status {run} --set open")
        }
        PrepareRefusal::NotCurrentStep => format!(
            "run {run} has no current step to prepare {id} at; next: writfold kb plan use <template>"
        ),
        PrepareRefusal::Closure => format!(
            "{}; next: writfold kb agent inspect {id}",
            g.closure.as_ref().err().map_or("", String::as_str)
        ),
        PrepareRefusal::PlanNotInRun => {
            let (skill, plan) = g.missing_plan.clone().unwrap_or_default();
            format!(
                "skill {skill} names plan {plan}, which is not in this run; next: writfold kb plan use {plan}"
            )
        }
        PrepareRefusal::InputUnread => format!(
            "--input {}; next: pass a readable, non-empty file of at most {MAX_INPUT} bytes",
            g.input.as_ref().err().map_or("", String::as_str)
        ),
        PrepareRefusal::PacketIncomplete => format!(
            "agent {id} requires a context packet and run {run}'s is partial; next: writfold kb resume {run} --packet, with a larger --budget-tokens"
        ),
        PrepareRefusal::Needs(i) => {
            let (who, gate, fact, owed) = g.needs.get(i).cloned().unwrap_or_else(|| {
                (
                    String::new(),
                    String::new(),
                    writfold_core::guard::Fact::Unknown,
                    String::new(),
                )
            });
            let word = match fact {
                writfold_core::guard::Fact::Unknown => "cannot be judged",
                _ => "is not met",
            };
            let next = match owed.contains("next:") {
                true => String::new(),
                false => format!("; next: writfold kb gate status {gate}"),
            };
            format!("{who} needs gate {gate}, which {word}: {owed}{next}")
        }
    }
}

/// The six facts, judged.
pub(super) fn judge(g: &Gathered<'_>, a: &Prepare, (run, open): (&str, bool)) -> Result<()> {
    let facts = kernel::Prepare {
        run_open: open,
        current_step: g.step.is_some(),
        closure_admitted: g.closure.is_ok(),
        plans_in_run: g.missing_plan.is_none(),
        input_read: g.input.is_ok(),
        packet_required: g.packet.is_some(),
        packet_complete: g.packet.as_ref().is_some_and(|(_, whole)| *whole),
    };
    let needs: Vec<writfold_core::guard::Fact> = g.needs.iter().map(|(_, _, f, _)| *f).collect();
    kernel::prepare(&facts, &needs).map_err(|r| KbError::Refused(refusal(r, g, a, run)))
}

/// `agent prepare`: judge, record, then write the package.
pub fn run(
    ws: &crate::workspace::Workspace,
    request: &Request,
    a: &Prepare,
    now: i64,
) -> Result<crate::emit::Output> {
    let state = (ws.repo.as_ref().map(|r| r.git_dir.join("writfold")))
        .ok_or_else(|| KbError::Refused("agent prepare needs a KB under Git".into()))?;
    let out = super::package::fresh(&request.cwd, &a.out)?;
    let tool = crate::workspace::GitTool::locate()?;
    let session = Session::open(ws, Some(&tool), false, now)?;
    let loaded = Loaded::open(&session, a.run.as_deref(), a.repo.as_deref(), &request.cwd)?;
    let top = loaded.subject.as_ref().map(|s| s.top.clone());
    let t = inspect::table_at(&ws.root, top.as_deref());
    let g = gather(&session, request, a, (&loaded, &t))?;
    let run_id = loaded.rec.id.clone();
    judge(
        &g,
        a,
        (&run_id, runs::state(&session, &loaded.rec) != "closed"),
    )?;
    let run_dir = runs::dir(&loaded.rec);
    let found = invocation::list(&ws.root, &run_dir, &run_id);
    let (record, files) = super::package::build(&t, &g, a, (&run_id, &found), now)?;
    let path = invocation::path(&run_dir, &record.id);
    let step = record.step.clone();
    drop(session);
    let ops = [
        guard(&run_dir, &run_id, Decided::Current(step)),
        Op::Create {
            path: path.clone(),
            bytes: invocation::bytes(&record),
        },
    ];
    atomic::apply(&ws.root, &state, &ops, request.probe)?;
    super::package::write(&out, &record, &files)?;
    Ok(super::package::shown(&record, &path, &a.out, request.json))
}
