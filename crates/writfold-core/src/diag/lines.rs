//! Positions and text: byte offsets to lines and columns, escaping of
//! untrusted text, and wrapping at word boundaries.
use super::{At, Span};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// The byte offset of every line start, built once per file; turns a
/// byte span into a 1-based line and a column in Unicode scalar values.
#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = alloc::vec![0];
        for (at, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(at + 1);
            }
        }
        LineIndex { starts }
    }

    /// The 0-based line holding byte `offset`.
    fn row(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(n) => n,
            Err(n) => n.saturating_sub(1),
        }
    }

    /// The text of 1-based line `line`, without its line break.
    pub fn line<'t>(&self, text: &'t str, line: u32) -> &'t str {
        let n = usize::try_from(line)
            .unwrap_or(usize::MAX)
            .saturating_sub(1);
        let Some(&start) = self.starts.get(n) else {
            return "";
        };
        let end = self.starts.get(n + 1).map_or(text.len(), |e| e - 1);
        let found = text.get(start..end.max(start)).unwrap_or("");
        found.strip_suffix('\r').unwrap_or(found)
    }

    /// The line, column and length of `span` in `text`. A span that
    /// crosses a line break is cut at it.
    pub fn at(&self, text: &str, span: Span) -> At {
        let start = floor(text, usize::try_from(span.start).unwrap_or(text.len()));
        let end = floor(text, usize::try_from(span.end).unwrap_or(text.len())).max(start);
        let row = self.row(start);
        let line_start = self.starts.get(row).copied().unwrap_or(0);
        let col = text.get(line_start..start).map_or(0, |t| t.chars().count());
        let rest = text.get(start..end).unwrap_or("");
        let len = rest.split('\n').next().map_or(0, |t| t.chars().count());
        let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
        At {
            line: n(row + 1),
            col: n(col + 1),
            len: n(len),
        }
    }
}

/// The largest character boundary at or below `at`.
fn floor(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Text from a file, safe for a terminal: every control character but
/// tab (ESC included) is written `\u{…}`.
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() && c != '\t' {
            out.push_str(&format!("\\u{{{:x}}}", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

/// The display width of `text` under `cell`.
pub fn width(text: &str, cell: fn(char) -> usize) -> usize {
    text.chars().map(cell).sum()
}

/// Wrap `text` at word boundaries: the first line has `first` cells, the
/// rest `rest`. A word longer than a line stays whole.
pub fn wrap(text: &str, first: usize, rest: usize, cell: fn(char) -> usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for word in text.split(' ') {
        let size = width(word, cell);
        let room = if out.is_empty() { first } else { rest };
        if !current.is_empty() && used + 1 + size > room {
            out.push(core::mem::take(&mut current));
            used = 0;
        }
        if !current.is_empty() {
            current.push(' ');
            used += 1;
        }
        current.push_str(word);
        used += size;
    }
    out.push(current);
    out
}
