//! The TOML subset every definition file uses: top-level `key = value`
//! lines, comments, basic and literal strings, integers, and arrays of
//! strings (which may span lines). Anything else is reported, never
//! guessed at; every file in the subset is valid TOML and means the same
//! thing to a full TOML parser.
//!
//! Every key and value keeps its byte span. A fault is reported and
//! reading goes on at a line boundary: the entry it broke is
//! [`Slot::Broken`], which silences every later check on that key.
use super::Sink;
use super::lex::{Cursor, Fault, integer, string};
use writfold_core::diag::Span;

pub use super::doc::{Doc, Entry, Slot, Spanned, Text, Value};

/// How reading goes on after a fault in a value.
pub enum Resume {
    /// At the next line.
    Line,
    /// Past the array's closing line, or at the next key.
    Array,
    /// Where the cursor is (an unclosed array, closed at its last item).
    Here,
}

pub type Read<T> = Result<T, (Fault, Resume)>;

/// Read `text`, reporting every fault to `sink`.
pub fn read(text: &str, sink: &mut Sink<'_>) -> Doc {
    let mut cur = Cursor { text, pos: 0 };
    let mut doc = Doc::default();
    loop {
        cur.space();
        let Some(first) = cur.peek() else {
            return doc;
        };
        let start = cur.pos;
        let entry = match entry(&mut cur) {
            Ok(entry) => entry,
            Err((fault, resume)) => {
                sink.fault(fault);
                skip(&mut cur, resume);
                match key_of(text, start) {
                    Some(key) if first != '[' => Entry {
                        key,
                        value: Slot::Broken,
                    },
                    _ => continue,
                }
            }
        };
        twice(&mut doc, entry, sink);
    }
}

fn twice(doc: &mut Doc, entry: Entry, sink: &mut Sink<'_>) {
    let Some(first) = doc.get(&entry.key.value) else {
        doc.entries.push(entry);
        return;
    };
    let line = sink.at(first.key.span).line;
    let why = format!("`{}` is given twice", entry.key.value);
    let d = sink.error("key.twice", Some(entry.key.span), why);
    d.label("given again");
    d.help("remove one of the two lines".into());
    d.note(format!("first given on line {line}"));
}

/// The key a broken line starts with, if it has one followed by `=`.
fn key_of(text: &str, start: usize) -> Option<Spanned<String>> {
    let cur = Cursor { text, pos: start };
    if !cur.at_key_line() {
        return None;
    }
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(rest.len());
    let span = Span::new(start, start + end);
    let value = rest[..end].to_owned();
    Some(Spanned { value, span })
}

fn skip(cur: &mut Cursor<'_>, resume: Resume) {
    match resume {
        Resume::Here => {}
        Resume::Line => cur.skip_line(),
        Resume::Array => loop {
            let end = cur.line_end();
            if cur.text[cur.pos..end].contains(']') || cur.pos >= cur.text.len() {
                cur.skip_line();
                return;
            }
            cur.skip_line();
            let mut probe = Cursor {
                text: cur.text,
                pos: cur.pos,
            };
            probe.space();
            if probe.at_key_line() || probe.peek().is_none() {
                cur.pos = probe.pos;
                return;
            }
        },
    }
}

/// A fault from `from` to the end of the line; reading goes on at the
/// next line.
pub fn line_fault(code: &'static str, cur: &Cursor<'_>, from: usize, why: &str) -> (Fault, Resume) {
    let end = cur.line_end().max(from);
    let fault = Fault::new(code, Span::new(from, end), why.into(), "");
    (fault, Resume::Line)
}

/// A fault on the one character at the cursor (none at a line end).
pub fn here_fault(cur: &Cursor<'_>, why: String, label: &'static str) -> (Fault, Resume) {
    let at = cur.pos;
    let size = cur.peek().filter(|c| !matches!(c, '\n' | '\r'));
    let span = Span::new(at, at + size.map_or(0, char::len_utf8));
    (
        Fault::new("syntax.expected", span, why, label),
        Resume::Line,
    )
}

fn key(cur: &mut Cursor<'_>) -> Read<Spanned<String>> {
    let start = cur.pos;
    if cur.peek() == Some('[') {
        let (fault, resume) = line_fault(
            "syntax.subset",
            cur,
            start,
            "tables are not used in these files",
        );
        let fault = fault.help("remove this line: keys are written at the top level".into());
        return Err((fault, resume));
    }
    while matches!(cur.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        cur.bump();
    }
    let value = cur.text[start..cur.pos].to_owned();
    let span = Span::new(start, cur.pos);
    if !value.is_empty() {
        return Ok(Spanned { value, span });
    }
    Err(match cur.peek() {
        Some('"' | '\'') => line_fault(
            "syntax.subset",
            cur,
            start,
            "quoted keys are not used in these files",
        ),
        _ => here_fault(cur, "expected a key".into(), "expected `key = value`"),
    })
}

/// One `key = value` line.
fn entry(cur: &mut Cursor<'_>) -> Read<Entry> {
    let start = cur.pos;
    let key = key(cur)?;
    cur.blank();
    if cur.peek() == Some('.') {
        let (fault, resume) = line_fault(
            "syntax.subset",
            cur,
            start,
            "dotted keys are not used in these files",
        );
        return Err((fault.help("write one top-level key".into()), resume));
    }
    if cur.peek() != Some('=') {
        let why = format!("expected `=` after `{}`", key.value);
        return Err(here_fault(cur, why, "expected `=`"));
    }
    cur.bump();
    cur.blank();
    let value = value(cur)?;
    cur.blank();
    match cur.peek() {
        None => {}
        Some('\n') => {
            cur.bump();
        }
        Some('\r') if cur.text[cur.pos..].starts_with("\r\n") => {
            cur.pos += 2;
        }
        Some(_) => {
            let why = "expected the end of the line";
            let (mut fault, resume) = line_fault("syntax.expected", cur, cur.pos, why);
            fault.label = "unexpected after the value";
            return Err((fault, resume));
        }
    }
    let value = Slot::Value(value);
    Ok(Entry { key, value })
}

fn value(cur: &mut Cursor<'_>) -> Read<Spanned<Value>> {
    match cur.peek() {
        Some('"' | '\'') => {
            let s = string(cur).map_err(|f| (f, Resume::Line))?;
            let span = s.span;
            Ok(Spanned {
                value: Value::Str(s.value),
                span,
            })
        }
        Some('[') => super::array::array(cur),
        Some(c) if c.is_ascii_digit() || c == '+' || c == '-' => {
            let n = integer(cur).map_err(|f| (f, Resume::Line))?;
            Ok(Spanned {
                value: Value::Int(n.value),
                span: n.span,
            })
        }
        Some('{') => Err(line_fault(
            "syntax.subset",
            cur,
            cur.pos,
            "inline tables are not used in these files",
        )),
        Some(c) if c.is_alphabetic() => Err((super::array::word(cur), Resume::Line)),
        _ => {
            let label = "expected a string, an integer or an array of strings";
            Err(here_fault(cur, "expected a value after `=`".into(), label))
        }
    }
}
