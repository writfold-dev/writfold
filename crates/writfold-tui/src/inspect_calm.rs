//! The calm inspector: short id and status chip, then only what needs
//! attention. Covered items and receipts stay one `d` away.
use super::{InspectLine, cell_span, checklist, items, optional, plain};
use crate::app::Detail;
use crate::cells::{self, Glyphs};
use crate::graph::Node;
use crate::lines::{self, Tone};
use crate::rail_text::shown_id;
use crate::source::{field, flags};
use crate::style::{self, Chip, Look};
use ratatui::text::{Line, Span};
use serde_json::Value;

/// Items whose status is settled; the strip still shows them.
const COVERED: [&str; 3] = ["supported", "not-applicable", "attested"];

fn blank() -> InspectLine {
    InspectLine {
        indent: 0,
        text: Line::default(),
    }
}

/// The id as the rail shows it, the status as a chip, the row's flags,
/// then the full id muted on its own line.
pub(super) fn header(node: &Node<'_>, look: Look) -> Vec<InspectLine> {
    let status = field(node.row, "status");
    let mut spans = vec![Span::styled(
        shown_id(node.id, node.group, false).to_owned(),
        style::style(look, Tone::Head),
    )];
    if !status.is_empty() {
        let kind = if status == "current" {
            Chip::Accent
        } else {
            Chip::Plain
        };
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            style::chip_text(look, status),
            style::chip(look, kind),
        ));
    }
    let suffix = lines::suffix(node.row, false);
    if !suffix.is_empty() {
        spans.push(Span::styled(suffix, style::style(look, Tone::Bad)));
    }
    vec![
        InspectLine {
            indent: 0,
            text: Line::from(spans),
        },
        plain(0, node.id, Tone::Dim, look),
        blank(),
    ]
}

/// Each gate as the verb words it (its bracketed state already says what a
/// trailing flag would), the strip, then the items still open or stale.
pub(super) fn gates(out: &mut Vec<InspectLine>, node: &Node<'_>, glyphs: Glyphs, look: Look) {
    let more: Vec<_> = lines::more(node.row).collect();
    let start = out.len();
    for (index, value) in more.iter().enumerate() {
        if field(value, "label") != "gate" {
            continue;
        }
        if out.len() == start && out.last().is_some_and(|l| l.text.width() > 0) {
            out.push(blank());
        }
        let text = format!("gate {}", field(value, "text"));
        out.push(plain(0, text, lines::tone(value), look));
        let items = items(&more, index);
        if let Some(spans) = checklist(value, &items, glyphs, look, false) {
            out.push(InspectLine {
                indent: 2,
                text: Line::from(spans),
            });
        }
        let open = items.iter().filter(|item| attention(item));
        out.extend(open.map(|item| item_line(item, glyphs, look)));
    }
    if out.len() > start {
        out.push(blank());
    }
}

fn attention(item: &Value) -> bool {
    field(item, "label") == "item" && !COVERED.contains(&field(item, "status"))
}

fn item_line(item: &Value, glyphs: Glyphs, look: Look) -> InspectLine {
    let status = field(item, "status");
    let tone = if optional(item) && flags(item).is_empty() {
        Tone::Dim
    } else {
        lines::tone(item)
    };
    InspectLine {
        indent: 2,
        text: Line::from(vec![
            cell_span(cells::segment(status, glyphs), look),
            Span::styled(
                format!(" {}", field(item, "text")),
                style::style(look, tone),
            ),
        ]),
    }
}

/// The command hint, shortened to fit so no key is cut off.
pub fn hint(detail: Detail, width: u16, look: Look) -> Line<'static> {
    let forms: &[&[(&str, &str)]] = if detail == Detail::Exact {
        &[
            &[("d", "less"), ("g", "gates"), ("x", "text")],
            &[("d", "less"), ("g", ""), ("x", "")],
        ]
    } else {
        &[
            &[
                ("d", "receipts and covered items"),
                ("g", "gates"),
                ("x", "text"),
            ],
            &[("d", "more"), ("g", "gates"), ("x", "text")],
            &[("d", "more"), ("g", ""), ("x", "")],
        ]
    };
    let line = |form: &[(&'static str, &'static str)]| -> Line<'static> {
        let mut spans = Vec::new();
        for (at, (key, label)) in form.iter().enumerate() {
            if at > 0 {
                spans.push(Span::raw("   "));
            }
            spans.push(Span::styled(*key, style::style(look, Tone::Key)));
            if !label.is_empty() {
                spans.push(Span::styled(
                    format!(" {label}"),
                    style::style(look, Tone::Dim),
                ));
            }
        }
        Line::from(spans)
    };
    let form = forms
        .iter()
        .find(|form| line(form).width() <= usize::from(width))
        .or(forms.last())
        .copied()
        .unwrap_or_default();
    line(form)
}
