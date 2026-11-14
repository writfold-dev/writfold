//! Gate counts and compact projections of the same full-pane answers.
use crate::app::{App, View};
use crate::lines::{self, Line, Tone, line};
use crate::source::{Reply, field};

const GATE_COUNTS: &[&str] = &[
    "gates",
    "satisfied",
    "open",
    "stale",
    "failed",
    "unknown",
    "attested",
    "waived",
    "invalid",
];

pub(crate) fn count_parts(answer: &Reply, names: &[&str]) -> String {
    names
        .iter()
        .filter_map(|n| answer.count(n).map(|v| format!("{n}={v}")))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn gate_summary(answer: &Reply) -> Line {
    if answer.trailer_text("reason").as_deref() == Some("no-gates") {
        return line("gates: none defined", Tone::Dim);
    }
    if let Some(error) = &answer.error {
        return line(format!("gates: {error}"), Tone::Dim);
    }
    let bad = ["stale", "failed", "invalid"]
        .iter()
        .any(|n| answer.count(n).is_some_and(|v| v > 0));
    let warn = ["open", "unknown"]
        .iter()
        .any(|n| answer.count(n).is_some_and(|v| v > 0));
    let role = if bad {
        Tone::Bad
    } else if warn {
        Tone::Warn
    } else {
        Tone::Good
    };
    line(format!("gates: {}", count_parts(answer, GATE_COUNTS)), role)
}

pub fn gates(answer: &Reply) -> Vec<Line> {
    if answer.error.is_some() {
        return lines::reply(answer);
    }
    let mut out = vec![gate_summary(answer)];
    out.extend(lines::reply(answer));
    out
}

fn inbox_rows(answer: &Reply) -> impl Iterator<Item = &serde_json::Value> {
    answer
        .rows
        .iter()
        .filter(|r| matches!(field(r, "kind"), "draft" | "proposal"))
}

fn has_tracked(answer: &Reply) -> bool {
    inbox_rows(answer).any(|row| crate::source::flags(row).contains(&"tracked"))
}

pub fn inbox(answer: &Reply) -> Vec<Line> {
    if answer.error.is_some() {
        return lines::reply(answer);
    }
    let caption = if has_tracked(answer) {
        "drafts and proposals; not records until confirmed or accepted"
    } else {
        "local; not in the KB until confirmed or accepted"
    };
    let mut out = vec![line(caption, Tone::Dim)];
    let rows: Vec<_> = inbox_rows(answer).flat_map(lines::row).collect();
    if rows.is_empty() {
        out.push(line("(none)", Tone::Dim));
    }
    out.extend(rows);
    out.extend(lines::notes(answer));
    out
}

fn inbox_counts(answer: &Reply) -> String {
    let drafts = inbox_rows(answer)
        .filter(|r| field(r, "kind") == "draft")
        .count();
    let proposals = inbox_rows(answer)
        .filter(|r| field(r, "kind") == "proposal")
        .count();
    format!("drafts={drafts} proposals={proposals}")
}

fn compact_gates(answer: &Reply) -> Vec<Line> {
    if answer.trailer_text("reason").as_deref() == Some("no-gates") {
        return gates(answer);
    }
    let role = gate_summary(answer).tone;
    let mut out = vec![
        line(count_parts(answer, &GATE_COUNTS[..5]), role),
        line(count_parts(answer, &GATE_COUNTS[5..]), role),
    ];
    for row in answer.rows.iter().filter(|r| field(r, "kind") == "gate") {
        out.push(line(
            format!(
                "{} [{}]{}",
                field(row, "id"),
                field(row, "status"),
                lines::suffix(row, false)
            ),
            lines::tone(row),
        ));
    }
    out.extend(lines::notes(answer));
    out
}

fn compact_inbox(answer: &Reply) -> Vec<Line> {
    let caption = if has_tracked(answer) {
        "pending records"
    } else {
        "local; not in the KB"
    };
    let mut out = vec![line(
        format!("{} ({caption})", inbox_counts(answer)),
        Tone::Dim,
    )];
    for row in inbox_rows(answer) {
        out.push(line(
            format!(
                "{} {} [{}]",
                field(row, "kind"),
                field(row, "id"),
                field(row, "status")
            ),
            lines::tone(row),
        ));
        out.push(line(
            format!("  {}{}", field(row, "text"), lines::suffix(row, false)),
            lines::tone(row),
        ));
    }
    if out.len() == 1 {
        out.push(line("(none)", Tone::Dim));
    }
    out
}

pub fn compact(app: &App, view: View) -> Vec<Line> {
    let Some(answer) = app.answer(view) else {
        return vec![line("(no run selected)", Tone::Dim)];
    };
    if answer.error.is_some() {
        return lines::reply(answer);
    }
    match view {
        View::Gates => compact_gates(answer),
        View::Inbox => compact_inbox(answer),
        View::Guards => crate::guards::content(app, answer, true),
        View::Timeline => {
            let mut out: Vec<_> = answer
                .rows
                .iter()
                .map(|r| line(lines::head(r, true, false), lines::tone(r)))
                .collect();
            out.extend(lines::notes(answer));
            out
        }
        _ => lines::reply(answer),
    }
}

pub fn summary(app: &App) -> String {
    let gates = match app.answer(View::Gates) {
        Some(answer) if answer.error.is_some() => "gates: refused".into(),
        Some(answer) => {
            let parts = answer
                .counts()
                .into_iter()
                .filter(|(n, v)| n != "satisfied" && n != "gates" && *v > 0)
                .map(|(n, v)| format!("{n}={v}"))
                .collect::<Vec<_>>()
                .join(" ");
            match answer.count("gates") {
                Some(n) => format!("gates: {parts} of {n}"),
                None => gate_summary(answer).text,
            }
        }
        None => "gates: no run selected".into(),
    };
    let inbox = match app.answer(View::Inbox) {
        Some(answer) if answer.error.is_some() => "refused".into(),
        Some(answer) => inbox_counts(answer),
        None => "no run selected".into(),
    };
    format!("{gates} | inbox: {inbox} | {}", crate::guards::summary(app))
}
