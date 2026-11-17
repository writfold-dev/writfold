//! `deliver withdraw`: a statement that a candidate is not meant, with who
//! and why. It is bound to the candidate's ID, not its bytes, so an edit
//! does not undo it; prepare again to go on. A withdrawn candidate decides
//! no step, the guard never passes it, it is left out of the `AllReady`
//! rule, the trailer's standing counts and the exit code, and `deliver
//! check`, `attest`, `waive` and `posted` refuse it.
//!
//! The record is `writfold-delivery-withdrawn/1`, one create-new file
//! `statements/withdrawn-<time>-<hex>.json` in the candidate's directory.
use super::candidate::Candidate;
use super::record;
use crate::error::{KbError, Result};
use crate::gate::statement;
use crate::write::{Op, unique};
use serde_json::json;
use std::path::Path;

pub const SCHEMA: &str = "writfold-delivery-withdrawn/1";
/// Withdrawal files start with this, beside the candidate's statements.
pub const PREFIX: &str = "withdrawn-";

/// A candidate's withdrawal as read back.
#[derive(Clone, Debug)]
pub struct Withdrawn {
    pub path: String,
    pub by: String,
    pub reason: String,
}

fn dir(c: &Candidate) -> String {
    format!("{}/{}", c.dir, statement::DIR)
}

/// Whether a statement file's text is a withdrawal (kept apart from the
/// candidate's attestations and waivers).
pub fn is_withdrawal(text: &str) -> bool {
    text.contains(SCHEMA)
}

/// The candidate's first valid withdrawal of `run`, by file name. A
/// withdrawal naming another candidate or run counts for nothing.
pub fn of(root: &Path, c: &Candidate, run: &str) -> Option<Withdrawn> {
    record::files(root, &dir(c), PREFIX)
        .into_iter()
        .find_map(|(_, path, text)| {
            let value = record::open(&text?, SCHEMA).ok()?;
            let ours = record::text(&value, "candidate").ok()? == c.id
                && record::text(&value, "run").ok()? == run;
            ours.then_some(Withdrawn {
                path,
                by: record::text(&value, "by").ok()?,
                reason: record::text(&value, "reason").ok()?,
            })
        })
}

/// The IDs of the candidates of `run` that are withdrawn.
pub fn ids(root: &Path, cands: &[Candidate], run: &str) -> Vec<String> {
    cands
        .iter()
        .filter(|c| of(root, c, run).is_some())
        .map(|c| c.id.clone())
        .collect()
}

/// The refusal of a verb on a withdrawn candidate.
pub fn refuse(w: &Withdrawn, id: &str, verb: &str) -> KbError {
    KbError::Refused(format!(
        "candidate {id} was withdrawn (by {}: {}); {verb} does not apply: prepare a new candidate",
        w.by, w.reason
    ))
}

/// Plan the withdrawal of `c`: refused when it is already withdrawn or a
/// done step is bound to it.
pub fn plan(
    root: &Path,
    c: &Candidate,
    run: (&str, &str),
    said: (&str, &str),
    now: i64,
) -> Result<(Op, String)> {
    let (run_id, run_dir) = run;
    if let Some(w) = of(root, c, run_id) {
        return Err(refuse(&w, &c.id, "deliver withdraw"));
    }
    let bound = crate::plan::delivery::bound_steps(root, run_dir, run_id);
    if let Some((_, step)) = bound.iter().find(|(id, _)| *id == c.id) {
        return Err(KbError::Refused(format!(
            "candidate {} is bound to {step} [done]; done is history",
            c.id
        )));
    }
    let reason = statement::check_text("--reason", said.0)?;
    let by = statement::check_text("--by", said.1)?;
    let path = format!("{}/{PREFIX}{}.json", dir(c), unique(now, &c.id));
    let bytes = record::seal(json!({
        "schema": SCHEMA,
        "run": run_id,
        "candidate": c.id,
        "reason": reason,
        "by": by,
        "recorded": crate::time::stamp(now),
    }));
    Ok((
        Op::Create {
            path: path.clone(),
            bytes,
        },
        path,
    ))
}
