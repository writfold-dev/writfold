//! Recorded step windows; independent of display metadata.
use super::{Baseline, Graph};

/// Each step's window begins at its splice when it is the first step of
/// that splice, else where the step before it ended (or began, when that
/// one is still open). A root splice after the first is the exception: it
/// was made with the plan (the required steps) or, for a required end
/// template added later, while the step before it was still open, so its
/// first step also begins where the step before it ended. (A splice under
/// a step is made while that step is current, after every step before it
/// closed, so its own baseline is the later one.)
pub(super) fn windows(graph: &mut Graph) {
    let mut previous: Option<Baseline> = None;
    let mut first_of: Vec<String> = Vec::new();
    let mut root_seen = false;
    for step in &mut graph.steps {
        let first = !first_of.contains(&step.splice);
        let later_root = first && step.root && root_seen;
        root_seen |= step.root;
        if first {
            first_of.push(step.splice.clone());
        }
        if (!first || later_root)
            && let Some(begin) = previous.take()
        {
            step.begin = begin;
        }
        previous = Some(step.end().cloned().unwrap_or_else(|| step.begin.clone()));
    }
}
