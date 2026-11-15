//! The inspector's ink: width, glyphs and level, and the line builders
//! every section uses (wrapped text with a hanging indent, glyph rows,
//! positions), plus the record's header.
use crate::cells::Glyphs;
use crate::kb_entry::{Entry, glyph};
use crate::lines::{self, Tone};
use crate::rail_text::{clip, left, width as cells};
use crate::source::{Reply, field};
use crate::style::{self, Look};
use ratatui::text::{Line, Span};
use serde_json::Value;

pub(crate) struct Ink {
    pub look: Look,
    pub glyphs: Glyphs,
    pub width: usize,
    pub full: bool,
}

impl Ink {
    pub(crate) fn style(&self, tone: Tone) -> ratatui::style::Style {
        style::style(self.look, tone)
    }

    /// `text` after `first`, wrapped at spaces, continued at `indent`.
    pub(crate) fn wrap(
        &self,
        out: &mut Vec<Line<'static>>,
        first: &str,
        text: &str,
        tone: Tone,
        indent: usize,
    ) {
        let mut line = first.to_owned();
        let mut fresh = true;
        for word in text.split(' ') {
            let gap = usize::from(!fresh);
            if !fresh && cells(&line) + gap + cells(word) > self.width {
                out.push(Line::styled(std::mem::take(&mut line), self.style(tone)));
                line = " ".repeat(indent);
                fresh = true;
            }
            if !fresh {
                line.push(' ');
            }
            // A word longer than the row continues whole on the next ones.
            let mut rest = word;
            while cells(&line) + cells(rest) > self.width && self.width > indent + 1 {
                let (head, tail) = split_at(rest, self.width.saturating_sub(cells(&line)));
                if head.is_empty() {
                    break;
                }
                line.push_str(head);
                out.push(Line::styled(std::mem::take(&mut line), self.style(tone)));
                line = " ".repeat(indent);
                rest = tail;
            }
            line.push_str(rest);
            fresh = false;
        }
        let shown = clip(&line, self.width, self.glyphs);
        out.push(Line::styled(shown, self.style(tone)));
    }

    /// One clipped row: indent, state glyph, label, then a dim word.
    pub(crate) fn glyph_row(
        &self,
        out: &mut Vec<Line<'static>>,
        status: &str,
        label: &str,
        word: &str,
    ) {
        let cell = glyph(status, self.glyphs);
        let room = self.width.saturating_sub(3 + cells(cell.glyph));
        let label = clip(label, room, self.glyphs);
        let word = clip(word, room.saturating_sub(cells(&label) + 2), self.glyphs);
        out.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(cell.glyph, self.style(cell.tone)),
            Span::raw(" "),
            Span::raw(label),
            Span::styled(
                if word.is_empty() {
                    String::new()
                } else {
                    format!("  {word}")
                },
                self.style(Tone::Dim),
            ),
        ]));
    }

    pub(crate) fn position(&self, out: &mut Vec<Line<'static>>, value: &Value, indent: usize) {
        let path = field(value, "path");
        if self.full && !path.is_empty() {
            let at = value
                .get("line")
                .and_then(Value::as_u64)
                .map(|n| format!(":{n}"));
            let text = format!("@{path}{}", at.unwrap_or_default());
            let room = self.width.saturating_sub(indent);
            let shown = format!("{}{}", " ".repeat(indent), left(&text, room, self.glyphs));
            out.push(Line::styled(shown, self.style(Tone::Dim)));
        }
    }

    pub(crate) fn heading(&self, out: &mut Vec<Line<'static>>, text: &str) {
        if out.last().is_some_and(|l| l.width() > 0) {
            out.push(Line::default());
        }
        out.push(Line::styled(text.to_owned(), self.style(Tone::Head)));
    }
}

/// `text` cut after at most `room` cells.
fn split_at(text: &str, room: usize) -> (&str, &str) {
    let mut taken = 0;
    for (at, ch) in text.char_indices() {
        let size = cells(ch.encode_utf8(&mut [0; 4]));
        if taken + size > room {
            return text.split_at(at);
        }
        taken += size;
    }
    (text, "")
}

pub(crate) fn more<'a>(row: &'a Value, label: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
    lines::more(row).filter(move |m| field(m, "label") == label)
}

pub(crate) fn refused(out: &mut Vec<Line<'static>>, reply: &Reply, ink: &Ink) {
    if let Some(error) = &reply.error {
        let text = format!("refused (exit {}): {error}", reply.exit);
        ink.wrap(out, "", &text, Tone::Bad, 0);
    }
}

/// Label and status chip, then the full id; `d` adds every flag as a word.
pub(crate) fn header(out: &mut Vec<Line<'static>>, entry: &Entry, ink: &Ink) {
    let chip = if entry.status.is_empty() {
        0
    } else {
        cells(&entry.status) + 4
    };
    let mut rows = crate::kb_rail::wrap_words(&entry.label, ink.width, ink.glyphs);
    let last = rows.pop().unwrap_or_default();
    for row in rows {
        out.push(Line::styled(row, ink.style(Tone::Head)));
    }
    let fits = cells(&last) + chip <= ink.width;
    let mut spans = vec![Span::styled(last, ink.style(Tone::Head))];
    if !entry.status.is_empty() {
        if !fits {
            out.push(Line::from(std::mem::take(&mut spans)));
        }
        spans.push(Span::raw(if fits { "  " } else { "" }));
        spans.push(Span::styled(
            style::chip_text(ink.look, &entry.status),
            style::chip(ink.look, style::Chip::Plain),
        ));
    }
    out.push(Line::from(spans));
    // A label that is the id already shows it whole.
    if entry.titled {
        out.push(Line::styled(
            left(&entry.id, ink.width, ink.glyphs),
            ink.style(Tone::Dim),
        ));
    }
    if ink.full && !entry.flags.is_empty() {
        let words: Vec<String> = entry.flags.iter().map(|f| format!("!{f}")).collect();
        ink.wrap(out, "", &words.join(" "), Tone::Bad, 0);
    }
}
