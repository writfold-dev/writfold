//! Guard facts belong to the verb's current run, not the selection.
use crate::app::{App, View};
use crate::lines::{self, Line, Tone, line};
use crate::source::{Reply, field};

pub(crate) fn current(answer: &Reply) -> Option<&str> {
    answer.notes.iter().find_map(|note| {
        note.strip_prefix("judged for the current run ")?
            .split_once(" in ")
            .map(|(id, _)| id)
    })
}

pub fn title(app: &App) -> String {
    if app.answer(View::Guards).is_some_and(|a| a.error.is_some()) {
        "guards".into()
    } else {
        format!(
            "guards (current run {})",
            app.current_run().unwrap_or("none")
        )
    }
}

fn counts(answer: &Reply) -> Line {
    let role = if ["deny", "invalid"]
        .iter()
        .any(|n| answer.count(n).is_some_and(|n| n > 0))
    {
        Tone::Bad
    } else if ["ask", "nudge"]
        .iter()
        .any(|n| answer.count(n).is_some_and(|n| n > 0))
    {
        Tone::Warn
    } else if answer.count("rules").is_some_and(|n| n > 0) {
        Tone::Good
    } else {
        Tone::Dim
    };
    let parts = crate::side::count_parts(answer, &["rules", "deny", "ask", "nudge", "invalid"]);
    line(parts, role)
}

pub fn content(app: &App, answer: &Reply, compact: bool) -> Vec<Line> {
    if answer.error.is_some() {
        return lines::reply(answer);
    }
    let mut out = Vec::new();
    if app.run_id() != app.current_run() {
        out.push(line(
            format!(
                "about the current run {}, not the selected run",
                app.current_run().unwrap_or("none")
            ),
            Tone::Dim,
        ));
    }
    out.push(counts(answer));
    if field(&answer.trailer, "reason") == "no-guards" {
        out.push(line("(no guard rules)", Tone::Dim));
    }
    if compact {
        for row in &answer.rows {
            let tone = lines::tone(row);
            out.push(line(lines::head(row, false, false), tone));
            if let Some(owed) = lines::more(row).find(|m| field(m, "label") == "owes") {
                out.push(lines::continuation(owed, tone, false, false));
            }
        }
    } else {
        out.extend(lines::reply(answer));
    }
    out
}

pub fn summary(app: &App) -> String {
    let Some(answer) = app.answer(View::Guards) else {
        return "guards: not asked".into();
    };
    if answer.error.is_some() {
        return "guards: refused".into();
    }
    match (
        answer.count("deny"),
        answer.count("ask"),
        answer.count("rules"),
    ) {
        (Some(deny), Some(ask), Some(rules)) => format!("guards: deny={deny} ask={ask} of {rules}"),
        _ => "guards: unknown".into(),
    }
}
