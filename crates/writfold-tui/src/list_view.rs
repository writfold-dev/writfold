//! The narrow full-frame run list supports scrolling without changing selection.
use crate::app::{App, Focus};
use crate::lines::{self, Line, Tone, line};
use crate::{paint, style};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

fn rows(app: &App) -> (Vec<Line>, Option<(usize, usize)>) {
    if app.runs.error.is_some() {
        return (lines::reply(&app.runs), None);
    }
    let mut out = Vec::new();
    let mut selected = None;
    for (n, i) in app.order.iter().enumerate() {
        let start = out.len();
        for (at, mut item) in lines::run_item(&app.runs.rows[*i]).into_iter().enumerate() {
            let mark = if n == app.selected && at == 0 {
                "> "
            } else {
                "  "
            };
            item.text.insert_str(0, mark);
            out.push(item);
        }
        if n == app.selected {
            selected = Some((start, out.len()));
        }
    }
    if out.is_empty() {
        out.push(line("(no runs)", Tone::Dim));
    }
    (out, selected)
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = paint::block(app.runs_title(), app.focus == Focus::Runs, app.look);
    let inner = border.inner(area);
    let (rows, selected) = rows(app);
    let height = usize::from(inner.height);
    let mut scroll = usize::from(app.scroll);
    if app.anchor
        && let Some((start, end)) = selected
    {
        if start < scroll {
            scroll = start;
        }
        if end > scroll + height {
            scroll = end.saturating_sub(height);
        }
    }
    scroll = scroll.min(rows.len().saturating_sub(height));
    app.scroll = u16::try_from(scroll).unwrap_or(u16::MAX);
    app.anchor = false;
    let pane = Paragraph::new(paint::text(rows, app.look))
        .block(border)
        .scroll((app.scroll, 0));
    frame.render_widget(pane, area);
    if let Some((start, end)) = selected {
        let start = start.max(scroll);
        let end = end.min(scroll + height);
        if end > start {
            let y = inner.y + u16::try_from(start - scroll).unwrap_or(0);
            let height = u16::try_from(end - start).unwrap_or(inner.height);
            frame.buffer_mut().set_style(
                Rect::new(inner.x, y, inner.width, height),
                style::selected(app.look),
            );
        }
    }
}
