//! A bracket rail with node-to-line positions and a bounded visible window.
use crate::cells::{self, Glyphs};
use crate::graph::Node;
use crate::lines::Tone;
use crate::style::{self, Look};
use ratatui::text::{Line, Span};
use std::collections::HashMap;

pub struct Rail<'a> {
    pub lines: Vec<Line<'a>>,
    pub node_lines: Vec<usize>,
}

fn mark(glyphs: Glyphs, unicode: &'static str, ascii: &'static str) -> &'static str {
    if glyphs == Glyphs::Unicode {
        unicode
    } else {
        ascii
    }
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

pub fn build<'a>(nodes: &[Node<'a>], width: u16, glyphs: Glyphs, look: Look) -> Rail<'a> {
    let ids: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let mut rail = Rail {
        lines: Vec::new(),
        node_lines: Vec::with_capacity(nodes.len()),
    };
    let mut stack: Vec<usize> = Vec::new();
    let mut root_seen = false;
    let chips = nodes.iter().map(|n| n.chips.len()).max().unwrap_or(0);
    let column = id_column(nodes);
    for (index, node) in nodes.iter().enumerate() {
        let path = group_path(index, nodes, &ids);
        let common = stack
            .iter()
            .zip(&path)
            .take_while(|(a, b)| same_group(&nodes[**a], &nodes[**b]))
            .count();
        close(&mut rail.lines, &stack, common, glyphs, look);
        stack.truncate(common);
        for ancestor in path.into_iter().skip(common) {
            let depth = stack.len();
            let shape = if depth > 0 {
                mark(glyphs, "├─╮", "+-\\")
            } else if root_seen {
                mark(glyphs, "┼", "+")
            } else {
                root_seen = true;
                mark(glyphs, "┬", "+")
            };
            rail.lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        " {}{shape} ",
                        mark(glyphs, "│ ", "| ").repeat(depth.saturating_sub(1))
                    ),
                    style::style(look, Tone::Dim),
                ),
                Span::styled(nodes[ancestor].group, style::style(look, Tone::Head)),
            ]));
            stack.push(ancestor);
        }
        rail.node_lines.push(rail.lines.len());
        let shape = Shape {
            chips,
            ids: column,
            glyphs,
        };
        rail.lines.push(node_line(node, width, shape, look));
    }
    close(&mut rail.lines, &stack, 0, glyphs, look);
    rail.lines.push(Line::styled(
        format!(" {}", mark(glyphs, "┴", "+")),
        style::style(look, Tone::Dim),
    ));
    rail
}

fn close(lines: &mut Vec<Line<'_>>, stack: &[usize], common: usize, glyphs: Glyphs, look: Look) {
    for depth in (common..stack.len()).rev().filter(|d| *d > 0) {
        lines.push(Line::styled(
            format!(
                " {}{}",
                mark(glyphs, "│ ", "| ").repeat(depth - 1),
                mark(glyphs, "├─╯", "+-/")
            ),
            style::style(look, Tone::Dim),
        ));
    }
}

fn left(text: &str, width: usize, glyphs: Glyphs) -> String {
    if Span::raw(text).width() <= width {
        return text.into();
    }
    let marker = mark(glyphs, "…", "...");
    let available = width.saturating_sub(Span::raw(marker).width());
    let mut taken = 0;
    let mut start = text.len();
    let mut bytes = [0; 4];
    for (at, ch) in text.char_indices().rev() {
        let size = Span::raw(&*ch.encode_utf8(&mut bytes)).width();
        if taken + size > available {
            break;
        }
        taken += size;
        start = at;
    }
    format!("{marker}{}", &text[start..])
}

/// Rail flags repeat only what the state glyph cannot say: disorder and invalid.
const ROW_FLAGS: [&str; 4] = ["out-of-order", "two-plans", "repeated", "invalid"];

#[derive(Clone, Copy)]
struct Shape {
    chips: usize,
    ids: usize,
    glyphs: Glyphs,
}

/// The widest id as shown (group-relative, plus its depth rail), capped so
/// rail text keeps room; ids are aligned to the deepest one.
fn id_column(nodes: &[Node<'_>]) -> usize {
    nodes
        .iter()
        .map(|n| Span::raw(shown_id(n.id, n.group)).width() + 2 * n.depth)
        .max()
        .unwrap_or(0)
        .min(36)
}

/// Inside a group, the id is shown from the group's own splice name; the
/// header above already names the ancestors (presentation only).
fn shown_id<'a>(id: &'a str, group: &str) -> &'a str {
    let name = group.split(' ').next().unwrap_or_default();
    if name.is_empty() {
        return id;
    }
    let prefix = format!("{name}/");
    if id.starts_with(&prefix) {
        return id;
    }
    id.rfind(&format!("/{prefix}"))
        .map_or(id, |at| &id[at + 1..])
}

fn clip(text: &str, width: usize, glyphs: Glyphs) -> String {
    if Span::raw(text).width() <= width {
        return text.into();
    }
    let marker = mark(glyphs, "…", "...");
    let keep = width.saturating_sub(Span::raw(marker).width());
    let end = text
        .char_indices()
        .nth(keep)
        .map_or(text.len(), |(at, _)| at);
    if keep == 0 {
        return String::new();
    }
    format!("{}{marker}", &text[..end])
}

fn node_line<'a>(node: &Node<'a>, width: u16, shape: Shape, look: Look) -> Line<'a> {
    let Shape { chips, ids, glyphs } = shape;
    let flags: Vec<_> = node
        .flags
        .iter()
        .filter(|f| ROW_FLAGS.contains(f))
        .collect();
    let state = node.state.cell(glyphs);
    let prefix = format!(
        " {}{} ",
        mark(glyphs, "│ ", "| ").repeat(node.depth),
        state.glyph
    );
    let flags_width: usize = flags.iter().map(|f| f.len() + 2).sum();
    let fixed = Span::raw(&prefix).width() + chips + 2 + flags_width;
    let available = usize::from(width).saturating_sub(fixed);
    let column = ids.saturating_sub(2 * node.depth).max(8);
    let id_width = if available > column + 8 {
        column
    } else {
        available
    };
    let id = left(shown_id(node.id, node.group), id_width, glyphs);
    let pad = id_width.saturating_sub(Span::raw(&id).width());
    let mut spans = vec![
        Span::styled(prefix, style::style(look, state.tone)),
        Span::raw(id),
        Span::raw(" ".repeat(pad + 1)),
    ];
    spans.extend(node.chips.iter().map(|status| {
        let cell = cells::chip(status, glyphs);
        Span::styled(cell.glyph, style::style(look, cell.tone))
    }));
    spans.push(Span::raw(
        " ".repeat(chips.saturating_sub(node.chips.len())),
    ));
    let text_width = available.saturating_sub(id_width);
    if text_width > 1 {
        spans.push(Span::styled(
            format!(" {}", clip(node.rail_text, text_width - 1, glyphs)),
            style::style(look, Tone::Dim),
        ));
    }
    for flag in flags {
        spans.push(Span::styled(
            format!(" !{flag}"),
            style::style(look, Tone::Bad),
        ));
    }
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
