//! A step in plain words: its name as the map shows it, where it sits,
//! what it needs, and the few words a map row carries. Every state word
//! is a lookup on the row's `status` and `flags` (and the plan trailer's
//! `current_owed`); nothing is read out of the verb's sentences.
use crate::graph::Node;
use crate::lines::{Tone, more};
use crate::rail_text::shown_id;
use crate::say::{Phrase, Piece, said};
use crate::say_shape::{template, when};
use crate::say_words::{STEP, STEP_FLAG, find};
use crate::source::{Reply, field, flags};
use serde_json::Value;

/// Standings that count as met: the product's definition.
pub const MET: [&str; 3] = ["satisfied", "attested", "waived"];

/// The step's name as the map shows it.
pub fn name<'a>(node: &Node<'a>) -> &'a str {
    shown_id(node.id, node.group, false)
}

/// The group header: the template's name, or the splice's first word.
pub fn group_name(splice: &str) -> &str {
    template(splice).map_or_else(|| crate::rail_text::splice_name(splice), |(name, _)| name)
}

/// The chip word beside the name (`next up`, `done`, `skipped`).
pub fn chip(node: &Node<'_>) -> Option<&'static str> {
    // A loop (#81): a spent loop says so; a step of an earlier round is
    // history, not a step to do again.
    if node.flags.contains(&"spent") {
        return Some("spent");
    }
    if node.row.get("superseded").and_then(Value::as_bool) == Some(true) {
        return Some("earlier round");
    }
    find(STEP, field(node.row, "status"))
        .map(|w| w.short)
        .filter(|s| !s.is_empty())
}

/// A loop step's round, as `round 2 of at most 5` (#81).
pub fn round(node: &Node<'_>) -> Option<String> {
    let lp = node.row.get("loop")?;
    let rounds = lp.get("rounds")?.as_u64()?;
    let max = lp.get("max")?.as_u64()?;
    Some(format!("round {rounds} of at most {max}"))
}

/// `step 7 of 11 · in findings`.
pub fn place(node: &Node<'_>, index: usize, total: usize) -> Phrase {
    let mut pieces = vec![said(format!("step {} of {total}", index + 1))];
    if !node.group.is_empty() {
        pieces.push(said(format!("in {}", group_name(node.group))));
    }
    Phrase::new(0, Tone::Dim, pieces)
}

fn gates(row: &Value) -> impl Iterator<Item = &Value> {
    more(row).filter(|m| field(m, "label") == "gate")
}

/// The step's gate names whose own status is `status` (or, with `None`,
/// any status that is not met).
fn gate_names(row: &Value, status: Option<&str>) -> Vec<String> {
    gates(row)
        .filter(|g| {
            let own = field(g, "status");
            status.map_or(!MET.contains(&own), |s| own == s)
        })
        .filter_map(|g| crate::say_shape::named(g).map(|(id, _)| id.to_owned()))
        .collect()
}

fn listed(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// When the step was recorded done, respelt: `2026-11-10 23:50`.
pub fn done_at(row: &Value) -> Option<String> {
    let done = more(row).find(|m| field(m, "label") == "done")?;
    let (at, rest) = when(field(done, "text").strip_prefix("recorded ")?)?;
    rest.is_empty().then_some(at)
}

/// The sentences under the head: what the step's state means.
pub fn needs(node: &Node<'_>, owed: Option<u64>) -> Vec<Phrase> {
    let row = node.row;
    let sentence = |text: String| Phrase::new(0, Tone::Plain, vec![said(text)]);
    let mut out = Vec::new();
    match field(row, "status") {
        "current" => out.push(sentence(match owed {
            Some(0) => "Its gates are met.".into(),
            Some(1) => "Needs 1 gate before it can be marked done.".into(),
            Some(n) => format!("Needs {n} gates before it can be marked done."),
            None => "Its gates are not judged here (no repository).".into(),
        })),
        "done" => {
            let at = done_at(row).map_or_else(|| "Done.".into(), |at| format!("Done on {at}."));
            out.push(sentence(at));
            let stale = gate_names(row, Some("stale")).len();
            if flags(row).contains(&"stale") && stale > 0 {
                let of = if stale == 1 { "gates is" } else { "gates are" };
                out.push(sentence(format!(
                    "It stays done, but {stale} of its {of} now out of date."
                )));
            }
        }
        "skipped" => out.push(sentence("Skipped.".into())),
        "later" => out.push(sentence(
            "Later in the plan. Its gates are not judged yet.".into(),
        )),
        other => out.push(Phrase::new(
            0,
            Tone::Plain,
            vec![Piece::Quoted(other.into())],
        )),
    }
    for flag in flags(row) {
        if let Some(word) = find(STEP_FLAG, flag) {
            out.push(Phrase::new(0, Tone::Bad, vec![said(capital(word.long))]));
        }
    }
    out
}

fn capital(text: &str) -> String {
    let mut chars = text.chars();
    let first = chars.next().map(|c| c.to_uppercase().collect::<String>());
    format!("{}{}.", first.unwrap_or_default(), chars.as_str())
}

/// The words a map row carries: always for the recorded current step and
/// flagged steps, for the selected row too; `None` keeps the row bare.
pub fn row(node: &Node<'_>, selected: bool) -> Option<(String, Tone)> {
    let row = node.row;
    let status = field(row, "status");
    let flagged: Vec<&str> = flags(row)
        .into_iter()
        .filter_map(|f| find(STEP_FLAG, f).map(|w| w.short))
        .collect();
    let stale = gate_names(row, Some("stale"));
    let stale_done = status == "done" && flags(row).contains(&"stale") && !stale.is_empty();
    let base = match status {
        "current" => {
            let owed = gate_names(row, None);
            Some(if owed.is_empty() {
                "its gates are met".to_owned()
            } else {
                format!("needs {}", listed(&owed))
            })
        }
        "done" if stale_done => {
            let verb = if stale.len() == 1 { "is" } else { "are" };
            Some(format!("done; {} {verb} now out of date", listed(&stale)))
        }
        "done" if selected => Some(done_at(row).map_or("done".into(), |at| format!("done {at}"))),
        "skipped" if selected => Some("skipped".into()),
        "later" if selected => Some("later in the plan".into()),
        _ => None,
    };
    let mut parts: Vec<String> = base.into_iter().collect();
    parts.extend(flagged.iter().map(|f| (*f).to_owned()));
    let tone = if stale_done || !flagged.is_empty() {
        Tone::Bad
    } else {
        Tone::Dim
    };
    (!parts.is_empty()).then(|| (parts.join(" · "), tone))
}

/// The plan head's parts: the position, then non-zero counts in words.
pub fn head(answer: &Reply) -> Vec<(String, Tone)> {
    let Some(steps) = answer.count("steps").filter(|n| *n > 0) else {
        return Vec::new();
    };
    let judged = answer.count("current");
    let marked = || {
        let steps = answer.rows.iter().filter(|r| field(r, "kind") == "step");
        let mut steps = steps.enumerate();
        steps
            .find(|(_, r)| field(r, "status") == "current")
            .map(|(at, _)| at as u64 + 1)
    };
    let position = match judged.or_else(marked) {
        Some(0) => format!("{steps} steps; every step is done or skipped"),
        Some(at) => format!("step {at} of {steps}"),
        None => format!("{steps} steps"),
    };
    let mut parts = vec![(position, Tone::Plain)];
    for (name, word, tone) in [
        ("done", "done", Tone::Plain),
        ("skipped", "skipped", Tone::Plain),
        ("stale", "out of date", Tone::Bad),
        ("moved", "with a changed template", Tone::Warn),
        ("invalid", "invalid", Tone::Bad),
        ("disorder", "out of order", Tone::Bad),
    ] {
        if let Some(n) = answer.count(name).filter(|n| *n > 0) {
            parts.push((format!("{n} {word}"), tone));
        }
    }
    if judged.is_none() {
        parts.push(("gates not judged (no repository)".into(), Tone::Dim));
    }
    parts
}

/// The newest plan event, in words: `2026-11-10 23:59 · prepare done`.
/// The step is named as the map names it; `None` when the row's text
/// does not have the event's shape or the step is not on the map.
pub fn last_event(event: &Value, nodes: &[Node<'_>]) -> Option<String> {
    let (at, rest) = crate::say_shape::event(event)?;
    let id = field(event, "id");
    let what = match field(event, "status") {
        "spliced" => format!("{} added", spliced(event, nodes)?),
        status => {
            let node = nodes.iter().find(|n| n.id == id)?;
            let word = find(STEP, status).map_or(status, |w| w.short);
            format!("{} {word}", name(node))
        }
    };
    let rest = crate::say::quote(rest).map(|p| format!(" {}", p.text()));
    Some(format!("{at} · {what}{}", rest.unwrap_or_default()))
}

/// The template a `spliced` event added: the group whose splice file is
/// the event's own file and whose splice name ends the event's id.
pub fn spliced<'a>(event: &Value, nodes: &[Node<'a>]) -> Option<&'a str> {
    let (path, id) = (field(event, "path"), field(event, "id"));
    nodes.iter().find_map(|node| {
        let splice = more(node.row).find(|m| field(m, "label") == "splice")?;
        let (template, ordinal) = template(node.group)?;
        let own = format!("{template}#{ordinal}");
        let ends = id == own || id.ends_with(&format!("/{own}"));
        (field(splice, "path") == path && ends).then_some(template)
    })
}
