//! Width-independent inspector paragraphs; indentation also applies to wrapped rows.
use crate::app::Detail;
use crate::cells::{self, Cell, Glyphs};
use crate::graph::Node;
use crate::lines::{self, Tone};
use crate::source::field;
use crate::style::{self, Look};
use ratatui::text::{Line, Span};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct InspectLine {
    pub indent: u16,
    pub text: Line<'static>,
}

fn plain(indent: u16, text: impl Into<String>, tone: Tone, look: Look) -> InspectLine {
    InspectLine {
        indent,
        text: Line::styled(text.into(), style::style(look, tone)),
    }
}

fn cell_span(cell: Cell, look: Look) -> Span<'static> {
    Span::styled(cell.glyph, style::style(look, cell.tone))
}

fn evidence(out: &mut Vec<InspectLine>, value: &Value, indent: u16, look: Look) {
    let path = field(value, "path");
    if path.is_empty() {
        return;
    }
    let mut text = format!("@{path}");
    if let Some(line) = value.get("line").and_then(Value::as_u64) {
        text.push_str(&format!(":{line}"));
    }
    out.push(plain(indent, text, Tone::Dim, look));
}

fn continuation(out: &mut Vec<InspectLine>, value: &Value, look: Look, full: bool) {
    let label = field(value, "label");
    let indent = match label {
        "gate" => 2,
        "item" => 4,
        _ => 0,
    };
    let separator = if matches!(label, "gate" | "item") {
        " "
    } else {
        ": "
    };
    let tone = if matches!(label, "read" | "next") {
        Tone::Info
    } else {
        lines::tone(value)
    };
    out.push(plain(
        indent,
        format!(
            "{label}{separator}{}{}",
            field(value, "text"),
            lines::suffix(value, false)
        ),
        tone,
        look,
    ));
    if full {
        evidence(out, value, indent + 2, look);
    }
}

// The producer prints the marker directly after the id and authoritative state.
fn optional(item: &Value) -> bool {
    let text = field(item, "text");
    let Some((id, _)) = text.split_once(' ') else {
        return false;
    };
    let prefix = format!("{id} [{}] (optional)", field(item, "status"));
    text.strip_prefix(&prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

/// The segment strip, one cell per required item in source order. The
/// detailed form leads with the count, which the calm gate line already has.
fn checklist(
    gate: &Value,
    items: &[&Value],
    glyphs: Glyphs,
    look: Look,
    full: bool,
) -> Option<Vec<Span<'static>>> {
    let text = field(gate, "text");
    let (_, count) = text.split_once(" checklist ")?;
    let (count, _) = count.split_once(" required covered")?;
    let mut spans = Vec::new();
    if full {
        spans.push(Span::raw(format!("{count} required covered  ")));
    }
    let mut optional_count = 0;
    for item in items.iter().filter(|item| field(item, "label") == "item") {
        if optional(item) {
            optional_count += 1;
        } else {
            spans.push(cell_span(
                cells::segment(field(item, "status"), glyphs),
                look,
            ));
            spans.push(Span::raw(" "));
        }
    }
    if optional_count > 0 {
        spans.push(Span::raw(format!("(+{optional_count} optional)")));
    }
    Some(spans)
}

/// The items of the gate at `index`: rows up to the next gate.
fn items<'v>(more: &[&'v Value], index: usize) -> Vec<&'v Value> {
    let rest = &more[index + 1..];
    let end = rest
        .iter()
        .position(|item| field(item, "label") == "gate")
        .unwrap_or(rest.len());
    rest[..end].to_vec()
}

fn gates(out: &mut Vec<InspectLine>, node: &Node<'_>, glyphs: Glyphs, look: Look) {
    // A step without gates already says "no gates" in its own text.
    if !node.chips.is_empty() {
        let mut spans = vec![Span::raw(format!("gates ({})  ", node.chips.len()))];
        spans.extend(
            node.chips
                .iter()
                .map(|status| cell_span(cells::chip(status, glyphs), look)),
        );
        out.push(InspectLine {
            indent: 0,
            text: Line::from(spans),
        });
    }
    let more: Vec<_> = lines::more(node.row).collect();
    for (index, value) in more.iter().enumerate() {
        let label = field(value, "label");
        if matches!(
            label,
            "splice" | "under" | "why" | "owes" | "read" | "next" | "done" | "skip"
        ) {
            continue;
        }
        continuation(out, value, look, true);
        if label == "gate"
            && let Some(spans) = checklist(value, &items(&more, index), glyphs, look, true)
        {
            out.push(InspectLine {
                indent: 4,
                text: Line::from(spans),
            });
        }
    }
}

fn labelled(out: &mut Vec<InspectLine>, node: &Node<'_>, labels: &[&str], look: Look, full: bool) {
    for label in labels {
        for value in lines::more(node.row).filter(|value| field(value, "label") == *label) {
            continuation(out, value, look, full);
        }
    }
}

/// Text excludes `indent`; render in a narrowed rectangle. The exact
/// level shows every continuation and receipt; the plain level is
/// `inspect_plain` (without a position or the plan's owed count here).
/// The last line is the command hint.
pub fn content(node: &Node<'_>, glyphs: Glyphs, look: Look, detail: Detail) -> Vec<InspectLine> {
    if detail == Detail::Plain {
        return plain::content(node, plain::Context::default(), glyphs, look);
    }
    let status = field(node.row, "status");
    let mut out = vec![header(node, look)];
    labelled(
        &mut out,
        node,
        &["splice", "under", "why", "owes"],
        look,
        true,
    );
    if status != "skipped" && !field(node.row, "text").is_empty() {
        out.push(plain(
            0,
            field(node.row, "text"),
            lines::tone(node.row),
            look,
        ));
    }
    gates(&mut out, node, glyphs, look);
    labelled(
        &mut out,
        node,
        &["read", "next", "done", "skip"],
        look,
        true,
    );
    if status == "skipped" {
        out.push(plain(
            0,
            format!("skip: {}", field(node.row, "text")),
            Tone::Skipped,
            look,
        ));
    }
    if matches!(status, "done" | "skipped") {
        evidence(&mut out, node.row, 2, look);
    }
    out.push(InspectLine {
        indent: 0,
        text: hint(detail, u16::MAX, look),
    });
    out
}

// The pane title carries "step N/M"; the body leads with the id alone so a
// long id wraps on its own line instead of stranding a prefix.
fn header(node: &Node<'_>, look: Look) -> InspectLine {
    let status = field(node.row, "status");
    let standing = if status.is_empty() {
        String::new()
    } else {
        format!(" [{status}]")
    };
    plain(
        0,
        format!("{}{standing}{}", node.id, lines::suffix(node.row, false)),
        Tone::Head,
        look,
    )
}

#[path = "inspect_plain.rs"]
pub mod plain;
pub use plain::hint;

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
