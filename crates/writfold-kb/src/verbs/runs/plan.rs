//! A run's recorded plan position, without locating a subject or judging gates.
use crate::emit::More;
use crate::index::query::Rec;
use crate::plan::{disorder, fold};
use std::path::Path;

pub(super) fn position(root: &Path, rec: &Rec) -> Option<More> {
    let graph = fold::read(root, &crate::runs::dir(rec), &rec.id);
    if !graph.planned() {
        return None;
    }
    let total = graph.steps.len();
    let done = graph.steps.iter().filter(|s| s.done.is_some()).count();
    let skipped = graph
        .steps
        .iter()
        .filter(|s| s.done.is_none() && s.skipped.is_some())
        .count();
    let mut row = More {
        label: "plan",
        flags: disorder::find(&graph)
            .kinds()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        ..More::default()
    };
    match graph.current() {
        Some(i) => {
            let step = &graph.steps[i];
            row.text = format!(
                "step {}/{total} done={done} skipped={skipped} current={}",
                i + 1,
                step.path
            );
            if let Some((body, r)) = &step.repeat {
                // Folded only: rounds, not whether the loop is spent (#78).
                let rounds = graph.rounds(step);
                row.text
                    .push_str(&format!(" loop={body} round={rounds}/{}", r.max));
            }
            row.path = Some(step.splice.clone());
            row.line = Some(1);
        }
        None => {
            let drift = crate::plan::drift::recorded(root, &graph, rec);
            let present = vec![true];
            let ended = crate::plan::summary::ended(&graph, None, (&drift, false), &present);
            row.text = format!(
                "{} steps={total} passed={} attested={} waived={} skipped={}",
                ended.shown(),
                ended.passed,
                ended.attested,
                ended.waived,
                ended.skipped
            );
        }
    }
    if !graph.invalid.is_empty() {
        row.flags.push("invalid".into());
    }
    Some(row)
}

/// Whether the run's plan still has an open step.
pub(super) fn open(root: &Path, rec: &Rec) -> bool {
    let graph = fold::read(root, &crate::runs::dir(rec), &rec.id);
    graph.planned() && graph.current().is_some()
}
