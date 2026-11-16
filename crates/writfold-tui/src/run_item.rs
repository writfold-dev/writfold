//! A run-list entry: five lines in the verb's order when detailed, two
//! when calm (title; state, project and plan position).
use crate::app::Detail;
use crate::lines::{Line, Tone, continuation, line, more, suffix, tone};
use crate::source::{field, flags};
use serde_json::Value;

pub fn run_item(row: &Value, detail: Detail) -> Vec<Line> {
    if detail == Detail::Exact {
        return full(row);
    }
    let role = if flags(row).contains(&"current") {
        Tone::Current
    } else {
        tone(row)
    };
    let mut out = vec![line(field(row, "text"), role)];
    let mut parts = Vec::new();
    let state = field(row, "status");
    if state != "open" && !state.is_empty() {
        parts.push(state);
    }
    for m in more(row) {
        let text = field(m, "text");
        match field(m, "label") {
            "field" => parts.extend(
                text.split(' ')
                    .find_map(|part| part.strip_prefix("project=")),
            ),
            "plan" => parts.extend(plan_words(text)),
            _ => {}
        }
    }
    if !parts.is_empty() {
        out.push(line(parts.join("  "), Tone::Dim));
    }
    out
}

/// The plan text up to its first `key=value` token, verbatim.
fn plan_words(text: &str) -> Option<&str> {
    let end: usize = text
        .split(' ')
        .take_while(|word| !word.contains('='))
        .map(|word| word.len() + 1)
        .sum();
    let words = text[..end.min(text.len())].trim_end();
    (!words.is_empty()).then_some(words)
}

/// Run-list fields stay in the verb's order, including its plan position.
fn full(row: &Value) -> Vec<Line> {
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
    use super::plan_words;

    #[test]
    fn plan_words_stop_before_the_first_key_value_token() {
        let text = "step 7/11 done=5 skipped=1 current=a/b";
        assert_eq!(plan_words(text), Some("step 7/11"));
        assert_eq!(plan_words("complete steps=2 done=2"), Some("complete"));
        assert_eq!(plan_words("none"), Some("none"));
        assert_eq!(plan_words("steps=2"), None);
    }
}
