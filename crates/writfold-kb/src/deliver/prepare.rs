//! `deliver prepare`: the candidate directory, written whole, and the note
//! when an older live, unbound candidate of the mode exists (this one now
//! decides).
use super::candidate::{self, Prepare};
use super::{open_run, step, withdraw};
use crate::Request;
use crate::error::Result;
use crate::index::query::Rec;
use crate::plan::recheck::{Decided, guard};
use crate::runs;
use crate::session::Session;
use crate::write::atomic;

/// `deliver prepare`: the candidate directory, written whole.
pub(super) fn prepare(
    session: Session<'_>,
    request: &Request,
    p: &Prepare,
    state: &std::path::Path,
) -> Result<(Rec, String, Vec<String>)> {
    let root = session.ws.root.clone();
    let rec = open_run(&session, p.run.as_deref(), true)?;
    let run_dir = runs::dir(&rec);
    let step = p
        .invocation
        .as_ref()
        .and_then(|_| current(&root, &run_dir, &rec.id));
    let at = ((rec.id.as_str(), run_dir.as_str()), step.as_deref());
    let (op, id) = candidate::plan(&root, &request.cwd, p, at, session.now)?;
    let older = older(&root, &rec, &p.mode);
    drop(session);
    // The recorded step must still be current when the candidate is
    // published, re-checked under the lock, as `agent prepare` does (#164).
    let ops: Vec<_> = (step.map(|s| guard(&run_dir, &rec.id, Decided::Current(s))))
        .into_iter()
        .chain([op])
        .collect();
    atomic::apply(&root, state, &ops, request.probe)?;
    let notes = vec![format!(
        "prepared {run_dir}/{}/{id}/{}: the exact payload; nothing is sent",
        candidate::DIR,
        candidate::PAYLOAD
    )];
    Ok((rec, id, notes.into_iter().chain(older).collect()))
}

/// The run's current step, when it has one.
fn current(root: &std::path::Path, run_dir: &str, run: &str) -> Option<String> {
    let graph = crate::plan::fold::read(root, run_dir, run);
    let i = graph.current()?;
    graph.steps.get(i).map(|s| s.path.clone())
}

/// The note `deliver prepare` adds when the run already has a live,
/// unbound candidate of the mode: the new one now decides.
fn older(root: &std::path::Path, rec: &Rec, mode: &str) -> Option<String> {
    let run_dir = runs::dir(rec);
    let cands = candidate::list(root, &run_dir, &rec.id);
    let withdrawn = withdraw::ids(root, &cands, &rec.id);
    let bound: Vec<String> = crate::plan::delivery::bound_steps(root, &run_dir, &rec.id)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let c = step::of_mode(&cands, mode)
        .into_iter()
        .rev()
        .find(|c| !withdrawn.contains(&c.id) && !bound.contains(&c.id))?;
    Some(format!(
        "{} is an older {mode} candidate; this one now decides. Withdraw the older one if it is not meant: writfold kb deliver withdraw {} --reason TEXT --by WHO",
        c.id, c.id
    ))
}
