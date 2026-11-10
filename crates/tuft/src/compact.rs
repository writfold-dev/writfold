//! The compact view: grep-like rows and one trailer line.
//!
//! A row is the projection's non-excerpt fields joined by `:`, then `: ` and
//! the excerpt text, e.g. `src/lib.rs:12: let a = 1;`. Text prints raw when
//! that is unambiguous and as a JSON string literal otherwise (see
//! [`excerpt`]), so a reader recovers the exact shown bytes either way.
use crate::ir::Field;
use crate::render::quote;
use crate::view::{Excerpt, Row, excerpt as cut};

/// Paths print raw unless empty, starting with `"`, or holding `:` or a
/// control character; then they print as a JSON string literal.
fn path(path: &str) -> String {
    if path.is_empty() || path.starts_with('"') || path.chars().any(|c| c == ':' || c.is_control())
    {
        quote(path)
    } else {
        path.to_owned()
    }
}

/// The shown text of an excerpt. With `lines` retention an untruncated
/// excerpt is whole lines, and its one final LF is the row end, not printed.
/// The text prints raw unless it would be ambiguous: it holds a control
/// character other than tab, starts with `"`, holds `…`, or is an
/// untruncated line excerpt without a final LF (the end of a file). Then
/// the exact shown bytes print as a JSON string literal. A truncated excerpt
/// ends with `…+NB`, the retained bytes not shown.
pub fn excerpt(x: &Excerpt, lines: bool) -> String {
    let body = if lines && x.hidden == 0 {
        x.text.strip_suffix('\n')
    } else {
        Some(x.text.as_str())
    };
    let raw = body.filter(|b| {
        !b.starts_with('"') && !b.contains('…') && !b.chars().any(|c| c.is_control() && c != '\t')
    });
    let mut out = raw.map_or_else(|| quote(&x.text), str::to_owned);
    if x.hidden > 0 {
        out.push_str(&format!("…+{}B", x.hidden));
    }
    out
}

/// One compact row. `binding` and `range` appear only when projected;
/// `range` is the retained (or hit) range, so the shown range is
/// `LO..HI-N` for a row truncated by `N` bytes.
pub fn row(row: &Row, fields: &[Field], lines: bool) -> String {
    let mut prefix = Vec::with_capacity(fields.len());
    let mut text = None;
    for field in fields {
        match *field {
            Field::Path => prefix.push(path(&row.path)),
            Field::Line => prefix.push(row.line.to_string()),
            Field::Range => prefix.push(format!("{}..{}", row.range.0, row.range.1)),
            Field::Binding => prefix.push(
                row.evidence
                    .as_ref()
                    .map_or("-".to_owned(), |e| e.binding.clone()),
            ),
            Field::Excerpt { max_chars } => {
                text = Some(cut(row, max_chars).map_or("-".to_owned(), |x| excerpt(&x, lines)));
            }
        }
    }
    let prefix = prefix.join(":");
    match text {
        None => prefix,
        Some(text) if prefix.is_empty() => text,
        Some(text) => format!("{prefix}: {text}"),
    }
}
