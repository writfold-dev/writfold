//! Recorded plan facts and step blocks, without derived obligations.
use crate::lines::{self, Line, Tone, line};
use crate::source::{Reply, field, flags};
use crate::style::{self, Look};
use ratatui::text::{Line as Row, Span};
use serde_json::Value;

/// The map's plan summary from the trailer counts: neutral, except the
/// counts that flag a problem. The position falls back to the row the verb
/// marks current when the counts were not judged (no subject repository).
/// Parts wrap between, never inside, a count.
pub fn summary(answer: &Reply, full: bool, look: Look, width: u16) -> Vec<Row<'static>> {
    let Some(steps) = answer.count("steps").filter(|n| *n > 0) else {
        return Vec::new();
    };
    let judged = answer.count("current");
    let marked = || {
        let steps = answer.rows.iter().filter(|r| field(r, "kind") == "step");
        steps
            .enumerate()
            .find(|(_, r)| field(r, "status") == "current")
            .map(|(at, _)| at as u64 + 1)
    };
    let head = match judged.or_else(marked) {
        Some(0) => format!("{steps} steps; every step is done or skipped"),
        Some(at) => format!("step {at} of {steps}"),
        None => format!("{steps} steps"),
    };
    let mut parts = vec![(head, Tone::Plain)];
    for name in ["done", "skipped"] {
        parts.extend(
            answer
                .count(name)
                .map(|n| (format!("{n} {name}"), Tone::Plain)),
        );
    }
    for name in ["stale", "moved", "invalid", "disorder"] {
        if let Some(n) = answer.count(name).filter(|n| *n > 0) {
            let tone = if name == "moved" {
                Tone::Warn
            } else {
                Tone::Bad
            };
            parts.push((format!("{n} {name}"), tone));
        }
    }
    if judged.is_none() {
        parts.push(("gates not judged".into(), Tone::Dim));
    }
    let mut out = pack(parts, look, usize::from(width));
    if let (true, Some(owed), Some(gates)) = (
        full && judged.is_some_and(|n| n > 0),
        answer.count("current_owed"),
        answer.count("current_gates"),
    ) {
        let text = format!("owes {owed} of {gates} gates");
        out.push(Row::styled(text, style::style(look, Tone::Plain)));
    }
    out
}

/// Four cells after the position, two between counts; a part that would
/// cross the edge starts the next row.
pub(crate) fn pack(parts: Vec<(String, Tone)>, look: Look, width: usize) -> Vec<Row<'static>> {
    let mut rows: Vec<Vec<Span<'static>>> = Vec::new();
    let mut used = 0;
    for (at, (text, tone)) in parts.into_iter().enumerate() {
        let gap = if at == 1 { 4 } else { 2 };
        let size = Span::raw(text.as_str()).width();
        match rows.last_mut() {
            Some(row) if used + gap + size <= width => {
                row.push(Span::raw(" ".repeat(gap)));
                used += gap + size;
                row.push(Span::styled(text, style::style(look, tone)));
            }
            _ => {
                used = size;
                rows.push(vec![Span::styled(text, style::style(look, tone))]);
            }
        }
    }
    rows.into_iter().map(Row::from).collect()
}

pub fn facts(answer: &Reply) -> Option<Line> {
    // An unplanned run has no progress to state: its pane shows the
    // verb's no-plan note and the gates, never an "all done" line.
    let steps = answer.count("steps").filter(|n| *n > 0)?;
    let done = answer.count("done")?;
    let skipped = answer.count("skipped")?;
    let mut text = format!("plan {done} done, {skipped} skipped of {steps} steps");
    match answer.count("current") {
        Some(0) => text.push_str("; every step is done or skipped"),
        Some(current) => {
            text.push_str(&format!("; current step {current}/{steps}"));
            if let (Some(owed), Some(gates)) =
                (answer.count("current_owed"), answer.count("current_gates"))
            {
                text.push_str(&format!("; owes {owed} of {gates} gates"));
            }
        }
        None => text.push_str("; gates not judged (no subject repository)"),
    }
    for name in ["stale", "moved", "invalid", "disorder"] {
        if let Some(n) = answer.count(name).filter(|n| *n > 0) {
            text.push_str(&format!(" {name}={n}"));
        }
    }
    let role = if ["stale", "invalid", "disorder"]
        .iter()
        .any(|n| answer.count(n).is_some_and(|v| v > 0))
    {
        Tone::Bad
    } else if answer.count("current_owed").is_some_and(|n| n > 0) {
        Tone::Warn
    } else if answer.count("current") == Some(0) {
        Tone::Good
    } else {
        Tone::Head
    };
    Some(line(text, role))
}

fn mark(row: &Value, strip: bool) -> char {
    match field(row, "status") {
        "done" if strip && flags(row).contains(&"stale") => '!',
        "done" => 'x',
        "skipped" => '-',
        "current" => '>',
        _ => ' ',
    }
}

fn step_tone(row: &Value) -> Tone {
    match field(row, "status") {
        "current" => Tone::Current,
        "skipped" => Tone::Skipped,
        "later" => Tone::Later,
        "done" if flags(row).is_empty() => Tone::Done,
        _ => lines::tone(row),
    }
}

fn step(row: &Value, expanded: bool, positions: bool) -> Vec<Line> {
    let role = step_tone(row);
    let mut out = vec![line(
        format!(
            " {} {}",
            mark(row, false),
            lines::head(row, positions, positions)
        ),
        role,
    )];
    let flagged = !flags(row).is_empty() || lines::more(row).any(|m| !flags(m).is_empty());
    let status = field(row, "status");
    for m in lines::more(row) {
        if !expanded
            && !flagged
            && (status == "later" || (status == "done" && field(m, "label") != "done"))
        {
            continue;
        }
        let show_path = positions || field(m, "label") == "read";
        out.push(lines::continuation(m, lines::tone(row), true, show_path));
    }
    out
}

pub fn content(
    answer: &Reply,
    width: Option<u16>,
    expanded: bool,
    positions: bool,
) -> (Vec<Line>, Option<usize>) {
    if answer.error.is_some() {
        return (lines::reply(answer), None);
    }
    let mut out: Vec<_> = facts(answer).into_iter().collect();
    let steps: Vec<_> = answer
        .rows
        .iter()
        .filter(|r| field(r, "kind") == "step")
        .collect();
    if !steps.is_empty()
        && width.is_none_or(|w| steps.len().saturating_mul(4).saturating_add(5) <= usize::from(w))
    {
        let cells: Vec<_> = steps
            .iter()
            .map(|r| format!("[{}]", mark(r, true)))
            .collect();
        out.push(line(format!("     {}", cells.join(" ")), Tone::Plain));
    }
    let mut current = None;
    for row in &answer.rows {
        if field(row, "kind") == "step" {
            if field(row, "status") == "current" {
                current = Some(out.len());
            }
            out.extend(step(row, expanded, positions));
        } else {
            let mut rows = lines::row(row);
            if field(row, "kind") == "plan-event" {
                for line in &mut rows {
                    line.tone = Tone::Bad;
                }
            }
            out.extend(rows);
        }
    }
    out.extend(lines::notes(answer));
    (out, current)
}
