//! The selected node's styled, hanging-indent paragraphs.
use crate::app::{App, Detail, Focus, View};
use crate::{graph, inspect, map_view, paint};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};

pub(crate) fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(answer) = app.answer(View::Progress) else {
        return;
    };
    let nodes = graph::nodes(answer);
    let Some(index) = nodes.iter().position(|n| Some(n.id) == app.node.as_deref()) else {
        return;
    };
    let title = format!("step {}/{}", index + 1, nodes.len());
    let border = paint::block(title, app.focus == Focus::Inspector, app.look);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let mut lines = inspect::content(&nodes[index], map_view::glyphs(app), app.look, app.level);
    lines.pop();
    if app.level == Detail::Exact {
        lines.extend(recorded(app));
    }
    let hint = inspect::hint(app.level, inner.width, app.look);
    let body = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let panes = paragraphs(lines, inner.width);
    let total: usize = panes.iter().map(|(_, count, _)| count).sum();
    let scroll =
        usize::from(app.inspect_scroll).min(total.saturating_sub(usize::from(body.height)));
    render(frame, panes, body, scroll);
    frame.render_widget(
        Paragraph::new(hint),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
    app.inspect_scroll = scroll.min(usize::from(u16::MAX)) as u16;
}

fn recorded(app: &App) -> impl Iterator<Item = inspect::InspectLine> + '_ {
    map_view::records(app)
        .into_iter()
        .map(move |line| inspect::InspectLine {
            indent: 0,
            text: ratatui::text::Line::styled(line.text, crate::style::style(app.look, line.tone)),
        })
}

fn paragraphs(
    lines: Vec<inspect::InspectLine>,
    width: u16,
) -> Vec<(u16, usize, Paragraph<'static>)> {
    lines
        .into_iter()
        .map(|line| {
            let indent = line.indent.min(width.saturating_sub(1));
            let pane = Paragraph::new(line.text).wrap(Wrap { trim: false });
            let count = pane.line_count(width - indent);
            (indent, count, pane)
        })
        .collect()
}

fn render(
    frame: &mut Frame<'_>,
    panes: Vec<(u16, usize, Paragraph<'_>)>,
    area: Rect,
    mut skip: usize,
) {
    let mut y = area.y;
    for (indent, count, pane) in panes {
        if skip >= count {
            skip -= count;
            continue;
        }
        if y >= area.bottom() {
            break;
        }
        let visible = (count - skip).min(usize::from(area.bottom() - y)) as u16;
        let pane = pane.scroll((skip as u16, 0));
        frame.render_widget(
            pane,
            Rect::new(area.x + indent, y, area.width - indent, visible),
        );
        y += visible;
        skip = 0;
    }
}
