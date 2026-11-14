//! Width-independent inspector paragraphs; indentation also applies to wrapped rows.
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

fn continuation(out: &mut Vec<InspectLine>, value: &Value, look: Look) {
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
    evidence(out, value, indent + 2, look);
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

fn checklist(gate: &Value, items: &[&Value], glyphs: Glyphs, look: Look) -> Option<InspectLine> {
    let text = field(gate, "text");
    let (_, count) = text.split_once(" checklist ")?;
    let (count, _) = count.split_once(" required covered")?;
    let mut spans = vec![Span::raw(format!("{count} required covered  "))];
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
    Some(InspectLine {
        indent: 4,
        text: Line::from(spans),
    })
}

fn gates(out: &mut Vec<InspectLine>, node: &Node<'_>, glyphs: Glyphs, look: Look) {
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
    let more: Vec<_> = lines::more(node.row).collect();
    for (index, value) in more.iter().enumerate() {
        let label = field(value, "label");
        if matches!(
            label,
            "splice" | "under" | "why" | "owes" | "read" | "next" | "done" | "skip"
        ) {
            continue;
        }
        continuation(out, value, look);
        if label == "gate" {
            let rest = &more[index + 1..];
            let end = rest
                .iter()
                .position(|item| field(item, "label") == "gate")
                .unwrap_or(rest.len());
            out.extend(checklist(value, &rest[..end], glyphs, look));
        }
    }
}

fn labelled(out: &mut Vec<InspectLine>, node: &Node<'_>, labels: &[&str], look: Look) {
    for label in labels {
        for value in lines::more(node.row).filter(|value| field(value, "label") == *label) {
            continuation(out, value, look);
        }
    }
}

/// `index` is zero-based. Text excludes `indent`; render in a narrowed rectangle.
pub fn content(
    node: &Node<'_>,
    index: usize,
    total: usize,
    glyphs: Glyphs,
    look: Look,
) -> Vec<InspectLine> {
    let status = field(node.row, "status");
    let mut out = vec![header(node, index, total, glyphs, look)];
    labelled(&mut out, node, &["splice", "under", "why", "owes"], look);
    if status != "skipped" && !field(node.row, "text").is_empty() {
        out.push(plain(
            0,
            field(node.row, "text"),
            lines::tone(node.row),
            look,
        ));
    }
    gates(&mut out, node, glyphs, look);
    labelled(&mut out, node, &["read", "next", "done", "skip"], look);
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
    out.push(plain(
        0,
        "full receipts: g   all steps as text: x",
        Tone::Dim,
        look,
    ));
    out
}

fn header(node: &Node<'_>, index: usize, total: usize, glyphs: Glyphs, look: Look) -> InspectLine {
    let separator = match glyphs {
        Glyphs::Unicode => "·",
        Glyphs::Ascii => "-",
    };
    let status = field(node.row, "status");
    let standing = if status.is_empty() {
        String::new()
    } else {
        format!(" [{status}]")
    };
    plain(
        0,
        format!(
            "step {}/{total} {separator} {}{standing}{}",
            index + 1,
            node.id,
            lines::suffix(node.row, false)
        ),
        Tone::Head,
        look,
    )
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
