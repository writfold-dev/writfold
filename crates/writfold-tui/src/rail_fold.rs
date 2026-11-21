//! Earlier loop rounds fold into one rail line (#81): a run of steps the
//! verb marks `superseded` (history, not judged again) under the same loop
//! reads `rounds 1-2: done`, unless the selection is inside the run, which
//! then shows every step. Navigation still walks every step, so moving
//! onto a fold opens it. Rendering only, over the same rows.
use crate::cells::State;
use crate::graph::Node;
use crate::lines::Tone;
use crate::rail::Show;
use crate::rail_text::{clip, mark, width as cells_of};
use crate::style;
use ratatui::text::{Line, Span};
use serde_json::Value;
use std::ops::Range;

fn superseded(node: &Node<'_>) -> bool {
    node.row.get("superseded").and_then(Value::as_bool) == Some(true)
}

/// The folded runs: maximal runs of superseded steps under one loop, the
/// one holding the selection left open.
pub fn folds(nodes: &[Node<'_>], selected: Option<usize>) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if !superseded(node) {
            continue;
        }
        match out.last_mut() {
            Some(run) if run.end == index && nodes[run.start].parent == node.parent => {
                run.end = index + 1;
            }
            _ => out.push(index..index + 1),
        }
    }
    out.retain(|run| !selected.is_some_and(|s| run.contains(&s)));
    out
}

/// `rounds 1-2: done`, `round 1: done`, or `rounds 1-3: history` when the
/// steps did not all end the same way.
pub fn label(run: &[Node<'_>]) -> String {
    let rounds: Vec<u64> = (run.iter())
        .filter_map(|n| n.row.get("round").and_then(Value::as_u64))
        .collect();
    let (low, high) = (rounds.iter().min(), rounds.iter().max());
    let span = match (low, high) {
        (Some(l), Some(h)) if l == h => format!("round {l}"),
        (Some(l), Some(h)) => format!("rounds {l}-{h}"),
        _ => "earlier rounds".to_owned(),
    };
    let all = |state: State| run.iter().all(|n| n.state == state);
    let how = if all(State::Done) {
        "done"
    } else if all(State::Skipped) {
        "skipped"
    } else {
        "history"
    };
    format!("{span}: {how}")
}

/// The fold's rail line, at the depth of its steps.
pub fn line<'a>(run: &[Node<'a>], width: u16, show: Show) -> Line<'a> {
    let depth = run.first().map_or(0, |n| n.depth);
    let bars = format!(" {}", mark(show.glyphs, "│ ", "| ").repeat(depth));
    let glyph = mark(show.glyphs, "⋯", "..");
    let room = usize::from(width).saturating_sub(cells_of(&bars) + cells_of(glyph) + 1);
    let dim = style::style(show.look, Tone::Dim);
    Line::from(vec![
        Span::styled(bars, dim),
        Span::styled(glyph, dim),
        Span::raw(" "),
        Span::styled(clip(&label(run), room, show.glyphs), dim),
    ])
}
