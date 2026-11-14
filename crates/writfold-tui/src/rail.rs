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
    let ids = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let mut rail = Rail {
        lines: Vec::new(),
        node_lines: Vec::with_capacity(nodes.len()),
    };
    let mut stack: Vec<usize> = Vec::new();
    let mut root_seen = false;
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
        rail.lines.push(node_line(node, width, glyphs, look));
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

fn node_line<'a>(node: &Node<'a>, width: u16, glyphs: Glyphs, look: Look) -> Line<'a> {
    let state = node.state.cell(glyphs);
    let prefix = format!(
        " {}{} ",
        mark(glyphs, "│ ", "| ").repeat(node.depth),
        state.glyph
    );
    let flags_width: usize = node.flags.iter().map(|f| f.len() + 2).sum();
    let fixed = Span::raw(&prefix).width() + node.chips.len() + 2 + flags_width;
    let available = usize::from(width).saturating_sub(fixed);
    let id_width = if available > 44 { 36 } else { available };
    let id = left(node.id, id_width, glyphs);
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
    let text_width = available.saturating_sub(id_width);
    if text_width > 0 {
        let end = node
            .rail_text
            .char_indices()
            .nth(text_width.saturating_sub(1))
            .map_or(node.rail_text.len(), |(at, _)| at);
        spans.push(Span::styled(
            format!(" {}", &node.rail_text[..end]),
            style::style(look, Tone::Dim),
        ));
    }
    for flag in &node.flags {
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
