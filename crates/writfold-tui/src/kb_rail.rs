//! Entries as rail rows: dim rail, state glyph, label, a two-cell mark
//! column and one dim word. Labels share one column (at most the pane less
//! 20 cells); ids are cut at the start, titles at the end.
use crate::cells::Glyphs;
use crate::kb_entry::{Entry, glyph, marks};
use crate::lines::Tone;
use crate::rail_text::{clip, left, mark, width as cells};
use crate::source::Reply;
use crate::style::{self, Look};
use ratatui::text::{Line, Span};

#[derive(Clone, Copy)]
pub struct Show<'a> {
    pub glyphs: Glyphs,
    pub look: Look,
    pub full: bool,
    pub width: u16,
    pub stale: Option<&'a Reply>,
}

/// Rows and, for each entry, the row it starts on.
pub struct Rail {
    pub lines: Vec<Line<'static>>,
    pub starts: Vec<usize>,
}

fn word(entry: &Entry, full: bool) -> String {
    if full && !entry.origin.is_empty() && !entry.status.is_empty() {
        format!("{}  {}", entry.word, entry.status)
    } else {
        entry.word.clone()
    }
}

/// The detailed second row: the entry's position, with the edge's origin
/// for a tree entry. The id is in the inspector.
fn second(entry: &Entry) -> String {
    let mut parts = Vec::new();
    if !entry.path.is_empty() {
        let line = entry.line.map(|n| format!(":{n}")).unwrap_or_default();
        parts.push(format!("@{}{line}", entry.path));
    }
    if !entry.origin.is_empty() {
        parts.push(format!("({})", entry.origin));
    }
    parts.join("  ")
}

pub fn build(entries: &[Entry], show: Show<'_>) -> Rail {
    let (glyphs, look) = (show.glyphs, show.look);
    let dim = style::style(look, Tone::Dim);
    let glyph_width = cells(glyph("", glyphs).glyph);
    let room = usize::from(show.width);
    let cap = room.saturating_sub(20).max(8);
    let column = entries
        .iter()
        .filter(|e| !e.header)
        .map(|e| cells(&e.label) + 2 * e.depth)
        .max()
        .unwrap_or(0)
        .min(cap);
    let mut rail = Rail {
        lines: Vec::new(),
        starts: Vec::new(),
    };
    let mut first = true;
    for entry in entries {
        rail.starts.push(rail.lines.len());
        if entry.header {
            let shape = match (first, glyphs) {
                (true, Glyphs::Unicode) => "┬",
                (false, Glyphs::Unicode) => "┼",
                _ => "+",
            };
            first = false;
            let head = format!(" {shape} ");
            let text = clip(&entry.label, room.saturating_sub(cells(&head)), glyphs);
            rail.lines.push(Line::from(vec![
                Span::styled(head, dim),
                Span::styled(text, style::style(look, Tone::Head)),
            ]));
            continue;
        }
        let bars = format!(" {}", mark(glyphs, "│ ", "| ").repeat(entry.depth));
        let label_room = column.saturating_sub(2 * entry.depth).max(1);
        let label = if entry.titled {
            clip(&entry.label, label_room, glyphs)
        } else {
            left(&entry.label, label_room, glyphs)
        };
        let pad = label_room.saturating_sub(cells(&label));
        let found = marks(entry, show.stale);
        let cell = glyph(&entry.status, glyphs);
        let used = cells(&bars) + glyph_width + 1 + label_room + 1 + 2 + 1;
        let word = clip(&word(entry, show.full), room.saturating_sub(used), glyphs);
        rail.lines.push(Line::from(vec![
            Span::styled(bars.clone(), dim),
            Span::styled(cell.glyph, style::style(look, cell.tone)),
            Span::raw(" "),
            Span::raw(label),
            Span::raw(" ".repeat(pad + 1)),
            Span::styled(
                if found.moved { "!" } else { " " },
                style::style(look, Tone::Bad),
            ),
            Span::styled(
                if found.unknown { "?" } else { " " },
                style::style(look, Tone::Warn),
            ),
            Span::raw(" "),
            Span::styled(word, dim),
        ]));
        if show.full {
            let indent = format!("{bars}{}", " ".repeat(glyph_width + 1));
            let text = left(&second(entry), room.saturating_sub(cells(&indent)), glyphs);
            rail.lines.push(Line::from(vec![
                Span::styled(indent, dim),
                Span::styled(text, dim),
            ]));
        }
    }
    if !entries.is_empty() {
        rail.lines
            .push(Line::styled(format!(" {}", mark(glyphs, "┴", "+")), dim));
    }
    rail
}

/// `text` broken at spaces into rows of at most `width` cells; a word
/// longer than a row is cut at its end with a marker.
pub fn wrap_words(text: &str, width: usize, glyphs: Glyphs) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        if !line.is_empty() && cells(&line) + 1 + cells(word) > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push(line);
    out.into_iter().map(|l| clip(&l, width, glyphs)).collect()
}

/// The first visible row: the selected entry with two rows of context
/// where the pane can hold them.
pub fn window(total: usize, selected: usize, height: usize, previous: usize) -> usize {
    let last = total.saturating_sub(height);
    let context = 2.min(height.saturating_sub(1) / 2);
    let mut start = previous.min(last);
    if selected < start + context {
        start = selected.saturating_sub(context);
    }
    if selected >= start + height.saturating_sub(context) {
        start = (selected + context + 1).saturating_sub(height);
    }
    start.min(last)
}
