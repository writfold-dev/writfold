//! The projects pane: each project row of `brief --all` (heading, then
//! phase and name; `d` adds the id and position), then `all records`.
use crate::app::{App, Detail, Focus};
use crate::kb_state::Scope;
use crate::lines::{self, Tone};
use crate::paint::{block, text};
use crate::rail_text::clip;
use crate::source::field;
use crate::{map_view, style};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{List, ListItem, ListState};

/// The project rows of `brief --all`, then `all records`.
pub(crate) fn draw(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = app.kb_projects();
    let title = format!("projects ({})", rows.len());
    let border = block(title, app.kb.focus == Focus::Runs, app.look);
    let room = usize::from(area.width.saturating_sub(4));
    let glyphs = map_view::glyphs(app);
    let full = app.level == Detail::Full;
    let mut items: Vec<ListItem<'static>> = Vec::new();
    for row in &rows {
        let heading = field(row, "text");
        let id = field(row, "id");
        let name = id.strip_prefix("project:").unwrap_or(id);
        let mut entry = vec![
            lines::line(heading, Tone::Plain),
            lines::line(
                format!("{}  {name}", field(row, "status"))
                    .trim()
                    .to_owned(),
                Tone::Dim,
            ),
        ];
        if full {
            entry.push(lines::line(id, Tone::Dim));
        }
        for line in &mut entry {
            line.text = clip(&line.text, room, glyphs);
        }
        if full {
            let at = format!(
                "@{}{}",
                field(row, "path"),
                row.get("line").map(|n| format!(":{n}")).unwrap_or_default()
            );
            entry.push(lines::line(
                crate::rail_text::left(&at, room, glyphs),
                Tone::Dim,
            ));
        }
        items.push(ListItem::new(text(entry, app.look)));
    }
    let all = vec![
        lines::line("all records", Tone::Plain),
        lines::line("by kind", Tone::Dim),
    ];
    items.push(ListItem::new(text(all, app.look)));
    let selected = match app.kb_scope() {
        Scope::All => rows.len(),
        Scope::Project(id) => rows
            .iter()
            .position(|r| field(r, "id") == id)
            .unwrap_or(rows.len()),
    };
    let list = List::new(items)
        .block(border)
        .highlight_style(style::selected(app.look))
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}
