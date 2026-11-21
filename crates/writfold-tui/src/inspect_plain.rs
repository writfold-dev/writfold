//! The plain inspector, in reading order: what the step is, where it
//! sits, what it needs, each gate with its standing, what to read and
//! what is suggested next. Receipts, covered items and the verbs' exact
//! sentences are one `d` away; the commands one `c` away.
use super::InspectLine;
use crate::app::Detail;
use crate::cells::Glyphs;
use crate::graph::Node;
use crate::lines::{Tone, more};
use crate::say::{Phrase, Piece, Told, align, quote, said};
use crate::say_step::{chip, name, needs, place};
use crate::source::field;
use crate::style::{self, Look};
use ratatui::text::{Line, Span};

/// Where the step sits and what the plan trailer says it owes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Context {
    pub index: usize,
    pub total: usize,
    pub owed: Option<u64>,
}

/// The inspector's phrases for one step.
pub fn told(node: &Node<'_>, cx: Context, glyphs: Glyphs) -> Told {
    let mut pieces = vec![Piece::Quoted(name(node).to_owned())];
    pieces.extend(chip(node).map(said));
    pieces.extend(crate::say_step::round(node).map(said));
    let mut told = Told::default();
    told.lines.push(Phrase::led(0, Tone::Current, pieces, 1));
    if cx.total > 0 {
        told.lines.push(place(node, cx.index, cx.total));
    }
    if let Some(began) = crate::say_step::began(node) {
        told.lines
            .push(Phrase::new(0, Tone::Dim, vec![said(began)]));
    }
    told.lines.push(Phrase::blank());
    let current = field(node.row, "status") == "current";
    told.lines
        .extend(needs(node, if current { cx.owed } else { None }));
    if field(node.row, "status") == "skipped" {
        told.withheld += 1;
    }
    let gates = crate::say_gate::step_gates(node.row, glyphs);
    told.lines.extend(gates.lines);
    told.withheld += gates.withheld;
    let mut after = Vec::new();
    for value in more(node.row) {
        let text = field(value, "text");
        let lead = match field(value, "label") {
            "read" => "Read first",
            "next" => "Suggested",
            _ => continue,
        };
        let piece = if lead == "Read first" {
            quote(text)
        } else {
            Some(crate::say_next::suggested(text))
        };
        match piece {
            Some(piece) => after.push(Phrase::led(0, Tone::Info, vec![said(lead), piece], 1)),
            None => told.withheld += 1,
        }
    }
    if !after.is_empty() {
        align(&mut after);
        told.lines.push(Phrase::blank());
        told.lines.extend(after);
    }
    told
}

/// The phrases as unwrapped lines, with the hint last (as `content` does
/// at the exact level).
pub fn content(node: &Node<'_>, cx: Context, glyphs: Glyphs, look: Look) -> Vec<InspectLine> {
    let told = told(node, cx, glyphs);
    let mut out: Vec<InspectLine> = told
        .lines
        .iter()
        .chain(told.closing().iter())
        .map(|p| InspectLine {
            indent: 0,
            text: Line::styled(p.text(), style::style(look, p.tone)),
        })
        .collect();
    out.push(InspectLine {
        indent: 0,
        text: hint(Detail::Plain, u16::MAX, look),
    });
    out
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
            &[("d", "exact wording"), ("c", "commands"), ("g", "gates")],
            &[("d", "exact"), ("c", "commands"), ("g", "gates")],
            &[("d", ""), ("c", ""), ("g", "")],
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
