//! Definitions that do not parse, in plain words: the list entry (the
//! invalid mark, the file's name and `line 6: unknown key …`), the count
//! (`1 definition has errors`) and the detail block with the source line
//! drawn by the diagnostics renderer. No code names or brackets in the
//! main text; the code sits dim at the end for someone who searches it.
use crate::cells::{self, Glyphs};
use crate::lines::Tone;
use crate::say::{Phrase, Piece, said};
use crate::source::{Reply, field};
use serde_json::Value;
use writfold_kb::diag::LineIndex;
use writfold_kb::diag::snippet::draw;

/// The kinds of row that stand for a definition that does not parse.
const KINDS: [&str; 2] = ["gate-definition", "guard-rule"];

fn plain(text: &str) -> String {
    text.replace('`', "")
}

/// `Unknown key x.` from `unknown key `x``.
fn sentence(text: &str) -> String {
    let text = plain(text);
    let mut chars = text.chars();
    let first = chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_default();
    let body = format!("{first}{}", chars.as_str());
    match body.ends_with(['.', '?']) {
        true => body,
        false => format!("{body}."),
    }
}

fn invalid(row: &Value) -> bool {
    KINDS.contains(&field(row, "kind")) && field(row, "status") == "invalid"
}

/// The diagnostics of the file `row` shows.
fn of<'a>(answer: &'a Reply, row: &Value) -> Vec<&'a Value> {
    let file = field(row, "path");
    answer
        .diagnostics
        .iter()
        .filter(|d| field(d, "file") == file)
        .collect()
}

/// `1 definition has errors`, when any does.
pub fn count(answer: &Reply) -> Option<Piece> {
    let n = answer.rows.iter().filter(|r| invalid(r)).count();
    match n {
        0 => None,
        1 => Some(said("1 definition has errors")),
        n => Some(said(format!("{n} definitions have errors"))),
    }
}

/// One list entry per definition that does not parse.
pub fn entries(answer: &Reply, glyphs: Glyphs) -> Vec<Phrase> {
    let mut out = Vec::new();
    for row in answer.rows.iter().filter(|r| invalid(r)) {
        let path = field(row, "path");
        let name = path.rsplit('/').next().unwrap_or(path);
        let at = row
            .get("line")
            .and_then(Value::as_u64)
            .map_or_else(String::new, |l| format!("line {l}: "));
        let pieces = vec![
            Piece::Mark(cells::chip("invalid", glyphs)),
            Piece::Quoted(name.to_owned()),
            said(format!("{at}{}", plain(field(row, "text")))),
        ];
        out.push(Phrase::led(0, Tone::Bad, pieces, 2));
    }
    out
}

fn line(indent: u16, tone: Tone, text: impl Into<String>) -> Phrase {
    Phrase::new(indent, tone, vec![said(text)])
}

/// The source line and its caret, from the text the verb judged.
fn snippet(answer: &Reply, d: &Value, out: &mut Vec<Phrase>) {
    let (Some(l), Some(c), Some(len)) = (
        d.get("line").and_then(Value::as_u64),
        d.get("col").and_then(Value::as_u64),
        d.get("len").and_then(Value::as_u64),
    ) else {
        return;
    };
    let Some((_, text)) = answer.sources.iter().find(|(f, _)| f == field(d, "file")) else {
        return;
    };
    let n = |v: u64| u32::try_from(v).unwrap_or(0);
    let source = LineIndex::new(text).line(text, n(l));
    let drawn = draw(source, n(c), n(len), 72, crate::measure::cell);
    let digits = u16::try_from(l.to_string().len()).unwrap_or(1);
    out.push(Phrase::blank());
    out.push(line(2, Tone::Plain, format!("{l} | {}", drawn.text)));
    let caret = format!(
        "| {}{}",
        " ".repeat(drawn.caret_at),
        "^".repeat(drawn.carets)
    );
    out.push(line(3 + digits, Tone::Bad, caret));
}

