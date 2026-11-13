//! The re-check under the KB write lock. Every plan write decides from a
//! fold read before the lock is taken; another session in the same clone
//! may write in between. So each plan write carries an [`Op::Plan`] guard
//! that the write protocol checks under the lock with every other guard
//! ([`crate::write::atomic`]), before any byte is published: the run's
//! events are folded again and the decision must still stand, else the
//! write is refused (exit 3) and nothing is written.
//!
//! - `plan set` (and `plan use` on a run without a plan): no splice exists;
//! - `plan advance` and `plan skip`: the step is still the current one;
//! - `plan use`: the step it splices under is still current and nothing
//!   was spliced under it since, so the ordinal decided is still the next
//!   one (re-checking only the current step lets two splices decided at
//!   one ordinal both land, out of order).
//!
//! Across clones no lock helps; a merge's disorder is flagged instead
//! ([`super::disorder`]).
use super::fold;
use crate::error::{KbError, Result};
use crate::write::Op;
use std::path::Path;

/// What a plan write decided from its fold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decided {
    /// The run had no plan.
    Unplanned,
    /// This step (its path) was the current one.
    Current(String),
    /// A splice under `parent` at `ordinal`; `current`: `parent` was the
    /// current step (false for a required end template at the root).
    Splice {
        parent: String,
        ordinal: u64,
        current: bool,
    },
}

/// The guard op for a decision about the run filed under `run_dir`.
pub fn guard(run_dir: &str, run: &str, decided: Decided) -> Op {
    Op::Plan {
        run_dir: run_dir.to_owned(),
        run: run.to_owned(),
        decided,
    }
}

/// Why `decided` no longer stands on `graph`, if it does not.
pub fn changed(graph: &fold::Graph, decided: &Decided) -> Option<String> {
    let current = graph
        .current()
        .and_then(|i| graph.steps.get(i))
        .map(|s| s.path.as_str());
    let is_current = |path: &str| current == Some(path);
    let now = || current.unwrap_or("no step");
    match decided {
        Decided::Unplanned if graph.planned() => Some("a plan was set meanwhile".into()),
        Decided::Unplanned => None,
        Decided::Current(path) if !is_current(path) => Some(format!(
            "{path} is no longer the current step (now {})",
            now()
        )),
        Decided::Current(_) => None,
        Decided::Splice {
            parent, current, ..
        } if *current && !is_current(parent) => Some(format!(
            "{parent} is no longer the current step (now {})",
            now()
        )),
        Decided::Splice {
            parent, ordinal, ..
        } => {
            let next = graph.next_ordinal(parent);
            (next != *ordinal).then(|| {
                let under = match parent.as_str() {
                    "" => "the root",
                    p => p,
                };
                format!("another splice under {under} took ordinal {ordinal}")
            })
        }
    }
}

/// Check the guard under the lock: fold again and refuse when the
/// decision no longer stands.
pub fn check(root: &Path, run_dir: &str, run: &str, decided: &Decided) -> Result<()> {
    let graph = fold::read(root, run_dir, run);
    match changed(&graph, decided) {
        None => Ok(()),
        Some(why) => Err(KbError::Refused(format!(
            "the plan changed since this command read it ({why}); nothing written; rerun"
        ))),
    }
}
