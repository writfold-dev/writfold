//! Draw only the visible rail window, followed by recorded verb facts.
use crate::app::{App, Focus, View};
use crate::cells::Glyphs;
use crate::lines::{self, Tone};
use crate::source::field;
use crate::{graph, paint, progress, rail, style, wrap};
use ratatui::{Frame, layout::Rect, style::Modifier, widgets::Paragraph};

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
        // The row the verb marks current is the position, judged or not.
        let nodes = graph::nodes(answer);
        let current = nodes
            .iter()
            .position(|n| field(n.row, "status") == "current");
        let position = match current {
            Some(at) => format!(
                "step {}/{} (first step neither done nor skipped)",
                at + 1,
                nodes.len()
            ),
            None if nodes.is_empty() => "no plan steps recorded".into(),
            None => "every step is done or skipped".into(),
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
            "· later, not judged; ▶ next step to pick up (recorded, not a liveness claim)",
        ]
    } else {
        [
            "+ satisfied * attested/waived o open ! stale x failed ? unknown",
            ". later, not judged; [>] next step to pick up (recorded, not a liveness claim)",
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
    records.extend(legend(glyphs).map(|s| lines::line(s, Tone::Dim)));
    let top = prelude(answer);
    // Facts and records wrap; the rail keeps at least four rows, so the
    // legend goes first, then the oldest record lines, when space is short.
    let budget = usize::from(inner.height.saturating_sub(4).min(inner.height / 2));
    let tall = |set: &[lines::Line]| {
        set.iter()
            .map(|l| wrap::height(l, inner.width))
            .sum::<usize>()
    };
    while !records.is_empty() && tall(&top) + tall(&records) > budget {
        records.pop();
    }
    let top_height = tall(&top).min(budget) as u16;
    let records_height = tall(&records) as u16;
    wrap::draw(
        frame,
        &top,
        Rect {
            height: top_height,
            ..inner
        },
        app.look,
        0,
    );
    let top = top_height;
    let height = inner.height.saturating_sub(top + records_height);
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
    let below = Rect::new(inner.x, inner.y + top + height, inner.width, records_height);
    wrap::draw(frame, &records, below, app.look, 0);
    app.rail_offset = window.start;
}

fn prelude(answer: &crate::source::Reply) -> Vec<lines::Line> {
    let mut lines: Vec<_> = progress::facts(answer).into_iter().collect();
    if answer.count("current").is_none() {
        lines.extend(
            answer
                .notes
                .first()
                .map(|n| lines::line(format!("# note: {n}"), Tone::Warn)),
        );
    }
    lines
}