/// The detail of each definition that does not parse: what it means, the
/// location, the source line, the fault and its fix, how many more.
pub fn blocks(answer: &Reply) -> Vec<Phrase> {
    let mut out = Vec::new();
    for row in answer.rows.iter().filter(|r| invalid(r)) {
        let all = of(answer, row);
        let Some(d) = all.first() else { continue };
        let first = match field(row, "kind") {
            "guard-rule" => "This guard rule cannot be read, so it is not guarding.",
            _ => "This gate definition cannot be read, so the gate cannot be run or met.",
        };
        out.push(Phrase::blank());
        out.push(line(0, Tone::Bad, first));
        out.push(Phrase::blank());
        let place = match (
            d.get("line").and_then(Value::as_u64),
            d.get("col").and_then(Value::as_u64),
        ) {
            (Some(l), Some(c)) => format!("{}, line {l}, column {c}", field(d, "file")),
            _ => field(d, "file").to_owned(),
        };
        out.push(line(0, Tone::Plain, place));
        snippet(answer, d, &mut out);
        out.push(Phrase::blank());
        let help = d.get("help").and_then(Value::as_str).map(sentence);
        let said_line = [Some(sentence(field(d, "message"))), help]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        out.push(line(0, Tone::Plain, said_line));
        let more = all.len() - 1;
        let tail = match more {
            0 => String::new(),
            1 => "1 more error in this file.".to_owned(),
            n => format!("{n} more errors in this file."),
        };
        let mut pieces = Vec::new();
        if !tail.is_empty() {
            pieces.push(said(tail));
        }
        pieces.push(said(field(d, "code").to_owned()));
        out.push(Phrase::new(0, Tone::Dim, pieces));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reply() -> Reply {
        Reply {
            rows: vec![
                json!({"kind": "gate-definition", "id": ".writfold/gates/unit.toml", "status": "invalid",
                "text": "unknown key `timout_seconds` in a gate definition", "path": ".writfold/gates/unit.toml", "line": 6}),
            ],
            diagnostics: vec![
                json!({"severity": "error", "code": "key.unknown", "file": ".writfold/gates/unit.toml",
                "line": 6, "col": 1, "len": 14, "message": "unknown key `timout_seconds` in a gate definition",
                "label": "not a gate key", "help": "did you mean `timeout_seconds`?", "notes": []}),
            ],
            sources: vec![(
                ".writfold/gates/unit.toml".into(),
                "a = 1\nb = 2\nc = 3\nd = 4\ne = 5\ntimout_seconds = 600\n".into(),
            )],
            ..Reply::default()
        }
    }

    fn text(p: &Phrase) -> String {
        p.pieces
            .iter()
            .map(Piece::text)
            .collect::<Vec<_>>()
            .join(" · ")
    }

    #[test]
    fn an_invalid_definition_reads_in_plain_words_with_its_line() {
        let r = reply();
        let entry: Vec<String> = entries(&r, Glyphs::Ascii).iter().map(text).collect();
        assert_eq!(
            entry,
            ["# · unit.toml · line 6: unknown key timout_seconds in a gate definition"]
        );
        assert_eq!(
            count(&r).map(|p| p.text().to_owned()).as_deref(),
            Some("1 definition has errors")
        );
        let block: Vec<String> = blocks(&r).iter().map(text).collect();
        assert!(
            block.contains(
                &"This gate definition cannot be read, so the gate cannot be run or met."
                    .to_owned()
            ),
            "{block:?}"
        );
        assert!(
            block.contains(&".writfold/gates/unit.toml, line 6, column 1".to_owned()),
            "{block:?}"
        );
        assert!(
            block.contains(&"6 | timout_seconds = 600".to_owned()),
            "{block:?}"
        );
        assert!(block.contains(&"| ^^^^^^^^^^^^^^".to_owned()), "{block:?}");
        assert!(
            block.contains(
                &"Unknown key timout_seconds in a gate definition. Did you mean timeout_seconds?"
                    .to_owned()
            ),
            "{block:?}"
        );
        assert!(block.iter().all(|l| !l.contains('`')), "{block:?}");
    }
}
