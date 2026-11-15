//! Knowledge-base mode's frame: projects, the view's list and the
//! inspector, in the run mode's rectangles and with its bars.
use crate::app::{App, Detail, Focus};
use crate::kb_rail::{self, Show};
use crate::kb_state::{KbView, Scope};
use crate::layout::{self, Class};
use crate::lines::{self, Tone};
use crate::paint::block;
use crate::{map_view, style};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &mut App) {
    app.kb_load();
    let size = frame.area();
    let class = layout::class(size.width, size.height);
    let narrow = class == Class::Narrow;
    let focus = app.kb.focus;
    let inspect = focus != Focus::Runs && (!narrow || focus == Focus::Inspector);
    let areas = layout::focused(size.width, size.height, true, inspect);
    crate::kb_bars::draw(frame, app, &areas, narrow);
    if let Some(area) = areas.runs {
        crate::kb_projects::draw(frame, app, area);
    }
    if let Some(area) = areas.inspector {
        crate::kb_inspect::draw(frame, app, area);
    }
    if narrow && app.kb.projects_view {
        crate::kb_projects::draw(frame, app, areas.main);
    } else if !narrow || areas.inspector.is_none() {
        list(frame, app, areas.main);
    }
}

/// The scope's name in sentences: the project's name, or `the KB`.
pub(crate) fn scope_name(app: &App) -> String {
    app.kb_scope().name().unwrap_or("the KB").to_owned()
}

fn title(app: &App, count: usize) -> String {
    match app.kb.view {
        KbView::Status => format!("status {}", scope_name(app)),
        KbView::Impact => "impact".into(),
        view => format!("{} ({count})", view.name()),
    }
}

/// What the list says when it has no entry of its kind.
fn empty(app: &App) -> String {
    let name = scope_name(app);
    match app.kb.view {
        KbView::Why => format!("{name} names no decisions"),
        KbView::Tracker => format!("nothing tracked within 2 links of {name}"),
        KbView::Impact => match app.kb_subject() {
            Some(subject) => format!("nothing reached within 2 links of {subject}"),
            None => "no subject: press a on a record".into(),
        },
        KbView::Stale => {
            let checked = app
                .kb
                .stale
                .as_ref()
                .and_then(|r| r.count("checked"))
                .unwrap_or(0);
            format!("nothing stale; {checked} checked")
        }
        KbView::Status => String::new(),
    }
}

/// Lines above the rail (the impact direction) and below it (a caption or
/// the walk's trailer).
fn around(app: &App) -> (Vec<lines::Line>, Vec<lines::Line>) {
    let name = scope_name(app);
    let subject = app.kb_subject().unwrap_or_default();
    let reply = app.kb_list_reply();
    let walk = |reply: Option<&crate::source::Reply>| {
        reply.map(|r| {
            let (text, partial) = crate::kb_tree::trailer(r);
            lines::line(text, if partial { Tone::Warn } else { Tone::Dim })
        })
    };
    match (app.kb.view, app.kb_scope()) {
        (KbView::Impact, _) => {
            let direction = app.kb.direction.name();
            let caption = match direction {
                "in" => format!("in  what cites or links to {subject}"),
                "out" => format!("out  what {subject} cites or links to"),
                _ => format!("both  every link of {subject}"),
            };
            let top = vec![
                lines::line(caption, Tone::Plain),
                lines::line("", Tone::Dim),
            ];
            (top, walk(reply).into_iter().collect())
        }
        (KbView::Tracker, Scope::Project(_)) => (Vec::new(), walk(reply).into_iter().collect()),
        (KbView::Why, Scope::Project(_)) => (
            Vec::new(),
            vec![lines::line(
                format!("decisions that name {name} as their project"),
                Tone::Dim,
            )],
        ),
        (KbView::Why, Scope::All) => (
            Vec::new(),
            vec![lines::line("every decision in the KB", Tone::Dim)],
        ),
        (KbView::Tracker, Scope::All) => (
            Vec::new(),
            vec![lines::line(
                "every ticket, investigation and signature",
                Tone::Dim,
            )],
        ),
        _ => (Vec::new(), Vec::new()),
    }
}

fn list(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let entries = app.kb_entries();
    let count = entries.iter().filter(|e| !e.header).count();
    let border = block(title(app, count), app.kb.focus == Focus::Detail, app.look);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let reply = app.kb_list_reply();
    let refused = reply.and_then(|r| r.error.as_ref()).is_some();
    if app.kb.view == KbView::Status || refused || count == 0 {
        let mut body = match reply {
            Some(r) if refused || app.kb.view == KbView::Status => lines::reply(r),
            _ => vec![lines::line(empty(app), Tone::Dim)],
        };
        if count == 0 && !refused && app.kb.view != KbView::Status {
            body.extend(reply.into_iter().flat_map(lines::notes));
        }
        let last = body.len().saturating_sub(1);
        app.kb.scroll = app.kb.scroll.min(u16::try_from(last).unwrap_or(u16::MAX));
        crate::wrap::draw(frame, &body, inner, app.look, app.kb.scroll);
        return;
    }
    let (top, bottom) = around(app);
    let glyphs = map_view::glyphs(app);
    let wrap = |rows: Vec<lines::Line>| -> Vec<lines::Line> {
        rows.into_iter()
            .flat_map(|l| {
                kb_rail::wrap_words(&l.text, usize::from(inner.width), glyphs)
                    .into_iter()
                    .map(move |text| lines::line(text, l.tone))
            })
            .collect()
    };
    let (top, bottom) = (wrap(top), wrap(bottom));
    let show = Show {
        glyphs: map_view::glyphs(app),
        look: app.look,
        full: app.level == Detail::Full,
        width: inner.width,
        stale: app.kb.stale.as_ref(),
    };
    let rail = kb_rail::build(&entries, show);
    let selected = app.kb_entry().map(|e| e.key);
    let at = entries
        .iter()
        .position(|e| !e.header && Some(&e.key) == selected.as_ref())
        .unwrap_or(0);
    let start_line = rail.starts.get(at).copied().unwrap_or(0);
    let height = usize::from(inner.height).saturating_sub(top.len() + bottom.len());
    let start = kb_rail::window(rail.lines.len(), start_line, height, app.kb.offset);
    app.kb.offset = start;
    let dim = |l: lines::Line| Line::styled(l.text, style::style(app.look, l.tone));
    let mut y = inner.y;
    for line in top {
        put(frame, dim(line), Rect::new(inner.x, y, inner.width, 1));
        y += 1;
    }
    for (offset, line) in rail.lines.into_iter().skip(start).take(height).enumerate() {
        let row = Rect::new(inner.x, y, inner.width, 1);
        let mut pane = Paragraph::new(line);
        if start + offset == start_line {
            pane = pane.style(style::selected(app.look));
        }
        frame.render_widget(pane, row);
        y += 1;
    }
    let bottom_y = inner
        .bottom()
        .saturating_sub(u16::try_from(bottom.len()).unwrap_or(0));
    for (n, line) in bottom.into_iter().enumerate() {
        let row = Rect::new(inner.x, bottom_y + n as u16, inner.width, 1);
        put(frame, dim(line), row);
    }
}

fn put(frame: &mut Frame<'_>, line: Line<'static>, area: Rect) {
    frame.render_widget(Paragraph::new(line), area);
}
