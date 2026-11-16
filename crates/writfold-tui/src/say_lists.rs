//! The Inbox and Guards views in plain words: counts first, then one
//! entry per row; the verbs' notes become their fixed sentences only when
//! recognised.
use crate::lines::{self, Tone};
use crate::say::{Phrase, Piece, Told, align, quote, said};
use crate::say_row::notes;
use crate::say_shape::requires;
use crate::say_words::{CONDITION, GUARD, MOVED, short};
use crate::source::{Reply, field, flags};
use serde_json::Value;

fn inbox_rows(answer: &Reply) -> impl Iterator<Item = &Value> {
    answer
        .rows
        .iter()
        .filter(|r| matches!(field(r, "kind"), "draft" | "proposal"))
}

/// `3 pending · 1 draft · 2 proposals`.
pub fn inbox_counts(answer: &Reply) -> Phrase {
    let count = |kind: &str| {
        inbox_rows(answer)
            .filter(|r| field(r, "kind") == kind)
            .count()
    };
    let (drafts, proposals) = (count("draft"), count("proposal"));
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let mut pieces = vec![said(format!("{} pending", drafts + proposals))];
    pieces.extend((drafts > 0).then(|| said(plural(drafts, "draft"))));
    pieces.extend((proposals > 0).then(|| said(plural(proposals, "proposal"))));
    Phrase::new(0, Tone::Dim, pieces)
}

pub fn inbox(answer: &Reply) -> Told {
    let mut told = Told::default();
    told.lines.push(inbox_counts(answer));
    let tracked = inbox_rows(answer).any(|r| flags(r).contains(&"tracked"));
    let caption = if tracked {
        "Nothing here is a record until someone confirms or accepts it."
    } else {
        "Nothing here is in the KB until someone confirms or accepts it."
    };
    told.lines
        .push(Phrase::new(0, Tone::Dim, vec![said(caption)]));
    told.lines.push(Phrase::blank());
    let width = inbox_rows(answer)
        .map(|r| field(r, "kind").len())
        .max()
        .unwrap_or(0)
        + 3;
    for row in inbox_rows(answer) {
        let mut state = vec![short(MOVED, field(row, "status"))];
        state.extend(
            flags(row)
                .into_iter()
                .filter(|f| *f != "proposal" && *f != "draft")
                .map(|f| short(MOVED, f)),
        );
        if field(row, "kind") == "draft" {
            state = vec![crate::say_words::long(MOVED, "draft")];
        }
        let tone = lines::tone(row);
        let mut head = vec![said(field(row, "kind"))];
        let indent = u16::try_from(width).unwrap_or(u16::MAX);
        match quote(field(row, "text")) {
            Some(text) => {
                head.push(text);
                told.lines.push(Phrase::led(0, tone, head, 1));
                told.lines.push(Phrase::new(indent, Tone::Dim, state));
            }
            None => {
                head.extend(state);
                told.lines.push(Phrase::led(0, tone, head, 1));
                let note = said("its text is in exact wording (d)");
                told.lines.push(Phrase::new(indent, Tone::Dim, vec![note]));
            }
        }
    }
    align(&mut told.lines);
    if inbox_rows(answer).next().is_none() {
        told.lines
            .push(Phrase::new(0, Tone::Dim, vec![said("Nothing is pending.")]));
    } else {
        told.lines.push(Phrase::blank());
        let tail = "c lists the confirm and accept commands; watch never runs them.";
        told.lines.push(Phrase::new(0, Tone::Dim, vec![said(tail)]));
    }
    notes(&mut told, answer, &[]);
    told
}

/// `2 rules · 1 would block · 1 would remind`.
pub fn guard_counts(answer: &Reply) -> Phrase {
    let mut pieces = Vec::new();
    for (name, word) in [
        ("rules", "rules"),
        ("deny", "would block"),
        ("ask", "would ask first"),
        ("nudge", "would remind"),
        ("invalid", "invalid"),
    ] {
        match answer.count(name) {
            Some(n) if n > 0 || name == "rules" => pieces.push(said(format!("{n} {word}"))),
            _ => {}
        }
    }
    let tone = crate::guards::tone(answer);
    Phrase::new(0, tone, pieces)
}

pub fn guards(answer: &Reply) -> Told {
    let mut told = Told::default();
    told.lines.push(guard_counts(answer));
    told.lines.push(Phrase::blank());
    let id_width = answer
        .rows
        .iter()
        .map(|r| field(r, "id").len())
        .max()
        .unwrap_or(0);
    let outcome = |row: &Value| short(GUARD, field(row, "status"));
    let out_width = answer
        .rows
        .iter()
        .map(|r| outcome(r).text().len())
        .max()
        .unwrap_or(0);
    let mut rows = Vec::new();
    for row in &answer.rows {
        let id = format!("{:<w$}", field(row, "id"), w = id_width + 2);
        let mut pieces = vec![
            Piece::Quoted(id),
            said(format!("{:<out_width$}", outcome(row).text())),
        ];
        match requires(field(row, "text")) {
            Some(condition) => pieces.push(short(CONDITION, condition)),
            None => told.withheld += 1,
        }
        rows.push(Phrase::led(0, lines::tone(row), pieces, 2));
    }
    align(&mut rows);
    told.lines.extend(rows);
    let sentence = "Judged for the current run as if each rule applied right now. A guard advises; it does not enforce, and nothing was run.";
    notes(
        &mut told,
        answer,
        &[("judged for the current run ", sentence)],
    );
    if field(&answer.trailer, "reason") == "no-guards" {
        told.lines
            .push(Phrase::new(0, Tone::Dim, vec![said("No guard rules.")]));
    }
    told
}
