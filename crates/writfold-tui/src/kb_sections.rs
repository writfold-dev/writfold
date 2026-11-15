//! The inspector's sections, each quoting one verb answer: the decision
//! (`why`), fields and links (`status`), runs (`timeline`), trailers and
//! notes.
use crate::app::App;
use crate::cells::Glyphs;
use crate::kb_entry::flag_marks;
use crate::kb_ink::{Ink, more, refused};
use crate::kb_state::Inspected;
use crate::lines::Tone;
use crate::rail_text::{clip, left};
use crate::source::{Reply, field, flags};
use ratatui::text::{Line, Span};
use serde_json::Value;

/// `decided`, `why`, then the cite strip and the cites that carry a flag.
pub(crate) fn decision(out: &mut Vec<Line<'static>>, id: &str, why: &Reply, ink: &Ink) {
    refused(out, why, ink);
    let Some(row) = why
        .rows
        .iter()
        .find(|r| r.get("section").is_none_or(Value::is_null))
    else {
        return;
    };
    if field(row, "id") != id {
        // `why` on a superseded decision answers with its successor.
        out.push(Line::default());
        ink.wrap(out, "", "why answers with its successor:", Tone::Warn, 2);
        ink.wrap(out, "  ", field(row, "text"), Tone::Plain, 2);
        out.push(Line::styled(
            left(&format!("  {}", field(row, "id")), ink.width, ink.glyphs),
            ink.style(Tone::Dim),
        ));
    }
    for label in ["decided", "why"] {
        for value in more(row, label) {
            if label == "why" || out.last().is_some_and(|l| l.width() > 0) {
                out.push(Line::default());
            }
            let first = format!("{label}{}", if label == "why" { "  " } else { " " });
            let indent = if label == "why" { 5 } else { 2 };
            ink.wrap(out, &first, field(value, "text"), Tone::Plain, indent);
            ink.position(out, value, indent);
        }
    }
    let cites: Vec<&Value> = more(row, "cite").collect();
    if cites.is_empty() {
        return;
    }
    out.push(Line::default());
    let mut strip = vec![Span::raw(format!("cites {}  ", cites.len()))];
    for (at, cite) in cites.iter().enumerate() {
        let marks = flag_marks(flags(cite));
        let (mark, tone) = if marks.moved {
            ("!", Tone::Bad)
        } else if marks.unknown {
            ("?", Tone::Warn)
        } else if ink.glyphs == Glyphs::Unicode {
            ("●", Tone::Good)
        } else {
            ("+", Tone::Good)
        };
        if at > 0 {
            strip.push(Span::raw(" "));
        }
        strip.push(Span::styled(mark, ink.style(tone)));
    }
    out.push(Line::from(strip));
    for cite in cites {
        let marks = flag_marks(flags(cite));
        if !ink.full && !marks.moved && !marks.unknown {
            continue;
        }
        let mark = if marks.moved {
            "!"
        } else if marks.unknown {
            "?"
        } else {
            " "
        };
        let tone = if marks.moved { Tone::Bad } else { Tone::Plain };
        ink.wrap(out, &format!("  {mark} "), field(cite, "text"), tone, 4);
        if !flags(cite).is_empty() {
            ink.wrap(out, "    ", &flags(cite).join(" "), Tone::Dim, 4);
        }
        ink.position(out, cite, 4);
    }
}

/// A record's `field` lines (every one: hiding empty values would mean
/// reading their text), each at its position at `d`.
pub(crate) fn fields(out: &mut Vec<Line<'static>>, row: &Value, ink: &Ink) {
    let mut any = false;
    for value in more(row, "field") {
        if !any {
            out.push(Line::default());
            any = true;
        }
        ink.wrap(out, "", field(value, "text"), Tone::Plain, 2);
        ink.position(out, value, 2);
    }
}

/// Rows of one section as glyph rows: title (or id) and the first `via`.
pub(crate) fn section(
    out: &mut Vec<Line<'static>>,
    reply: &Reply,
    name: &str,
    heading: &str,
    ink: &Ink,
) {
    let rows: Vec<&Value> = reply
        .rows
        .iter()
        .filter(|r| r.get("section").and_then(Value::as_str) == Some(name))
        .collect();
    if rows.is_empty() {
        return;
    }
    ink.heading(out, heading);
    for row in rows {
        let text = field(row, "text");
        let label = if text.is_empty() {
            field(row, "id")
        } else {
            text
        };
        let via = more(row, "via")
            .next()
            .map(|v| field(v, "text"))
            .unwrap_or_default();
        let word = if via.is_empty() {
            field(row, "status")
        } else {
            via
        };
        ink.glyph_row(out, field(row, "status"), label, word);
        if ink.full {
            for value in more(row, "outcome") {
                ink.wrap(out, "    ", field(value, "text"), Tone::Dim, 4);
            }
            ink.position(out, row, 4);
        }
    }
}

/// One line per distinct run among `timeline`'s `checkpoint` rows, by the
/// title the runs list has for it; `d` adds each checkpoint's text.
pub(crate) fn runs(out: &mut Vec<Line<'static>>, app: &App, timeline: &Reply, ink: &Ink) {
    let rows: Vec<&Value> = timeline
        .rows
        .iter()
        .filter(|r| field(r, "kind") == "checkpoint")
        .collect();
    let mut seen: Vec<&str> = Vec::new();
    for row in &rows {
        let id = field(row, "id");
        if seen.contains(&id) {
            continue;
        }
        if seen.is_empty() {
            ink.heading(out, "runs");
        }
        seen.push(id);
        let title = app
            .runs
            .rows
            .iter()
            .find(|r| field(r, "id") == id)
            .map_or(id, |r| field(r, "text"));
        out.push(Line::raw(format!(
            "  {}",
            clip(title, ink.width.saturating_sub(2), ink.glyphs)
        )));
        if ink.full {
            for row in rows.iter().filter(|r| field(r, "id") == id) {
                ink.wrap(out, "    ", field(row, "text"), Tone::Dim, 4);
            }
        }
    }
}

pub(crate) fn trailer(out: &mut Vec<Line<'static>>, verb: &str, reply: &Reply, ink: &Ink) {
    let text = |key: &str| {
        reply
            .trailer
            .get(key)
            .map(|v| v.to_string().trim_matches('"').to_owned())
    };
    if let (Some(coverage), Some(seen), Some(total)) = (
        text("coverage"),
        text("records_examined"),
        text("records_total"),
    ) {
        let line = format!("{verb}: coverage {coverage}, records {seen}/{total}");
        out.push(Line::styled(
            clip(&line, ink.width, ink.glyphs),
            ink.style(Tone::Dim),
        ));
    }
}

/// The verbs' notes and an unknown answer's reason, dim and once each.
pub(crate) fn notes(out: &mut Vec<Line<'static>>, record: &Inspected, ink: &Ink) {
    let mut seen: Vec<&str> = Vec::new();
    let replies = [Some(&record.status), record.why.as_ref()];
    let mut first = true;
    for reply in replies.into_iter().flatten() {
        if let Some(reason) = reply.trailer_text("reason") {
            if first {
                out.push(Line::default());
                first = false;
            }
            ink.wrap(out, "", &format!("answer unknown: {reason}"), Tone::Warn, 2);
        }
        for note in &reply.notes {
            if seen.contains(&note.as_str()) {
                continue;
            }
            if first {
                out.push(Line::default());
                first = false;
            }
            seen.push(note);
            ink.wrap(out, "", note, Tone::Dim, 2);
        }
    }
}
