//! The record inspector: `status ID`, `why ID` for a decision and
//! `timeline ID`, or `uses ID` for a node outside the KB, quoted in calm
//! sections; `d` adds every position, fresh cite, mention and field.
use crate::app::{App, Detail, Focus};
use crate::kb_entry::Entry;
use crate::kb_ink::{Ink, header, refused};
use crate::kb_sections::{decision, fields, notes, runs, section, trailer};
use crate::kb_state::Inspected;
use crate::lines::Tone;
use crate::rail_text::left;
use crate::source::field;
use crate::style;
use crate::{map_view, paint};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use serde_json::Value;

/// The inspector's lines for one entry, from its cached answers.
pub(crate) fn content(
    app: &App,
    entry: &Entry,
    record: &Inspected,
    ink: &Ink,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    header(&mut out, entry, ink);
    let status = &record.status;
    refused(&mut out, status, ink);
    if entry.kind == "cite" {
        out.push(Line::default());
        ink.wrap(&mut out, "", &entry.text, Tone::Plain, 2);
        if !entry.flags.is_empty() {
            ink.wrap(&mut out, "", &entry.flags.join(" "), Tone::Bad, 2);
        }
    }
    if let Some(why) = &record.why {
        decision(&mut out, &entry.id, why, ink);
    } else if let Some(row) = status.rows.first().filter(|r| field(r, "id") == entry.id) {
        fields(&mut out, row, ink);
    }
    if let Some(why) = &record.why {
        section(&mut out, why, "secondary", "see also", ink);
    }
    let cited = record.timeline.is_none();
    section(&mut out, status, "tracked-by", "linked from", ink);
    if cited {
        let mut uses = status.clone();
        for row in &mut uses.rows {
            row["section"] = Value::from("cited-by");
        }
        section(&mut out, &uses, "cited-by", "cited by", ink);
    }
    if ink.full {
        section(&mut out, status, "mentions", "mentions", ink);
    }
    if let Some(timeline) = record.timeline.as_ref().filter(|_| entry.kind != "run") {
        runs(&mut out, app, timeline, ink);
    }
    notes(&mut out, record, ink);
    if ink.full {
        out.push(Line::default());
        trailer(&mut out, if cited { "uses" } else { "status" }, status, ink);
        if let Some(why) = &record.why {
            trailer(&mut out, "why", why, ink);
        }
        if let Some(timeline) = &record.timeline {
            trailer(&mut out, "timeline", timeline, ink);
        }
    }
    out
}

/// The command hint, shortened to fit.
fn hint(app: &App, width: usize) -> Line<'static> {
    let detail = if app.level == Detail::Exact {
        "less"
    } else {
        "sources and fresh cites"
    };
    let forms: [&[(&str, &str)]; 3] = [
        &[("d", detail), ("f", "run"), ("a", "impact")],
        &[
            (
                "d",
                if app.level == Detail::Exact {
                    "less"
                } else {
                    "more"
                },
            ),
            ("f", "run"),
            ("a", "impact"),
        ],
        &[("d", ""), ("f", ""), ("a", "")],
    ];
    let look = app.look;
    let line = |form: &[(&str, &str)]| {
        let mut spans = Vec::new();
        for (at, (key, label)) in form.iter().enumerate() {
            if at > 0 {
                spans.push(Span::raw("   "));
            }
            spans.push(Span::styled(
                (*key).to_owned(),
                style::style(look, Tone::Key),
            ));
            if !label.is_empty() {
                spans.push(Span::styled(
                    format!(" {label}"),
                    style::style(look, Tone::Dim),
                ));
            }
        }
        Line::from(spans)
    };
    forms
        .iter()
        .map(|form| line(form))
        .find(|l| l.width() <= width)
        .unwrap_or_else(|| line(forms[2]))
}

pub(crate) fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let entry = app.kb_entry().or_else(|| project_entry(app));
    let entries = app.kb_entries();
    let selectable: Vec<_> = entries.iter().filter(|e| !e.header).collect();
    let kind = |e: &crate::kb_entry::Entry| {
        if e.kind.is_empty() {
            "record".to_owned()
        } else {
            e.kind.clone()
        }
    };
    let title = match &entry {
        Some(e) => match selectable.iter().position(|s| s.key == e.key) {
            Some(at) => format!("{} {}/{}", kind(e), at + 1, selectable.len()),
            None => kind(e),
        },
        None => "record".into(),
    };
    let border = paint::block(title, app.kb.focus == Focus::Inspector, app.look);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let (Some(entry), true) = (entry, inner.height > 1) else {
        return;
    };
    let Some(record) = app.kb.records.get(&entry.id) else {
        return;
    };
    let ink = Ink {
        look: app.look,
        glyphs: map_view::glyphs(app),
        width: usize::from(inner.width),
        full: app.level == Detail::Exact,
    };
    let mut lines = content(app, &entry, record, &ink);
    let source = source_line(record, &entry, &ink);
    lines.extend(source);
    let body = usize::from(inner.height - 1);
    let scroll = usize::from(app.kb.inspect_scroll).min(lines.len().saturating_sub(body));
    for (y, line) in lines.into_iter().skip(scroll).take(body).enumerate() {
        let row = Rect::new(inner.x, inner.y + y as u16, inner.width, 1);
        frame.render_widget(Paragraph::new(line), row);
    }
    let bottom = Rect::new(inner.x, inner.bottom() - 1, inner.width, 1);
    frame.render_widget(Paragraph::new(hint(app, usize::from(inner.width))), bottom);
    app.kb.inspect_scroll = u16::try_from(scroll).unwrap_or(u16::MAX);
}

/// The record's own source, `@path:line`, cut from the left to one line.
fn source_line(record: &Inspected, entry: &Entry, ink: &Ink) -> Option<Line<'static>> {
    let row = record
        .status
        .rows
        .first()
        .filter(|r| field(r, "id") == entry.id);
    let (path, line) = match row {
        Some(row) => (
            field(row, "path").to_owned(),
            row.get("line").and_then(Value::as_u64),
        ),
        None => (entry.path.clone(), entry.line),
    };
    if path.is_empty() {
        return None;
    }
    let text = format!(
        "@{path}{}",
        line.map(|n| format!(":{n}")).unwrap_or_default()
    );
    Some(Line::styled(
        left(&text, ink.width, ink.glyphs),
        ink.style(Tone::Dim),
    ))
}

/// The status view inspects the project record itself.
fn project_entry(app: &App) -> Option<Entry> {
    let id = app.kb_scope().id()?.to_owned();
    let row = app
        .kb_projects()
        .into_iter()
        .find(|r| field(r, "id") == id)?;
    Some(crate::kb_entry::from_row(row, field(row, "text")))
}
