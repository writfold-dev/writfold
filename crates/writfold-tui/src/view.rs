//! Responsive terminal panes; all content is projected by `frame` and `side`.
use crate::app::{App, Focus, View};
use crate::frame as content;
use crate::layout::{self, Areas, Class};
use crate::lines::{self, Tone};
use crate::paint::{block, initial_scroll, style, text};
use crate::side;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph, Wrap};

/// Load visible answers and draw the whole frame.
pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let size = frame.area();
    app.resize(size.width, size.height);
    let class = layout::class(size.width, size.height);
    if class == Class::TooSmall {
        let message = format!(
            "writfold watch needs 60x12 (this terminal is {}x{})",
            size.width, size.height
        );
        frame.render_widget(Paragraph::new(message), size);
        return;
    }
    let areas = layout::areas(size.width, size.height);
    bars(frame, app, &areas, class == Class::Narrow);
    if let Some(area) = areas.runs {
        runs(frame, app, area);
    }
    if class == Class::Narrow && app.runs_view {
        runs(frame, app, areas.main);
    } else {
        detail(frame, app, areas.main);
    }
    sides(frame, app, &areas);
    if app.help {
        help(frame);
    }
}

fn bars(frame: &mut Frame<'_>, app: &App, areas: &Areas, narrow: bool) {
    let header = Line::styled(content::header(app, narrow), style(Tone::Head));
    let tabs = Line::styled(content::tabs(app, narrow), style(Tone::Key));
    frame.render_widget(Paragraph::new(header), areas.header);
    frame.render_widget(Paragraph::new(tabs), areas.tabs);
    if let Some(area) = areas.summary {
        frame.render_widget(Paragraph::new(side::summary(app)), area);
    }
    let footer = if narrow {
        let pin = app.runs.trailer_text("kb");
        let pin = pin.as_deref().map_or("?", |pin| pin);
        let notice = app.notice.as_deref().map_or("", |notice| notice);
        format!("kb={pin} ? q {notice}")
    } else {
        content::footer(app, true)
    };
    let footer = Line::styled(footer, style(Tone::Dim));
    frame.render_widget(Paragraph::new(footer), areas.footer);
}

fn detail(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = ratatui::widgets::Block::bordered();
    let inner = border.inner(area);
    let detail = content::detail(app, app.view, Some(inner.width), app.expand_all, false);
    if app.anchor {
        app.scroll = initial_scroll(&detail.lines, detail.current, inner.width, inner.height);
        app.anchor = false;
    }
    let pane = Paragraph::new(text(detail.lines))
        .block(block(detail.title, app.focus == Focus::Detail))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    frame.render_widget(pane, area);
}

fn runs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let border = block(app.runs_title(), app.focus == Focus::Runs);
    if let Some(error) = &app.runs.error {
        let message = Paragraph::new(Line::styled(error.as_str(), style(Tone::Bad))).block(border);
        frame.render_widget(message, area);
        return;
    }
    let items: Vec<ListItem<'static>> = app
        .order
        .iter()
        .filter_map(|index| app.runs.rows.get(*index))
        .map(|row| ListItem::new(text(lines::run_item(row))))
        .collect();
    let list = List::new(items)
        .block(border)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn sides(frame: &mut Frame<'_>, app: &App, areas: &Areas) {
    let timeline = areas.timeline.map(|mut area| {
        // Until the guards pane is available, timeline uses its space as well.
        if let Some(guards) = areas.guards {
            area.y = guards.y;
            area.height += guards.height;
        }
        area
    });
    for (view, area) in [
        (View::Gates, areas.gates),
        (View::Inbox, areas.inbox),
        (View::Timeline, timeline),
    ] {
        if let Some(area) = area {
            side(frame, app, view, area);
        }
    }
}

fn side(frame: &mut Frame<'_>, app: &App, view: View, area: Rect) {
    let border = block(view.name().into(), false);
    let inner = border.inner(area);
    let pane = Paragraph::new(text(side::compact(app, view)));
    let scroll = if view == View::Timeline {
        let offset = pane
            .line_count(inner.width)
            .saturating_sub(usize::from(inner.height));
        u16::try_from(offset).unwrap_or(u16::MAX)
    } else {
        0
    };
    frame.render_widget(pane.block(border).scroll((scroll, 0)), area);
}

fn help(frame: &mut Frame<'_>) {
    let area = frame.area();
    let width = area.width.min(100);
    let body = Paragraph::new(text(content::help())).wrap(Wrap { trim: false });
    let rows = body.line_count(width.saturating_sub(2)).saturating_add(2);
    let height = area.height.min(u16::try_from(rows).unwrap_or(u16::MAX));
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    frame.render_widget(body.block(block("help".into(), true)), popup);
}
