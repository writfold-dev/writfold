//! JSON-lines files a person writes (`.writfold/queries.jsonl`, the rows
//! of `gate cover --from`): each line one object. A line that is not JSON
//! is reported at the parser's own column; a key and its value are found
//! by a small scan of the raw line, so a fault points at the key or value
//! it is about.
use super::{Field, Sink};
use serde_json::{Map, Value};
use writfold_core::diag::Span;
use writfold_core::diag::suggest::{did_you_mean, listed, suggest};

/// One object line: its 1-based number, its byte offset in the file, its
/// text and its keys.
pub struct Line<'t> {
    pub number: usize,
    pub start: usize,
    pub raw: &'t str,
    pub map: Map<String, Value>,
}

impl Line<'_> {
    /// The whole line.
    pub fn span(&self) -> Span {
        Span::new(self.start, self.start + self.raw.len())
    }

    /// Where `"key"` is written, quotes included.
    pub fn key(&self, key: &str) -> Option<Span> {
        let quoted = serde_json::to_string(key).ok()?;
        let mut from = 0;
        while let Some(at) = self.raw[from..].find(&quoted).map(|a| a + from) {
            let after = self.raw[at + quoted.len()..].trim_start();
            if after.starts_with(':') {
                return Some(Span::new(self.start + at, self.start + at + quoted.len()));
            }
            from = at + quoted.len();
        }
        None
    }

    /// Where the value of `key` is written (or its key, when the value
    /// cannot be delimited).
    pub fn value(&self, key: &str) -> Option<Span> {
        let k = self.key(key)?;
        let rel = usize::try_from(k.end).ok()? - self.start;
        let rest = &self.raw[rel..];
        let colon = rest.find(':')? + 1;
        let lead = rest[colon..].len() - rest[colon..].trim_start().len();
        let begin = rel + colon + lead;
        let len = extent(&self.raw[begin..]);
        Some(Span::new(self.start + begin, self.start + begin + len))
    }
}

/// The length of the JSON value at the start of `text`: a string to its
/// closing quote, an array or object to its closing bracket, anything
/// else to the next `,`, `}` or `]`.
fn extent(text: &str) -> usize {
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for (i, c) in text.char_indices() {
        if quoted {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => {
                    quoted = false;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '[' | '{' => depth += 1,
            ']' | '}' if depth > 1 => depth -= 1,
            ']' | '}' if depth == 1 => return i + 1,
            ',' | ']' | '}' => return i,
            _ => {}
        }
    }
    text.trim_end().len()
}

/// The object lines of `text`, blank lines skipped; a line that is not a
/// JSON object is reported (`syntax.json`) and left out.
pub fn objects<'t>(sink: &mut Sink<'_>, text: &'t str, what: &str) -> Vec<Line<'t>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (n, raw) in text.split('\n').enumerate() {
        let here = start;
        start += raw.len() + 1;
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if raw.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(raw) {
            Ok(Value::Object(map)) => out.push(Line {
                number: n + 1,
                start: here,
                raw,
                map,
            }),
            Ok(_) => {
                let why = format!("{what} is a JSON object");
                let span = Span::new(here, here + raw.len());
                sink.error("syntax.json", Some(span), why)
                    .label("not an object");
            }
            Err(e) => {
                let at = here + e.column().saturating_sub(1).min(raw.len());
                let next = raw
                    .get(at - here..)
                    .and_then(|r| r.chars().next())
                    .map_or(0, char::len_utf8);
                let text = e.to_string();
                let why = text.split(" at line ").next().unwrap_or(&text).to_owned();
                let d = sink.error(
                    "syntax.json",
                    Some(Span::new(at, at + next)),
                    format!("this line is not JSON: {why}"),
                );
                d.label("here");
            }
        }
    }
    out
}

/// Report every key of `line` not in `allowed`, with a suggestion.
pub fn unknown(sink: &mut Sink<'_>, line: &Line<'_>, allowed: &[&str], label: &str) -> bool {
    let mut ok = true;
    for key in line.map.keys().filter(|k| !allowed.contains(&k.as_str())) {
        let d = sink.error(
            "key.unknown",
            line.key(key).or(Some(line.span())),
            format!("unknown key `{key}`"),
        );
        d.label(label);
        match did_you_mean(&suggest(key, allowed), '`') {
            Some(help) => d.help = Some(help),
            None => d
                .notes
                .extend(listed(allowed).map(|l| format!("the keys are {l}"))),
        }
        ok = false;
    }
    ok
}

/// `key` must be a string when present (`null` counts as absent).
pub fn text(sink: &mut Sink<'_>, line: &Line<'_>, key: &str) -> Field<String> {
    match line.map.get(key) {
        None | Some(Value::Null) => Field::Absent,
        Some(Value::String(s)) => Field::Got(s.clone()),
        Some(_) => {
            let why = format!("`{key}` must be a string");
            sink.error("key.type", line.value(key), why)
                .label("expected a string");
            Field::Bad
        }
    }
}

/// An optional string: `Some(None)` when absent, `None` when bad.
pub fn optional(field: Field<String>) -> Option<Option<String>> {
    match field {
        Field::Got(s) => Some(Some(s)),
        Field::Absent => Some(None),
        Field::Bad => None,
    }
}

/// A required key absent from `line`, reported at the line.
pub fn missing(sink: &mut Sink<'_>, line: &Line<'_>, key: &str, needs: &str, add: &str) {
    // An unknown key that was said to mean `key` already names the fault.
    if line
        .map
        .keys()
        .any(|k| k != key && suggest(k, &[key]) == [key])
    {
        return;
    }
    let d = sink.error(
        "key.missing",
        Some(line.span()),
        format!("{needs} needs `{key}`"),
    );
    d.label("in this row");
    d.help(format!("add {add}"));
}
