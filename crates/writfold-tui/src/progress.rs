//! Recorded plan facts and step blocks, without derived obligations.
use crate::lines::{self, Line, Tone, line};
use crate::source::{Reply, field, flags};
use serde_json::Value;

pub fn facts(answer: &Reply) -> Option<Line> {
    let steps = answer.count("steps")?;
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
