//! Drawing one frame with ratatui: the runs on the left, the selected
//! run's pane on the right, the KB pin at the bottom, help on top.
use crate::app::{App, Focus, View};
use crate::lines::{self, Tone};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

fn style(tone: Tone) -> Style {
    match tone {
        Tone::Plain => Style::default(),
        Tone::Dim => Style::default().fg(Color::DarkGray),
        Tone::Head => Style::default().add_modifier(Modifier::BOLD),
        Tone::Good => Style::default().fg(Color::Green),
        Tone::Warn => Style::default().fg(Color::Yellow),
        Tone::Bad => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    }
}

fn text(lines: Vec<lines::Line>) -> Text<'static> {
    Text::from(
        lines
            .into_iter()
            .map(|l| Line::styled(l.text, style(l.tone)))
            .collect::<Vec<_>>(),
    )
}

fn block(title: String, focused: bool) -> Block<'static> {
    let border = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Line::styled(title, style(Tone::Head)))
}

/// Draw the whole frame.
pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let [body, foot] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).areas(body);
    runs(frame, app, left);
    let (title, detail) = lines::detail(app);
    let tabs = match app.view {
        View::Resume => "[resume] stale brief",
        View::Stale => "resume [stale] brief",
        View::Brief => "resume stale [brief]",
    };
    let pane = Paragraph::new(text(detail))
        .block(block(
            format!(" {title}  ({tabs}) "),
            app.focus == Focus::Detail,
        ))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    frame.render_widget(pane, right);
    let footer = format!(" {}  ? help  q quit", lines::footer(app));
    frame.render_widget(Paragraph::new(Line::styled(footer, style(Tone::Dim))), foot);
    if app.help {
        help(frame);
    }
}

fn runs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem<'static>> = app
        .order
        .iter()
        .filter_map(|i| app.runs.rows.get(*i))
        .map(|row| ListItem::new(text(lines::run_item(row))))
        .collect();
    let title = format!(" runs ({}) ", items.len());
    let block = block(title, app.focus == Focus::Runs);
    if let Some(error) = &app.runs.error {
        let message = Paragraph::new(Line::styled(error.clone(), style(Tone::Bad)))
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(message, area);
        return;
    }
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn help(frame: &mut Frame<'_>) {
    let area = frame.area();
    let width = area.width.min(68);
    let height = area
        .height
        .min(u16::try_from(lines::HELP.len()).unwrap_or(u16::MAX) + 2);
    let popup = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    let body: Vec<Line<'static>> = lines::HELP.iter().map(|l| Line::raw(*l)).collect();
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(body).block(block(" help ".into(), true)),
        popup,
    );
}
