//! The plain level's phrases. A piece is either the viewer's own words
//! (`Said`, from the tables in `say_words`), a verb's or an author's text
//! passed through whole (`Quoted`), or a state glyph (`Mark`). Pure: no
//! terminal, no app state.
use crate::cells::Cell;
use crate::lines::Tone;

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

/// True when no whitespace-separated token of `text` is notation.
pub fn readable(text: &str) -> bool {
    if text.contains("writfold kb") || text.contains('`') || text.contains('→') {
        return false;
    }
    text.split_whitespace().all(plain_token)
}

/// `Quoted` when the text reads as words, else nothing: whole or nothing.
pub fn quote(text: &str) -> Option<Piece> {
    (!text.is_empty() && readable(text)).then(|| Piece::Quoted(text.to_owned()))
}

fn plain_token(token: &str) -> bool {
    let chars: Vec<char> = token.chars().collect();
    let flagged = |lead: &str| {
        token
            .strip_prefix(lead)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_alphabetic())
    };
    let bracket = token
        .find('[')
        .is_some_and(|open| token[open..].contains(']'));
    !(bracket
        || token.starts_with('@')
        || flagged("--")
        || flagged("!")
        || between(&chars, '=', char::is_alphanumeric)
        || between(&chars, '_', char::is_alphabetic)
        || run(&chars, |c| c.is_ascii_digit()) >= 8
        || run(&chars, |c| c.is_ascii_hexdigit()) >= 12
        || iso(token)
        || typed(&chars)
        || token.split('/').filter(|s| !s.is_empty()).count() >= 3)
}

/// `mark` with a matching character on both sides.
fn between(chars: &[char], mark: char, kind: fn(char) -> bool) -> bool {
    chars
        .windows(3)
        .any(|w| w[1] == mark && kind(w[0]) && kind(w[2]))
}

fn run(chars: &[char], kind: impl Fn(char) -> bool) -> usize {
    let mut best = 0;
    let mut now = 0;
    for c in chars {
        now = if kind(*c) { now + 1 } else { 0 };
        best = best.max(now);
    }
    best
}

/// `DDDD-DD-DDTDD:DD` anywhere in the token.
fn iso(token: &str) -> bool {
    let shape = "dddd-dd-ddTdd:dd";
    let bytes = token.as_bytes();
    bytes.windows(shape.len()).any(|w| {
        w.iter().zip(shape.bytes()).all(|(b, s)| match s {
            b'd' => b.is_ascii_digit(),
            other => *b == other,
        })
    })
}

/// A lowercase word directly followed by `:` and more text (`gate:lint`).
fn typed(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(at, c)| {
        *c == ':'
            && at > 0
            && chars[at - 1].is_ascii_lowercase()
            && chars.get(at + 1).is_some_and(|n| !n.is_whitespace())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_refuses_notation() {
        for text in [
            "1 reference by agent; cited evidence moved: gate:lint was satisfied, now stale",
            "mechanical pass exit=0 at kb@0ea3a743bfef scope=a61da1b21345 paths=1",
            "created=2026-11-10T23:59:00Z session=s-progress",
            "project:harbor",
            "writfold kb gate run lint",
            "unit [satisfied] mechanical",
            "@runs/x.json:1",
            "see --by WHO",
            "flag !stale here",
            "commit 0ea3a743bfef1",
            "run 202611010900",
            "at 2026-11-10T23:59",
            "decided_by ops",
            "proposal/decision/2026-03-02-batch",
            "a `tick`",
            "a → b",
        ] {
            assert!(!readable(text), "{text}");
        }
        for text in [
            "changed in scope: harbor/lint.txt",
            "1 reference by agent",
            "entry 3 of 3: Review started.",
            "Review started; lint went stale after the rename.",
            "ticket HBR-210: Lint gate scope misses the command module",
            "at 14:10 on 2026-11-10",
        ] {
            assert!(readable(text), "{text}");
        }
        assert_eq!(quote(""), None);
        assert_eq!(quote("a=b"), None);
    }

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
