//! Responsive terminal panes; all content is projected by `frame` and `side`.
use crate::app::{App, Focus, View};
use crate::layout::{self, Areas, Class};
use crate::lines::{self, Tone};
use crate::paint::{block, initial_scroll, text};
use crate::{chrome, frame as content};
use crate::{side, style};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};

/// Load visible answers and draw the whole frame.
pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let size = frame.area();
    app.resize(size.width, size.height);
    frame.render_widget(Block::default().style(style::base(app.look)), size);
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
    chrome::bars(frame, app, &areas, class == Class::Narrow);
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
        chrome::help(frame, app.look);
    }
}

fn detail(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = ratatui::widgets::Block::bordered();
    let inner = border.inner(area);
    let detail = content::detail(app, app.view, Some(inner.width), app.expand_all, false);
    if app.anchor {
        app.scroll = initial_scroll(&detail.lines, detail.current, inner.width, inner.height);
        app.anchor = false;
    }
    let pane = Paragraph::new(text(detail.lines, app.look))
        .block(block(detail.title, app.focus == Focus::Detail, app.look))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    frame.render_widget(pane, area);
}

fn runs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let border = block(app.runs_title(), app.focus == Focus::Runs, app.look);
    if let Some(error) = &app.runs.error {
        let message = Paragraph::new(Line::styled(
            error.as_str(),
            style::style(app.look, Tone::Bad),
        ))
        .block(border);
        frame.render_widget(message, area);
        return;
    }
    let items: Vec<ListItem<'static>> = app
        .order
        .iter()
        .filter_map(|index| app.runs.rows.get(*index))
        .map(|row| ListItem::new(text(lines::run_item(row), app.look)))
        .collect();
    let list = List::new(items)
        .block(border)
        .highlight_style(style::selected(app.look))
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
    let border = block(view.name().into(), false, app.look);
    let inner = border.inner(area);
    let pane = Paragraph::new(text(side::compact(app, view), app.look));
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
