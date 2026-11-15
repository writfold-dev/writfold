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
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

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
    if app.mode == crate::kb_state::Mode::Kb {
        crate::kb_view::draw(frame, app);
        if app.help {
            chrome::help(frame, app);
        }
        return;
    }
    let inspect =
        app.focus != Focus::Runs && (class != Class::Narrow || app.focus == Focus::Inspector);
    let areas = layout::focused(size.width, size.height, app.map_visible(), inspect);
    chrome::bars(frame, app, &areas, class == Class::Narrow);
    if let Some(area) = areas.runs {
        runs(frame, app, area);
    }
    if let Some(area) = areas.inspector {
        crate::inspect_view::draw(frame, app, area);
    }
    if class == Class::Narrow && app.runs_view {
        crate::list_view::draw(frame, app, areas.main);
    } else if class != Class::Narrow || areas.inspector.is_none() {
        if app.map_visible() {
            crate::map_view::draw(frame, app, areas.main);
        } else {
            detail(frame, app, areas.main);
        }
    }
    sides(frame, app, &areas);
    if app.help {
        chrome::help(frame, app);
    }
}

fn detail(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = ratatui::widgets::Block::bordered();
    let inner = border.inner(area);
    let mut detail = content::detail(app, app.view, Some(inner.width), app.expand_all, false);
    // The inbox tab badge is one number; the view leads with its breakdown.
    if app.view == View::Inbox
        && let Some(answer) = app.answer(View::Inbox).filter(|a| a.error.is_none())
    {
        let counts = side::inbox_counts(answer);
        detail.lines.insert(0, lines::line(counts, Tone::Dim));
    }
    if app.anchor {
        app.scroll = initial_scroll(&detail.lines, detail.current, inner.width, inner.height);
        app.anchor = false;
    }
    if app.scroll > 0 {
        let last = detail
            .lines
            .iter()
            .map(|l| crate::wrap::height(l, inner.width))
            .sum::<usize>()
            .saturating_sub(1);
        app.scroll = app.scroll.min(u16::try_from(last).unwrap_or(u16::MAX));
    }
    frame.render_widget(
        block(detail.title, app.focus == Focus::Detail, app.look),
        area,
    );
    crate::wrap::draw(frame, &detail.lines, inner, app.look, app.scroll);
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
    // Entries clip with a marker inside the pane, after the "> " gutter.
    let room = usize::from(area.width.saturating_sub(4));
    let glyphs = crate::map_view::glyphs(app);
    let items: Vec<ListItem<'static>> = app
        .order
        .iter()
        .filter_map(|index| app.runs.rows.get(*index))
        .map(|row| {
            let mut entry = lines::run_item(row, app.level);
            for line in &mut entry {
                line.text = crate::rail_text::clip(&line.text, room, glyphs);
            }
            ListItem::new(text(entry, app.look))
        })
        .collect();
    let list = List::new(items)
        .block(border)
        .highlight_style(style::selected(app.look))
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn sides(frame: &mut Frame<'_>, app: &App, areas: &Areas) {
    for (view, area) in [
        (View::Gates, areas.gates),
        (View::Inbox, areas.inbox),
        (View::Guards, areas.guards),
        (View::Timeline, areas.timeline),
    ] {
        if let Some(area) = area {
            side(frame, app, view, area);
        }
    }
}

fn side(frame: &mut Frame<'_>, app: &App, view: View, area: Rect) {
    let title = if view == View::Guards {
        "guards (current run)"
    } else {
        view.name()
    };
    let border = block(title.into(), false, app.look);
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
