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
            row.path = Some(step.splice.clone());
            row.line = Some(1);
        }
        None => row.text = format!("complete steps={total} done={done} skipped={skipped}"),
    }
    if !graph.invalid.is_empty() {
        row.flags.push("invalid".into());
    }
    Some(row)
}
