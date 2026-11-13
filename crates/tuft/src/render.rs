//! Field rendering shared by the text and JSON forms of `emit`.
use crate::ir::Field;
use crate::view::{Row, excerpt};
use std::fmt::Write as _;
use writfold_domain::citation::Coverage;

/// A JSON string literal: `\" \\ \n \r \t` and `\u00XX` for other controls.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Paths print bare unless they hold whitespace, quotes, backslashes or
/// control characters; then they print as a quoted literal.
fn path_text(path: &str) -> String {
    if path.is_empty()
        || path
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '\\')
    {
        quote(path)
    } else {
        path.to_owned()
    }
}

/// One text row: fields in projection order, separated by one space.
/// An excerpt prints as `LO..HI:"text"`, the exact file range shown, and a
/// truncated one ends with `…+NB`, the retained bytes it does not show.
pub fn row_text(row: &Row, fields: &[Field]) -> String {
    let mut parts = Vec::with_capacity(fields.len());
    for field in fields {
        parts.push(match *field {
            Field::Path => path_text(&row.path),
            Field::Line => row.line.to_string(),
            Field::Range => format!("{}..{}", row.range.0, row.range.1),
            Field::Binding => row
                .evidence
                .as_ref()
                .map_or("-".to_owned(), |e| e.binding.clone()),
            Field::Excerpt { max_chars } => match excerpt(row, max_chars) {
                Some(x) => {
                    let marker = if x.hidden > 0 {
                        format!("…+{}B", x.hidden)
                    } else {
                        String::new()
                    };
                    format!("{}..{}:{}{marker}", x.range.0, x.range.1, quote(&x.text))
                }
                None => "-".to_owned(),
            },
        });
    }
    parts.join(" ")
}

pub fn row_json(row: &Row, fields: &[Field]) -> String {
    let mut parts = Vec::with_capacity(fields.len());
    for field in fields {
        let value = match *field {
            Field::Path => quote(&row.path),
            Field::Line => row.line.to_string(),
            Field::Range => format!("[{},{}]", row.range.0, row.range.1),
            Field::Binding => row
                .evidence
                .as_ref()
                .map_or("null".to_owned(), |e| quote(&e.binding)),
            Field::Excerpt { max_chars } => {
                excerpt(row, max_chars).map_or("null".to_owned(), |x| {
                    format!(
                        "{{\"range\":[{},{}],\"text\":{},\"truncated\":{},\"hidden_bytes\":{}}}",
                        x.range.0,
                        x.range.1,
                        quote(&x.text),
                        x.hidden > 0,
                        x.hidden
                    )
                })
            }
        };
        parts.push(format!("{}:{value}", quote(field.name())));
    }
    format!("{{{}}}", parts.join(","))
}

pub fn coverage_text(search: &crate::view::SearchReport) -> String {
    if let Some(index) = &search.index {
        return format!(
            "coverage=partial index_commit={} index_sha256={}",
            index.commit, index.sha256
        );
    }
    let coverage = search.coverage;
    match coverage {
        Coverage::Unavailable => "coverage=unavailable".into(),
        Coverage::Measured { examined, total } => format!(
            "coverage={} files={examined}/{total}",
            if coverage.complete() {
                "complete"
            } else {
                "partial"
            }
        ),
    }
}

pub fn coverage_json(search: &crate::view::SearchReport) -> String {
    if let Some(index) = &search.index {
        return format!(
            "{{\"state\":\"partial\",\"index_commit\":{},\"index_sha256\":{}}}",
            quote(&index.commit),
            quote(&index.sha256)
        );
    }
    let coverage = search.coverage;
    match coverage {
        Coverage::Unavailable => "{\"state\":\"unavailable\"}".into(),
        Coverage::Measured { examined, total } => format!(
            "{{\"state\":\"{}\",\"examined_files\":{examined},\"total_files\":{total}}}",
            if coverage.complete() {
                "complete"
            } else {
                "partial"
            }
        ),
    }
}
