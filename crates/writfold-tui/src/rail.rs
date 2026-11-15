//! A bracket rail with node-to-line positions and a bounded visible window.
use crate::cells::{self, Glyphs};
use crate::graph::Node;
use crate::lines::Tone;
use crate::rail_text::{clip, id_column, left, mark, shown_id, splice_name, width as cells_of};
use crate::source::field;
use crate::style::{self, Look};
use ratatui::text::{Line, Span};
use std::collections::HashMap;

pub struct Rail<'a> {
    pub lines: Vec<Line<'a>>,
    pub node_lines: Vec<usize>,
}

fn group_path<'a>(index: usize, nodes: &[Node<'a>], ids: &HashMap<&str, usize>) -> Vec<usize> {
    let mut path = vec![index];
    let mut parent = nodes[index].parent;
    while path.len() <= nodes.len() {
        let Some(next) = parent.and_then(|id| ids.get(id)).copied() else {
            break;
        };
        if path.contains(&next) {
            break;
        }
        path.push(next);
        parent = nodes[next].parent;
    }
    path.reverse();
    path
}

fn same_group(a: &Node<'_>, b: &Node<'_>) -> bool {
    a.group == b.group && a.parent == b.parent
}

pub fn build<'a>(nodes: &[Node<'a>], width: u16, show: Show) -> Rail<'a> {
    let ids: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let mut rail = Rail {
        lines: Vec::new(),
        node_lines: Vec::with_capacity(nodes.len()),
    };
    let mut stack: Vec<usize> = Vec::new();
    let mut root_seen = false;
    let shape = Shape {
        chips: nodes.iter().map(|n| n.chips.len()).max().unwrap_or(0),
        ids: id_column(nodes, show.full),
        show,
    };
    for (index, node) in nodes.iter().enumerate() {
        let path = group_path(index, nodes, &ids);
        let common = stack
            .iter()
            .zip(&path)
            .take_while(|(a, b)| same_group(&nodes[**a], &nodes[**b]))
            .count();
        close(&mut rail.lines, &stack, common, show);
        stack.truncate(common);
        for ancestor in path.into_iter().skip(common) {
            let depth = stack.len();
            let first = !root_seen;
            root_seen = true;
            let group = nodes[ancestor].group;
            rail.lines.push(header(group, depth, first, width, show));
            stack.push(ancestor);
        }
        rail.node_lines.push(rail.lines.len());
        let current = field(node.row, "status") == "current";
        let text = show.full || current || show.selected == Some(index);
        rail.lines.push(node_line(node, width, shape, text));
    }
    close(&mut rail.lines, &stack, 0, show);
    rail.lines.push(Line::styled(
        format!(" {}", mark(show.glyphs, "┴", "+")),
        style::style(show.look, Tone::Dim),
    ));
    rail
}

/// A group header: dim rail, then the splice name (calm) or the whole
/// splice text (detailed), clipped with a marker at the pane edge.
fn header<'a>(group: &'a str, depth: usize, first: bool, width: u16, show: Show) -> Line<'a> {
    let glyphs = show.glyphs;
    let shape = if depth > 0 {
        mark(glyphs, "├─╮", "+-\\")
    } else if first {
        mark(glyphs, "┬", "+")
    } else {
        mark(glyphs, "┼", "+")
    };
    let rail = format!(
        " {}{shape} ",
        mark(glyphs, "│ ", "| ").repeat(depth.saturating_sub(1))
    );
    let text = if show.full { group } else { splice_name(group) };
    let room = usize::from(width).saturating_sub(cells_of(&rail));
    Line::from(vec![
        Span::styled(rail, style::style(show.look, Tone::Dim)),
        Span::styled(
            clip(text, room, glyphs),
            style::style(show.look, Tone::Head),
        ),
    ])
}

fn close(lines: &mut Vec<Line<'_>>, stack: &[usize], common: usize, show: Show) {
    for depth in (common..stack.len()).rev().filter(|d| *d > 0) {
        lines.push(Line::styled(
            format!(
                " {}{}",
                mark(show.glyphs, "│ ", "| ").repeat(depth - 1),
                mark(show.glyphs, "├─╯", "+-/")
            ),
            style::style(show.look, Tone::Dim),
        ));
    }
}

