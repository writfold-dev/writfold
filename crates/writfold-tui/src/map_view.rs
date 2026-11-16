//! Draw only the visible rail window, followed by recorded verb facts.
use crate::app::{App, Detail, Focus, View};
use crate::cells::Glyphs;
use crate::lines::{self, Tone};
use crate::rail_text::clip;
use crate::source::field;
use crate::{graph, paint, progress, rail, style};
use ratatui::text::Line as Row;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, layout::Rect, style::Style};

pub(crate) fn glyphs(app: &App) -> Glyphs {
    if app.dumb || app.look.depth == style::Depth::None {
        Glyphs::Ascii
    } else {
        Glyphs::Unicode
    }
}

/// The run's newest plan event and checkpoint, quoted from their verbs.
pub(crate) fn records(app: &App) -> Vec<lines::Line> {
    let event = app
        .answer(View::Timeline)
        .and_then(|a| a.rows.iter().rev().find(|r| field(r, "kind") == "plan"));
    let checkpoint = app
        .row()
        .and_then(|r| lines::more(r).find(|m| field(m, "label") == "checkpoint"));
    [("last plan event", event), ("last checkpoint", checkpoint)]
        .into_iter()
        .map(|(label, row)| {
            let text = row.map_or("(none recorded)", |r| field(r, "text"));
            lines::line(format!("{label}: {text}"), Tone::Dim)
        })
        .collect()
}

/// The one-row caption for the marker; the full legend lives in help.
pub(crate) fn caption(glyphs: Glyphs) -> &'static str {
    if glyphs == Glyphs::Unicode {
        "▶ next step to pick up, as recorded"
    } else {
        "[>] next step to pick up, as recorded"
    }
}

/// Calm: the last plan event on one clipped row. Detailed: the event and
/// checkpoint, wrapped. Both end with the marker caption.
fn below(app: &App, width: u16, glyphs: Glyphs) -> Vec<Row<'static>> {
    let dim = style::style(app.look, Tone::Dim);
    let mut records = records(app);
    let mut out: Vec<Row<'static>> = if app.level == Detail::Exact {
        records
            .drain(..)
            .map(|l| Row::styled(l.text, dim))
            .collect()
    } else {
        let event = records.swap_remove(0).text.replacen(": ", "  ", 1);
        vec![Row::styled(clip(&event, usize::from(width), glyphs), dim)]
    };
    out.push(Row::styled(caption(glyphs), dim));
    out
}

fn rows_of(lines: &[Row<'static>], width: u16) -> u16 {
    let count = Paragraph::new(lines.to_vec())
        .wrap(Wrap { trim: false })
        .line_count(width);
    u16::try_from(count).unwrap_or(u16::MAX)
}

fn put(frame: &mut Frame<'_>, lines: Vec<Row<'static>>, area: Rect) {
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

pub(crate) fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = paint::block("plan".into(), app.focus == Focus::Detail, app.look);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let Some(answer) = app.answer(View::Progress) else {
        return;
    };
    let nodes = graph::nodes(answer);
    let selected = nodes
        .iter()
        .position(|n| Some(n.id) == app.node.as_deref())
        .unwrap_or(0);
    let glyphs = glyphs(app);
    let show = rail::Show {
        glyphs,
        look: app.look,
        full: app.level == Detail::Exact,
        selected: Some(selected),
    };
    let rail = rail::build(&nodes, inner.width, show);
    let top = prelude(answer, app, inner.width);
    let mut bottom = below(app, inner.width, glyphs);
    let (top_height, spacer, bottom_height) = heights(&top, &mut bottom, inner, rail.lines.len());
    put(
        frame,
        top,
        Rect::new(inner.x, inner.y, inner.width, top_height),
    );
    let top = top_height + spacer;
    let height = inner.height.saturating_sub(top + bottom_height);
    let window = rail::window(&rail, selected, usize::from(height), app.rail_offset);
    let area = Rect::new(inner.x, inner.y + top, inner.width, height);
    draw_rail(frame, app, rail, window.clone(), area, selected);
    let at = Rect::new(inner.x, inner.y + top + height, inner.width, bottom_height);
    put(frame, bottom, at);
    app.rail_offset = window.start;
}

/// Rows for the summary, a spacer (only when the whole rail fits) and the
/// block below. The rail keeps at least four rows; the caption goes first,
/// then the oldest record line, when space is short.
fn heights(
    top: &[Row<'static>],
    bottom: &mut Vec<Row<'static>>,
    inner: Rect,
    rail: usize,
) -> (u16, u16, u16) {
    let budget = inner.height.saturating_sub(4).min(inner.height / 2);
    let top_height = rows_of(top, inner.width).min(budget);
    while !bottom.is_empty() && top_height + rows_of(bottom, inner.width) > budget {
        bottom.pop();
    }
    let bottom_height = rows_of(bottom, inner.width);
    let fits = rail + usize::from(top_height + bottom_height) < usize::from(inner.height);
    (top_height, u16::from(fits), bottom_height)
}

fn draw_rail(
    frame: &mut Frame<'_>,
    app: &App,
    rail: rail::Rail<'_>,
    window: std::ops::Range<usize>,
    area: Rect,
    selected: usize,
) {
    let selected_line = rail.node_lines.get(selected).copied();
    for (y, line) in rail
        .lines
        .into_iter()
        .skip(window.start)
        .take(usize::from(area.height))
        .enumerate()
    {
        // The selected row takes the theme's selection surface across the
        // pane width; glyph colours stay legible on it (reverse video at
        // degraded depths).
        let row = if selected_line == Some(window.start + y) {
            style::selected(app.look)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(line).style(row),
            Rect::new(area.x, area.y + y as u16, area.width, 1),
        );
    }
}

fn prelude(answer: &crate::source::Reply, app: &App, width: u16) -> Vec<Row<'static>> {
    let mut lines = progress::summary(answer, app.level == Detail::Exact, app.look, width);
    if answer.count("current").is_none() {
        lines.extend(
            answer
                .notes
                .first()
                .map(|n| Row::styled(format!("# note: {n}"), style::style(app.look, Tone::Warn))),
        );
    }
    lines
}
