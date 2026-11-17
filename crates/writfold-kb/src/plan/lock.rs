//! The refusal of `plan skip` on a step the run's pin locks
//! (`unskippable`, policy spec 2.4), decided by
//! [`writfold_core::plan::skip_check`]: exit 3, nothing written, the step
//! and what it owes, and where the lock comes from.
use super::{Loaded, show};
use crate::emit::{Answer, More};
use crate::error::Result;
use crate::runs;
use crate::session::Session;

/// The answer of a refused skip of step `at`.
pub(super) fn refused(session: &Session<'_>, loaded: &Loaded, at: usize) -> Result<Answer> {
    let step = &loaded.graph.steps[at];
    let gates = (loaded.judged(session)?)
        .and_then(|mut all| (at < all.len()).then(|| all.swap_remove(at)))
        .unwrap_or_default();
    let mut row = show::step_row(step, true, &gates, !gates.is_empty());
    row.flags.push("locked".into());
    if !row.more.iter().any(|m| m.label == "next") {
        row.more.push(More {
            label: "next",
            text: "writfold kb plan advance, once the step's gates are met".into(),
            ..More::default()
        });
    }
    let (pins, _) = super::pin::read(&session.ws.root, &runs::dir(&loaded.rec), &loaded.rec.id);
    let first = pins.first();
    let source = (first.and_then(|p| p.required.as_ref()))
        .map_or_else(|| "required.toml".to_owned(), |r| r.source.clone());
    let words =
        (first.and_then(|p| p.required.as_ref())).map_or_else(Vec::new, |r| r.unskippable.clone());
    let list = serde_json::to_string(&words)
        .unwrap_or_default()
        .replace(',', ", ");
    let line = std::fs::read_to_string(session.ws.root.join(&source))
        .ok()
        .and_then(|t| {
            t.lines()
                .position(|l| l.trim_start().starts_with("unskippable"))
        })
        .map_or(1, |n| n + 1);
    let pinned = first.map_or_else(String::new, |p| format!(", pinned @{}:1", p.path));
    let mut answer = Answer {
        refused: true,
        rows: vec![row],
        ..Answer::default()
    };
    answer.notes.push(format!(
        "not skipped: {} is a required step this run cannot skip (unskippable = {list} @{source}:{line}{pinned}); nothing written",
        step.path
    ));
    Ok(answer)
}