/// Rail flags repeat only what the state glyph cannot say: disorder and invalid.
const ROW_FLAGS: [&str; 4] = ["out-of-order", "two-plans", "repeated", "invalid"];

/// What the rail shows: glyph set, colours, the detail level and the
/// selected node (whose text the calm level shows beside the current one).
#[derive(Clone, Copy)]
pub struct Show {
    pub glyphs: Glyphs,
    pub look: Look,
    pub full: bool,
    pub selected: Option<usize>,
}

#[derive(Clone, Copy)]
struct Shape {
    chips: usize,
    ids: usize,
    show: Show,
}

/// One step row: dim rail bars, the state glyph in its tone, the shown id,
/// aligned chips, then (when `text`) the verb's rail text and row flags.
fn node_line<'a>(node: &Node<'a>, width: u16, shape: Shape, text: bool) -> Line<'a> {
    let Shape { chips, ids, show } = shape;
    let (glyphs, look) = (show.glyphs, show.look);
    let flags: Vec<_> = node
        .flags
        .iter()
        .filter(|f| ROW_FLAGS.contains(f))
        .collect();
    let state = node.state.cell(glyphs);
    let bars = format!(" {}", mark(glyphs, "│ ", "| ").repeat(node.depth));
    let flags_width: usize = flags.iter().map(|f| f.len() + 2).sum();
    let fixed = cells_of(&bars) + cells_of(state.glyph) + 1 + chips + 2 + flags_width;
    let available = usize::from(width).saturating_sub(fixed);
    // Every row's id column ends at the same cell; only a capped column
    // (very deep nesting) can leave a row less than one cell.
    let column = ids.saturating_sub(2 * node.depth).max(1);
    let id_width = if available > column + 8 {
        column
    } else {
        available
    };
    let id = left(shown_id(node.id, node.group, show.full), id_width, glyphs);
    let pad = id_width.saturating_sub(cells_of(&id));
    let mut spans = vec![
        Span::styled(bars, style::style(look, Tone::Dim)),
        Span::styled(state.glyph, style::style(look, state.tone)),
        Span::raw(" "),
        Span::raw(id),
        Span::raw(" ".repeat(pad + 1)),
    ];
    spans.extend(node.chips.iter().map(|status| {
        let cell = cells::chip(status, glyphs);
        Span::styled(cell.glyph, style::style(look, cell.tone))
    }));
    let text_width = available.saturating_sub(id_width);
    let text = text && text_width > 1;
    if text || !flags.is_empty() {
        let pad = chips.saturating_sub(node.chips.len());
        spans.push(Span::raw(" ".repeat(pad)));
    }
    if text {
        let text = format!(" {}", clip(node.rail_text, text_width - 1, glyphs));
        spans.push(Span::styled(text, style::style(look, Tone::Dim)));
    }
    let bad = style::style(look, Tone::Bad);
    spans.extend(flags.iter().map(|f| Span::styled(format!(" !{f}"), bad)));
    Line::from(spans)
}

/// Keep two rows of context where the viewport can hold them. No u16 offset.
pub fn window(
    rail: &Rail<'_>,
    selected: usize,
    height: usize,
    previous: usize,
) -> std::ops::Range<usize> {
    let last = rail.lines.len().saturating_sub(height);
    let selected = rail.node_lines.get(selected).copied().unwrap_or(0);
    let context = 2.min(height.saturating_sub(1) / 2);
    let mut start = previous.min(last);
    if selected < start + context {
        start = selected.saturating_sub(context);
    }
    if selected >= start + height.saturating_sub(context) {
        start = selected.saturating_add(context + 1).saturating_sub(height);
    }
    start = start.min(last);
    start..start.saturating_add(height).min(rail.lines.len())
}

#[cfg(test)]
#[path = "rail_tests.rs"]
mod tests;
