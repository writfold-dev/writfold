//! Arrays of strings, which may span lines, and the bare words the subset
//! does not have (booleans, `inf`, an unquoted name).
//!
//! An array whose `]` is missing is found where it matters: when a line
//! that starts `key =` comes where `,` or `]` was expected, the array is
//! reported once, at the end of its last item, closed there, and the line
//! is read as the next entry, so the keys after it are still checked.
use super::lex::{Cursor, Fault, string};
use super::toml::{Read, Resume, Spanned, Text, Value, here_fault};
use writfold_core::diag::{LineIndex, Span};

fn unclosed(cur: &Cursor<'_>, open: usize, last: usize) -> (Fault, Resume) {
    let index = LineIndex::new(cur.text);
    let opened = index.at(cur.text, Span::new(open, open));
    let end = index.at(cur.text, Span::new(last, last));
    let why = "this array is never closed".into();
    let mut fault = Fault::new(
        "syntax.unclosed-array",
        Span::new(last, last),
        why,
        "expected `,` or `]` here",
    );
    fault.help = Some(format!("add `]` at the end of line {}", end.line));
    let next = match cur.peek() {
        None => "the file ends before it closes".to_owned(),
        Some(_) => {
            let rest = &cur.text[cur.pos..];
            let key: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            let line = index.at(cur.text, Span::new(cur.pos, cur.pos)).line;
            format!("line {line} starts the next key, `{key}`")
        }
    };
    let opens = format!("{}:{}", opened.line, opened.col);
    fault
        .notes
        .push(format!("the array opens at {opens}, and {next}"));
    (fault, Resume::Here)
}

/// Whether a line break lies between `from` and the cursor.
fn crossed(cur: &Cursor<'_>, from: usize) -> bool {
    cur.text[from..cur.pos].contains('\n')
}

fn array_fault(cur: &Cursor<'_>, why: &str, label: &'static str) -> (Fault, Resume) {
    let (fault, _) = here_fault(cur, why.into(), label);
    (fault, Resume::Array)
}

/// `[` … `]`, the cursor on the `[`.
pub fn array(cur: &mut Cursor<'_>) -> Read<Spanned<Value>> {
    let open = cur.pos;
    cur.bump();
    let mut items: Vec<Spanned<Text>> = Vec::new();
    let mut last = cur.pos;
    loop {
        cur.space();
        match cur.peek() {
            Some(']') => break,
            None => return Err(unclosed(cur, open, last)),
            Some(_) if crossed(cur, last) && cur.at_key_line() => {
                return Err(unclosed(cur, open, last));
            }
            Some('"' | '\'') => {
                let item = string(cur).map_err(|f| (f, Resume::Array))?;
                last = cur.pos;
                items.push(item);
            }
            Some(_) => {
                let label = "arrays hold strings only";
                let mut out = array_fault(cur, "arrays in these files hold strings only", label);
                out.0.code = "syntax.subset";
                out.0.span = word_span(cur);
                return Err(out);
            }
        }
        cur.space();
        match cur.peek() {
            Some(',') => {
                cur.bump();
                last = cur.pos;
            }
            Some(']') => break,
            None => return Err(unclosed(cur, open, last)),
            Some(_) if crossed(cur, last) && cur.at_key_line() => {
                return Err(unclosed(cur, open, last));
            }
            Some(_) => {
                return Err(array_fault(
                    cur,
                    "expected `,` or `]` in an array",
                    "expected `,` or `]`",
                ));
            }
        }
    }
    cur.bump();
    let span = Span::new(open, cur.pos);
    Ok(Spanned {
        value: Value::List(items),
        span,
    })
}

fn word_span(cur: &Cursor<'_>) -> Span {
    let rest = &cur.text[cur.pos..];
    let len = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | ']' | '#'))
        .unwrap_or(rest.len())
        .max(rest.chars().next().map_or(0, char::len_utf8));
    Span::new(cur.pos, cur.pos + len)
}

/// A bare word where a value belongs.
pub fn word(cur: &Cursor<'_>) -> Fault {
    let span = word_span(cur);
    let text =
        &cur.text[usize::try_from(span.start).unwrap_or(0)..usize::try_from(span.end).unwrap_or(0)];
    match text {
        "true" | "false" => {
            let fault = Fault::new(
                "syntax.subset",
                span,
                "booleans are not used in these files".into(),
                "",
            );
            fault.help("these files have no booleans; write the value this key takes as a string or an integer".into())
        }
        "inf" | "nan" | "+inf" | "-inf" => Fault::new(
            "syntax.subset",
            span,
            "floats are not used in these files".into(),
            "",
        ),
        _ => {
            let why = format!("`{text}` is not a value: strings are quoted");
            let fault = Fault::new(
                "syntax.expected",
                span,
                why,
                "expected a string, an integer or an array",
            );
            fault.help(format!("write it in quotes: \"{text}\""))
        }
    }
}
