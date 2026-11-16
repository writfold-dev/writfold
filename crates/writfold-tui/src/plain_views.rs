//! The run text views at the plain level: which phrases each view shows,
//! its pane title, and the side stack's short forms. A refusal, a missing
//! answer and the Progress text list keep the exact lines.
use crate::app::{App, View};
use crate::cells::Glyphs;
use crate::graph;
use crate::rail_text::clip;
use crate::say::{Phrase, Piece, Told, align, said};
use crate::say_words::{GUARD, STANDING, short};
use crate::source::field;
use crate::style::Look;
use ratatui::text::{Line, Span};
use serde_json::Value;

/// A row's name as the map or the runs list has it.
fn names(app: &App, row: &Value) -> Option<String> {
    let id = field(row, "id");
    if field(row, "kind") == "run" {
        let run = app.runs.rows.iter().find(|r| field(r, "id") == id)?;
        return Some(field(run, "text").to_owned());
    }
    let nodes = graph::nodes(app.answer(View::Progress)?);
    if field(row, "kind") == "plan" && field(row, "status") == "spliced" {
        return crate::say_step::spliced(row, &nodes).map(str::to_owned);
    }
    let node = nodes.iter().find(|n| n.id == id)?;
    Some(crate::say_step::name(node).to_owned())
}

/// The view's phrases, or `None` where the exact lines stay.
pub fn told(app: &App, view: View, glyphs: Glyphs) -> Option<Told> {
    let answer = app.answer(view).filter(|a| a.error.is_none())?;
    let names = |row: &Value| names(app, row);
    Some(match view {
        View::Progress => return None,
        View::Gates => crate::say_gate::view(answer, glyphs),
        View::Inbox => crate::say_lists::inbox(answer),
        View::Guards => crate::say_lists::guards(answer, glyphs),
        View::Timeline => crate::say_row::timeline(answer, &names),
        View::Stale | View::Brief | View::Resume => crate::say_row::generic(answer, glyphs, &names),
    })
}

/// `gates`, `timeline`, …; guards say whose run they judged.
pub fn title(app: &App, view: View) -> String {
    match view {
        View::Guards if app.answer(view).is_some_and(|a| a.error.is_some()) => "guards".into(),
        View::Guards if app.run_id() != app.current_run() => "guards · not the selected run".into(),
        View::Guards => "guards · the current run".into(),
        View::Stale | View::Brief => match app.project() {
            Some(project) => format!("{} {project}", view.name()),
            None => view.name().into(),
        },
        _ => view.name().into(),
    }
}

/// The side stack's short form: the count line, then one line per row.
pub fn side(app: &App, view: View, glyphs: Glyphs) -> Option<Told> {
    let answer = app.answer(view).filter(|a| a.error.is_none())?;
    let mut told = Told::default();
    let rows = answer.rows.iter();
    match view {
        View::Gates => {
            told.lines.push(crate::say_gate::counts(answer));
            for row in rows.filter(|r| field(r, "kind") == "gate") {
                let status = field(row, "status");
                let pieces = vec![
                    Piece::Mark(crate::cells::chip(status, glyphs)),
                    Piece::Quoted(field(row, "id").to_owned()),
                    short(STANDING, status),
                ];
                told.lines
                    .push(Phrase::led(0, crate::lines::tone(row), pieces, 2));
            }
        }
        View::Inbox => {
            told.lines.push(crate::say_lists::inbox_counts(answer));
            let inbox = rows.filter(|r| matches!(field(r, "kind"), "draft" | "proposal"));
            for row in inbox {
                let text = crate::say::quote(field(row, "text"))
                    .unwrap_or_else(|| said("its text is in exact wording (d)"));
                let pieces = vec![said(field(row, "kind")), text];
                told.lines
                    .push(Phrase::led(0, crate::lines::tone(row), pieces, 1));
            }
        }
        View::Guards => {
            told.lines.push(crate::say_lists::guard_counts(answer));
            for row in rows {
                let pieces = vec![
                    Piece::Quoted(field(row, "id").to_owned()),
                    short(GUARD, field(row, "status")),
                ];
                told.lines
                    .push(Phrase::led(0, crate::lines::tone(row), pieces, 1));
            }
        }
        _ => return told_timeline(app, glyphs),
    }
    align(&mut told.lines);
    Some(told)
}

fn told_timeline(app: &App, glyphs: Glyphs) -> Option<Told> {
    let mut told = told(app, View::Timeline, glyphs)?;
    // The side pane keeps rows only; the caption is the full view's.
    told.lines.retain(|p| p.lead > 0);
    Some(told)
}

/// Each phrase on one row, cut at the pane edge (the side stack does not
/// wrap).
pub fn rows(told: &Told, width: usize, look: Look, glyphs: Glyphs) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for p in told.lines.iter().chain(told.closing().iter()) {
        let line = crate::say_draw::phrase(p, usize::MAX / 2, look)
            .into_iter()
            .next()
            .unwrap_or_default();
        out.push(cut(line, width, glyphs));
    }
    out
}

fn cut(line: Line<'static>, width: usize, glyphs: Glyphs) -> Line<'static> {
    let mut used = 0;
    let mut spans = Vec::new();
    for span in line.spans {
        let size = crate::rail_text::width(&span.content);
        if used + size <= width {
            used += size;
            spans.push(span);
            continue;
        }
        let text = clip(&span.content, width.saturating_sub(used), glyphs);
        spans.push(Span::styled(text, span.style));
        break;
    }
    Line::from(spans)
}
