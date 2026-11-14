//! JSON rows shared by the terminal panes and the plain frame.
use crate::source::{Reply, field, flags};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Dim,
    Head,
    Good,
    Warn,
    Bad,
    Vouched,
    Current,
    Done,
    Skipped,
    Later,
    Info,
    Key,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
}

pub fn line(text: impl Into<String>, tone: Tone) -> Line {
    Line {
        text: text.into(),
        tone,
    }
}

const BAD: &[&str] = &[
    "failed",
    "stale",
    "invalid",
    "changed",
    "gone",
    "expired",
    "moved",
    "regression",
    "disagree",
    "unsupported",
    "out-of-order",
    "two-plans",
    "repeated",
    "deny",
];
const WARN: &[&str] = &[
    "open",
    "unknown",
    "unverifiable",
    "draft",
    "past-ttl",
    "template-moved",
    "undefined",
    "proposal",
    "ask",
    "nudge",
    "skipped",
];

/// Status and flags are authoritative; bracketed text is never parsed.
pub fn tone(row: &Value) -> Tone {
    let flagged = flags(row);
    let has = |words: &[&str]| {
        flagged
            .iter()
            .any(|f| words.iter().any(|w| f.starts_with(w)))
    };
    let status = field(row, "status");
    if has(BAD) || status == "would-deny" {
        return Tone::Bad;
    }
    if has(WARN) || matches!(status, "would-ask" | "would-nudge" | "proposed" | "draft") {
        return Tone::Warn;
    }
    match status {
        "satisfied" | "supported" | "verified" | "would-pass" | "clean" => Tone::Good,
        "attested" | "waived" | "not-applicable" => Tone::Vouched,
        "later" => Tone::Later,
        "closed" => Tone::Dim,
        _ => Tone::Plain,
    }
}

pub fn suffix(row: &Value, positions: bool) -> String {
    let mut out = String::new();
    let path = field(row, "path");
    if positions && !path.is_empty() {
        out.push_str(&format!(" @{path}"));
        if let Some(n) = row.get("line").and_then(Value::as_u64) {
            out.push_str(&format!(":{n}"));
        }
    }
    for flag in flags(row) {
        out.push_str(&format!(" !{flag}"));
    }
    out
}

pub fn more(row: &Value) -> impl Iterator<Item = &Value> {
    row.get("more")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

pub fn head(row: &Value, kind: bool, positions: bool) -> String {
    let mut out = if kind {
        format!("{} {}", field(row, "kind"), field(row, "id"))
    } else {
        field(row, "id").to_owned()
    };
    let status = field(row, "status");
    if !status.is_empty() {
        out.push_str(&format!(" [{status}]"));
    }
    let text = field(row, "text");
    if !text.is_empty() {
        out.push_str(&format!(" {text}"));
    }
    out.push_str(&suffix(row, positions));
    out
}

pub fn continuation(value: &Value, parent: Tone, progress: bool, positions: bool) -> Line {
    let label = field(value, "label");
    let (indent, separator) = match (progress, label) {
        (true, "gate") => ("     ", " "),
        (true, "item") => ("       ", " "),
        (true, _) => ("     ", ": "),
        _ => ("  ", ": "),
    };
    let own = if matches!(label, "next" | "confirm" | "accept" | "read") {
        Tone::Info
    } else {
        tone(value)
    };
    let role = match own {
        Tone::Plain if parent == Tone::Bad => Tone::Bad,
        Tone::Plain => Tone::Dim,
        other => other,
    };
    line(
        format!(
            "{indent}{label}{separator}{}{}",
            field(value, "text"),
            suffix(value, positions)
        ),
        role,
    )
}

pub fn row(row: &Value) -> Vec<Line> {
    let role = tone(row);
    let mut out = vec![line(head(row, true, true), role)];
    out.extend(more(row).map(|m| continuation(m, role, false, true)));
    out
}

pub fn notes(answer: &Reply) -> impl Iterator<Item = Line> + '_ {
    answer
        .notes
        .iter()
        .map(|n| line(format!("# note: {n}"), Tone::Dim))
}

pub fn reply(answer: &Reply) -> Vec<Line> {
    if let Some(error) = &answer.error {
        return vec![line(
            format!("refused (exit {}): {error}", answer.exit),
            Tone::Bad,
        )];
    }
    let mut out: Vec<_> = answer.rows.iter().flat_map(row).collect();
    out.extend(notes(answer));
    if out.is_empty() {
        out.push(line("(no rows)", Tone::Dim));
    }
    out
}

/// Run-list fields stay in the verb's order, including its plan position.
pub fn run_item(row: &Value) -> Vec<Line> {
    let state = field(row, "status");
    let role = if flags(row).contains(&"current") {
        Tone::Current
    } else {
        tone(row)
    };
    let mut out = vec![
        line(
            format!("[{state}] {}{}", field(row, "id"), suffix(row, false)),
            role,
        ),
        line(format!("  {}", field(row, "text")), tone(row)),
    ];
    for m in more(row) {
        let text = field(m, "text");
        match field(m, "label") {
            "field" if !text.is_empty() => out.push(line(format!("  {text}"), Tone::Dim)),
            "checkpoint" => out.push(line(format!("  last: {text}"), Tone::Dim)),
            "plan" => out.push(continuation(m, role, false, false)),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Tone, continuation};

    #[test]
    fn continuation_tones_use_status_not_bracketed_text() {
        for label in ["gate", "item"] {
            let tone = |status: &str, text: &str| {
                let value = serde_json::json!({
                    "label": label, "status": status, "text": text, "flags": []
                });
                continuation(&value, Tone::Plain, true, false).tone
            };
            let stale = tone("stale", "unit [satisfied]");
            assert_eq!(stale, tone("stale", "wording without brackets"));
            assert_ne!(stale, tone("satisfied", "unit [stale]"));
        }
    }
}
