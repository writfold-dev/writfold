//! The command rows (`c`): ids and commands of what is on screen, as
//! borderless full-width lines a terminal selection copies cleanly. Ids
//! and paths are `#` comment lines; a command is printed whole, exactly
//! as the verb gave it, broken at a space with a trailing ` \` when it is
//! longer than the frame. `watch` runs none of them.
use crate::app::{App, View};
use crate::kb_state::Mode;
use crate::lines::more;
use crate::source::{Reply, field};
use crate::style;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use serde_json::Value;

/// At most this many rows, the first and the last included.
pub const ROWS: usize = 10;

const HEAD: &str = "copy into a shell; watch never runs them · UPPERCASE words are placeholders";

/// The labels whose text is a command or a file to read.
const COMMANDS: [&str; 4] = ["next", "confirm", "accept", "read"];

/// One row's commands, in the verb's order; `read` names its file.
fn continuations(out: &mut Vec<String>, row: &Value) {
    for value in more(row) {
        let label = field(value, "label");
        if !COMMANDS.contains(&label) {
            continue;
        }
        if label == "read" {
            let path = field(value, "path");
            if !path.is_empty() {
                out.push(format!("# read  {path}"));
            }
        } else {
            out.push(field(value, "text").to_owned());
        }
    }
}

fn view_rows(answer: &Reply, out: &mut Vec<String>) {
    for row in &answer.rows {
        let mut own = Vec::new();
        continuations(&mut own, row);
        if own.is_empty() && field(row, "kind") != "guard" {
            continue;
        }
        out.push(format!("# {} {}", field(row, "kind"), field(row, "id")));
        if field(row, "kind") == "guard" {
            out.push(format!("# file  {}", field(row, "path")));
        }
        out.extend(own);
    }
}

/// The title and the lines, before wrapping.
pub fn rows(app: &App) -> (String, Vec<String>) {
    if app.mode == Mode::Kb {
        return kb_rows(app);
    }
    let mut out = Vec::new();
    let run = app.run().unwrap_or_default();
    out.push(format!("# run   {run}"));
    if app.view == View::Progress && app.map_visible() {
        let nodes = app.answer(View::Progress).map(crate::graph::nodes);
        let node = nodes
            .iter()
            .flatten()
            .find(|n| Some(n.id) == app.node.as_deref());
        if let Some(node) = node {
            out.push(format!("# step  {}", node.id));
            continuations(&mut out, node.row);
            let name = crate::say_step::name(node);
            return (format!("commands for step {name} · {HEAD}"), out);
        }
    }
    if let Some(answer) = app.answer(app.view).filter(|a| a.error.is_none()) {
        view_rows(answer, &mut out);
    }
    (format!("commands for {} · {HEAD}", app.view.name()), out)
}

fn kb_rows(app: &App) -> (String, Vec<String>) {
    let entry = app.kb_entry();
    let id = entry
        .as_ref()
        .map(|e| e.id.clone())
        .or_else(|| app.kb_scope().id().map(str::to_owned))
        .unwrap_or_default();
    let mut out = vec![format!("# record {id}")];
    if let Some(entry) = &entry
        && !entry.path.is_empty()
    {
        let line = entry.line.map(|n| format!(":{n}")).unwrap_or_default();
        out.push(format!("# file   {}{line}", entry.path));
    }
    let kind = entry.as_ref().map(|e| e.kind.as_str()).unwrap_or("project");
    if id.starts_with("code:") {
        out.push(format!("writfold kb uses {id}"));
    } else if kind != "cite" && !id.is_empty() {
        out.push(format!("writfold kb status {id}"));
        if kind == "decision" {
            out.push(format!("writfold kb why {id}"));
        }
        out.push(format!("writfold kb timeline {id}"));
    }
    (format!("commands for this record · {HEAD}"), out)
}

/// Lines no wider than `width` cells: a longer command breaks at a space
/// and ends in ` \`; a longer comment continues on a `#` line.
pub fn wrap(lines: &[String], width: usize) -> Vec<String> {
    let cells = |s: &str| s.chars().count();
    let mut out = Vec::new();
    for line in lines {
        let comment = line.starts_with('#');
        let (tail, lead) = if comment { ("", "#   ") } else { (" \\", "  ") };
        let mut row = String::new();
        let mut fresh = true;
        for word in line.split(' ') {
            if !fresh && cells(&row) + 1 + cells(word) + cells(tail) > width {
                out.push(format!("{row}{tail}"));
                row = lead.to_owned();
                fresh = true;
            }
            if !fresh {
                row.push(' ');
            }
            row.push_str(word);
            fresh = false;
        }
        out.push(row);
    }
    out
}

/// Replace the bottom rows of the frame with the command rows.
pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    let width = usize::from(area.width.saturating_sub(1));
    let (head, lines) = rows(app);
    let body = wrap(&lines, width);
    let room = (ROWS - 2).min(usize::from(area.height.saturating_sub(4)));
    let last = body.len().saturating_sub(room);
    app.commands_scroll = app.commands_scroll.min(u16::try_from(last).unwrap_or(0));
    let start = usize::from(app.commands_scroll);
    let shown: Vec<&String> = body.iter().skip(start).take(room).collect();
    let more = if body.len() > room {
        "j/k scroll   any other key closes"
    } else {
        "any key closes"
    };
    let dim = style::style(app.look, crate::lines::Tone::Dim);
    let mut rows = vec![Line::styled(
        format!(
            " {}",
            crate::rail_text::clip(&head, width, crate::map_view::glyphs(app))
        ),
        style::style(app.look, crate::lines::Tone::Head),
    )];
    rows.extend(shown.into_iter().map(|l| Line::raw(format!(" {l}"))));
    rows.push(Line::styled(format!(" {more}"), dim));
    let height = u16::try_from(rows.len())
        .unwrap_or(u16::MAX)
        .min(area.height);
    let at = Rect::new(area.x, area.bottom() - height, area.width, height);
    frame.render_widget(Clear, at);
    frame.render_widget(Paragraph::new(rows).style(style::base(app.look)), at);
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn long_commands_break_at_spaces_with_a_continuation() {
        let line = "writfold kb gate cover review-list tests supported REF… --by WHO".to_owned();
        let rows = wrap(std::slice::from_ref(&line), 30);
        assert!(rows.len() > 1, "{rows:?}");
        for row in &rows[..rows.len() - 1] {
            assert!(row.ends_with(" \\") && row.chars().count() <= 30, "{row}");
        }
        let joined: String = rows
            .iter()
            .map(|r| r.trim_end_matches(" \\").trim_start().to_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(joined, line);
        let comment = wrap(&["# step a/b/c/d/e/f/g/h/i/j/k/l/m".into()], 12);
        assert!(comment.iter().all(|r| r.starts_with('#')), "{comment:?}");
    }
}
