//! `gate cover <gate> --from FILE|- --by WHO`: several cover rows of one
//! checklist gate in one write plan. The input is JSON lines, one object
//! per row:
//!
//! ```text
//! {"item":"errors","outcome":"supported","refs":["gate:unit"],"note":"…"}
//! {"item":"perf","outcome":"not-applicable","reason":"no hot path"}
//! ```
//!
//! `item` and `outcome` are required; `refs`, `reason` and `note` are
//! optional; any other key, an item named twice, and any refused row
//! refuse the batch (exit 3), each fault at its line, nothing written.
use super::write::{self, Cover};
use crate::Request;
use crate::defs::{Field, Sink, jsonl};
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::workspace::{GitTool, Workspace};
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use writfold_core::checklist::Outcome;
use writfold_core::diag::suggest::{did_you_mean, suggest};

/// The most input a batch reads.
const INPUT_LIMIT: u64 = 4 << 20;

/// Where the rows are read from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rows {
    /// A file, relative to the working directory.
    File(PathBuf),
    /// Standard input (`--from -`).
    Stdin,
}

/// Several cover rows of one gate to record.
#[derive(Clone, Debug)]
pub struct Batch {
    pub gate: String,
    pub from: Rows,
    pub by: String,
    pub run: Option<String>,
    pub repo: Option<PathBuf>,
}

/// One row as read: its line number and the cover it asks for.
pub type Line = (usize, Cover);

fn refused(line: usize, message: impl std::fmt::Display) -> KbError {
    KbError::Refused(format!("--from line {line}: {message}"))
}

const KEYS: [&str; 5] = ["item", "outcome", "refs", "reason", "note"];
const OUTCOMES: [&str; 4] = ["supported", "unsupported", "not-applicable", "none-found"];

/// `refs`: absent, or a list of strings.
fn references(sink: &mut Sink<'_>, l: &jsonl::Line<'_>) -> Option<Vec<String>> {
    match l.map.get("refs") {
        None | Some(Value::Null) => Some(Vec::new()),
        Some(Value::Array(items)) if items.iter().all(Value::is_string) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        Some(Value::Array(_)) => {
            let why = "`refs` holds a value that is not a string".into();
            sink.error("key.type", l.value("refs"), why)
                .label("expected strings");
            None
        }
        Some(_) => {
            let why = "`refs` is not a list".into();
            let d = sink.error("key.type", l.value("refs"), why);
            d.label("expected an array");
            d.help("write a list: \"refs\": [\"gate:unit\"]".into());
            None
        }
    }
}

fn outcome(sink: &mut Sink<'_>, l: &jsonl::Line<'_>) -> Option<Outcome> {
    let written = match jsonl::text(sink, l, "outcome") {
        Field::Bad => return None,
        other => other.got().unwrap_or_default(),
    };
    if let Some(o) = Outcome::parse(&written) {
        return Some(o);
    }
    let found = suggest(&written, &OUTCOMES);
    let why =
        format!("outcome {written:?} is not supported, unsupported, not-applicable or none-found");
    let d = sink.error("value.unknown", l.value("outcome").or(Some(l.span())), why);
    d.label("not an outcome");
    d.help = did_you_mean(&found, '"');
    None
}

/// One JSON line into a cover of `b`'s gate; its faults reported.
fn row(sink: &mut Sink<'_>, b: &Batch, l: &jsonl::Line<'_>) -> Option<Cover> {
    let known = jsonl::unknown(sink, l, &KEYS, "not a row key");
    let item = match jsonl::text(sink, l, "item") {
        Field::Got(i) if !i.is_empty() => Some(i),
        Field::Got(_) | Field::Absent => {
            jsonl::missing(sink, l, "item", "a cover row", "\"item\": \"<item ID>\"");
            None
        }
        Field::Bad => None,
    };
    let outcome = outcome(sink, l);
    let references = references(sink, l);
    let reason = jsonl::optional(jsonl::text(sink, l, "reason"));
    let note = jsonl::optional(jsonl::text(sink, l, "note"));
    known.then_some(())?;
    Some(Cover {
        gate: b.gate.clone(),
        item: item?,
        outcome: outcome?,
        references: references?,
        reason: reason?,
        note: note?,
        by: b.by.clone(),
        run: b.run.clone(),
        repo: b.repo.clone(),
    })
}

/// Every row of `input`, with its line number; every malformed line, an
/// item named twice and input with no rows are refused.
pub fn parse(b: &Batch, input: &str) -> Result<Vec<Line>> {
    let shown = match &b.from {
        Rows::File(path) => format!("--from {}", path.display()),
        Rows::Stdin => "--from -".to_owned(),
    };
    let mut sink = Sink::new(&shown, input);
    let mut out: Vec<Line> = Vec::new();
    for l in jsonl::objects(&mut sink, input, "a cover row") {
        let Some(cover) = row(&mut sink, b, &l) else {
            continue;
        };
        if let Some((first, _)) = out.iter().find(|(_, c)| c.item == cover.item) {
            let why = format!(
                "item {} is already covered on line {first}: one row per item per batch",
                cover.item
            );
            sink.error("value.twice", l.value("item"), why)
                .label("covered again");
            continue;
        }
        out.push((l.number, cover));
    }
    if !sink.found.is_empty() {
        return Err(KbError::Definition(Box::new(sink.report())));
    }
    if out.is_empty() {
        return Err(KbError::Refused("--from holds no rows".into()));
    }
    Ok(out)
}

/// The input text: the file (relative to `cwd`) or `stdin`, at most
/// [`INPUT_LIMIT`] bytes.
pub fn input(from: &Rows, cwd: &std::path::Path, stdin: impl Read) -> Result<String> {
    let mut text = String::new();
    let read = match from {
        Rows::File(path) => std::fs::File::open(cwd.join(path))?
            .take(INPUT_LIMIT + 1)
            .read_to_string(&mut text)?,
        Rows::Stdin => stdin.take(INPUT_LIMIT + 1).read_to_string(&mut text)?,
    };
    if read as u64 > INPUT_LIMIT {
        return Err(KbError::Refused(format!(
            "--from is larger than {INPUT_LIMIT} bytes"
        )));
    }
    Ok(text)
}

pub fn record(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    b: &Batch,
    now: i64,
) -> Result<Output> {
    let text = input(&b.from, &request.cwd, std::io::stdin().lock())?;
    let lines = parse(b, &text)?;
    let (numbers, covers): (Vec<usize>, Vec<Cover>) = lines.into_iter().unzip();
    let blame = |index: usize, e: KbError| {
        let line = numbers.get(index).copied().unwrap_or(0);
        let item = covers.get(index).map_or("", |c| c.item.as_str());
        match e {
            KbError::Usage(m) | KbError::Refused(m) => refused(line, format!("item {item}: {m}")),
            other => other,
        }
    };
    write::record_all(ws, tool, request, &covers, (now, &blame))
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
