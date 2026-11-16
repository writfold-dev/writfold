//! The plain level's phrases. A piece is either the viewer's own words
//! (`Said`, from the tables in `say_words`), a verb's or an author's text
//! passed through whole (`Quoted`), or a state glyph (`Mark`). Pure: no
//! terminal, no app state.
use crate::cells::Cell;
use crate::lines::Tone;
pub use crate::say_read::{quote, readable};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Said(String),
    Quoted(String),
    Mark(Cell),
}

impl Piece {
    pub fn text(&self) -> &str {
        match self {
            Self::Said(text) | Self::Quoted(text) => text,
            Self::Mark(cell) => cell.glyph,
        }
    }
}

pub fn said(text: impl Into<String>) -> Piece {
    Piece::Said(text.into())
}

/// One line: `lead` pieces (a mark and a name) form the first column,
/// the rest follow, joined with ` · `, at `column` cells (0: right after
/// the lead). Continuation rows hang under the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phrase {
    pub indent: u16,
    pub tone: Tone,
    pub pieces: Vec<Piece>,
    pub lead: usize,
    pub column: u16,
}

impl Phrase {
    /// A line with no lead column.
    pub fn new(indent: u16, tone: Tone, pieces: Vec<Piece>) -> Self {
        Self {
            indent,
            tone,
            pieces,
            lead: 0,
            column: 0,
        }
    }

    /// A line whose first `lead` pieces form a column.
    pub fn led(indent: u16, tone: Tone, pieces: Vec<Piece>, lead: usize) -> Self {
        Self {
            lead: lead.min(pieces.len()),
            ..Self::new(indent, tone, pieces)
        }
    }

    pub fn blank() -> Self {
        Self::new(0, Tone::Plain, Vec::new())
    }

    /// The lead as text: its pieces joined by one space.
    pub fn lead_text(&self) -> String {
        join(&self.pieces[..self.lead], " ")
    }

    /// The rest as text: its pieces joined by ` · `.
    pub fn rest_text(&self) -> String {
        join(&self.pieces[self.lead..], " · ")
    }

    /// The whole line as text, before wrapping.
    pub fn text(&self) -> String {
        let lead = self.lead_text();
        let rest = self.rest_text();
        let mut out = " ".repeat(usize::from(self.indent));
        out.push_str(&lead);
        if !lead.is_empty() && !rest.is_empty() {
            let at = usize::from(self.indent) + crate::rail_text::width(&lead);
            out.push_str(&" ".repeat(usize::from(self.column).saturating_sub(at).max(3)));
        }
        out.push_str(&rest);
        out
    }
}

fn join(pieces: &[Piece], gap: &str) -> String {
    pieces
        .iter()
        .map(Piece::text)
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(gap)
}

/// Align the rest of every led phrase in `lines` at one column: the widest
/// lead plus three cells.
pub fn align(lines: &mut [Phrase]) {
    let widest = lines
        .iter()
        .filter(|p| p.lead > 0)
        .map(|p| usize::from(p.indent) + crate::rail_text::width(&p.lead_text()))
        .max()
        .unwrap_or(0);
    let column = u16::try_from(widest + 3).unwrap_or(u16::MAX);
    for line in lines.iter_mut().filter(|p| p.lead > 0) {
        line.column = column;
    }
}

/// Phrases and the number of verb lines withheld because they could not be
/// shown whole in plain words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Told {
    pub lines: Vec<Phrase>,
    pub withheld: usize,
    pub notes: usize,
}

impl Told {
    /// The closing line for whatever was withheld, if anything was.
    pub fn closing(&self) -> Option<Phrase> {
        let line = |n: usize, what: &str| {
            let s = if n == 1 { "" } else { "s" };
            format!("{n} {what}{s} in exact wording (d)")
        };
        let text = match (self.withheld, self.notes) {
            (0, 0) => return None,
            (n, 0) => line(n, "line"),
            (0, n) => line(n, "note"),
            (l, n) => format!("{} and {}", line(l, "line"), line(n, "note")),
        };
        Some(Phrase::new(0, Tone::Dim, vec![said(text)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_led_phrase_pads_to_its_column() {
        let mut lines = vec![
            Phrase::led(0, Tone::Plain, vec![said("ab"), said("x")], 1),
            Phrase::led(2, Tone::Plain, vec![said("abcd"), said("y"), said("z")], 1),
        ];
        align(&mut lines);
        assert_eq!(lines[0].text(), "ab       x");
        assert_eq!(lines[1].text(), "  abcd   y · z");
        let told = Told {
            withheld: 2,
            ..Told::default()
        };
        let closing = told.closing().map(|p| p.text());
        assert_eq!(closing.as_deref(), Some("2 lines in exact wording (d)"));
    }
}
