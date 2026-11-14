//! Draw only the visible rail window, followed by recorded verb facts.
use crate::app::{App, Focus, View};
use crate::cells::Glyphs;
use crate::lines::{self, Tone};
use crate::source::field;
use crate::{graph, paint, progress, rail, style};
use ratatui::{Frame, layout::Rect, style::Modifier, text::Line, widgets::Paragraph};

pub(crate) fn glyphs(app: &App) -> Glyphs {
    if app.dumb || app.look.depth == style::Depth::None {
        Glyphs::Ascii
    } else {
        Glyphs::Unicode
    }
}

pub(crate) fn records(app: &App) -> Vec<lines::Line> {
    let mut out = Vec::new();
    if let Some(answer) = app.answer(View::Progress) {
        let position = match (answer.count("current"), answer.count("steps")) {
            (Some(0), _) => "every step is done or skipped".into(),
            (Some(current), Some(total)) => {
                format!("step {current}/{total} (first step neither done nor skipped)")
            }
            _ => "gates not judged (no subject repository)".into(),
        };
        out.push(lines::line(
            format!("recorded position: {position}"),
            Tone::Dim,
        ));
    }
    let event = app
        .answer(View::Timeline)
        .and_then(|a| a.rows.iter().rev().find(|r| field(r, "kind") == "plan"));
    out.push(lines::line(
        format!(
            "last plan event: {}",
            event.map_or("(none recorded)", |r| field(r, "text"))
        ),
        Tone::Dim,
    ));
    let checkpoint = app
        .row()
        .and_then(|r| lines::more(r).find(|m| field(m, "label") == "checkpoint"));
    out.push(lines::line(
        format!(
            "last checkpoint: {}",
            checkpoint.map_or("(none recorded)", |r| field(r, "text"))
        ),
        Tone::Dim,
    ));
    out
}

fn legend(glyphs: Glyphs) -> [&'static str; 2] {
    if glyphs == Glyphs::Unicode {
        [
            "● satisfied ◆ attested/waived ○ open ! stale × failed ? unknown",
            "· later, not judged; ▶ recorded current step, not a liveness claim",
        ]
    } else {
        [
            "o satisfied/open * attested/waived ! stale x failed ? unknown",
            ". later, not judged; [>] recorded current step, not a liveness claim",
        ]
    }
}

pub(crate) fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = paint::block(
        format!("progress: map {}", app.run_id().unwrap_or_default()),
        app.focus == Focus::Detail,
        app.look,
    );
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
    let rail = rail::build(&nodes, inner.width, glyphs, app.look);
    let mut records = records(app);
    if inner.width >= 64 {
        records.extend(legend(glyphs).map(|s| lines::line(s, Tone::Dim)));
    }
    records.truncate(usize::from(inner.height.saturating_sub(4)));
    let top = prelude(frame, answer, inner, app.look);
    let height = inner.height.saturating_sub(top + records.len() as u16);
    let window = rail::window(&rail, selected, usize::from(height), app.rail_offset);
    let selected_line = rail.node_lines.get(selected).copied();
    for (y, mut line) in rail
        .lines
        .into_iter()
        .skip(window.start)
        .take(usize::from(height))
        .enumerate()
    {
        if selected_line == Some(window.start + y) {
            line = line.style(style::style(app.look, Tone::Plain).add_modifier(Modifier::REVERSED));
        }
        frame.render_widget(
            Paragraph::new(line),
            Rect::new(inner.x, inner.y + top + y as u16, inner.width, 1),
        );
    }
    for (y, line) in records.into_iter().enumerate() {
        frame.render_widget(
            Paragraph::new(Line::styled(line.text, style::style(app.look, line.tone))),
            Rect::new(inner.x, inner.y + top + height + y as u16, inner.width, 1),
        );
    }
    app.rail_offset = window.start;
}

fn prelude(
    frame: &mut Frame<'_>,
    answer: &crate::source::Reply,
    area: Rect,
    look: style::Look,
) -> u16 {
    let mut lines: Vec<_> = progress::facts(answer).into_iter().collect();
    if answer.count("current").is_none() {
        lines.extend(
            answer
                .notes
                .first()
                .map(|n| lines::line(format!("# note: {n}"), Tone::Warn)),
        );
    }
    let top = lines.len() as u16;
    for (y, line) in lines.into_iter().enumerate() {
        frame.render_widget(
            Paragraph::new(Line::styled(line.text, style::style(look, line.tone))),
            Rect::new(area.x, area.y + y as u16, area.width, 1),
        );
    }
    top
}
