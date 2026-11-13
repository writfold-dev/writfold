//! `gate cover <gate> --from FILE|- --by WHO`: several cover rows of one
//! checklist gate in one write plan. The input is JSON lines, one object
//! per row:
//!
//! ```text
//! {"item":"errors","outcome":"supported","refs":["gate:unit"],"note":"…"}
//! {"item":"perf","outcome":"not-applicable","reason":"no hot path"}
//! ```
//!
//! `item` and `outcome` are required; `refs` (a list of references),
//! `reason` and `note` are optional, and any other key is refused. Blank
//! lines are skipped. Each row is checked exactly as the single-item form
//! checks its arguments ([`super::write`]), on one subject snapshot; an
//! item named twice is refused (one judgement per item per batch), and
//! any refused row refuses the batch (exit 3) naming its line, with
//! nothing written.
use super::write::{self, Cover};
use crate::Request;
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::workspace::{GitTool, Workspace};
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use writfold_core::checklist::Outcome;

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

fn text(map: &serde_json::Map<String, Value>, key: &str, line: usize) -> Result<Option<String>> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(refused(line, format!("{key} is not a string"))),
    }
}

fn references(map: &serde_json::Map<String, Value>, line: usize) -> Result<Vec<String>> {
    match map.get("refs") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|r| {
                r.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| refused(line, "refs holds a value that is not a string"))
            })
            .collect(),
        Some(_) => Err(refused(line, "refs is not a list")),
    }
}

/// One JSON line into a cover of `b`'s gate.
fn row(b: &Batch, line: usize, source: &str) -> Result<Cover> {
    let value: Value =
        serde_json::from_str(source).map_err(|e| refused(line, format!("not JSON ({e})")))?;
    let map = value
        .as_object()
        .ok_or_else(|| refused(line, "not a JSON object"))?;
    if let Some(key) = map
        .keys()
        .find(|k| !["item", "outcome", "refs", "reason", "note"].contains(&k.as_str()))
    {
        return Err(refused(
            line,
            format!("unknown key {key:?} (item, outcome, refs, reason, note)"),
        ));
    }
    let item = text(map, "item", line)?
        .filter(|i| !i.is_empty())
        .ok_or_else(|| refused(line, "item is missing"))?;
    let outcome = text(map, "outcome", line)?.unwrap_or_default();
    Ok(Cover {
        gate: b.gate.clone(),
        item,
        outcome: Outcome::parse(&outcome).ok_or_else(|| {
            refused(
                line,
                "outcome is not supported, unsupported or not-applicable",
            )
        })?,
        references: references(map, line)?,
        reason: text(map, "reason", line)?,
        note: text(map, "note", line)?,
        by: b.by.clone(),
        run: b.run.clone(),
        repo: b.repo.clone(),
    })
}

/// Every row of `input`, with its line number; refuses a malformed line,
/// an item named twice, and input with no rows.
pub fn parse(b: &Batch, input: &str) -> Result<Vec<Line>> {
    let mut out: Vec<Line> = Vec::new();
    for (index, source) in input.lines().enumerate() {
        let line = index + 1;
        if source.trim().is_empty() {
            continue;
        }
        let cover = row(b, line, source)?;
        if let Some((first, _)) = out.iter().find(|(_, c)| c.item == cover.item) {
            return Err(refused(
                line,
                format!(
                    "item {} is already covered on line {first}: one row per item per batch",
                    cover.item
                ),
            ));
        }
        out.push((line, cover));
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
mod tests {
    use super::*;

    fn batch() -> Batch {
        Batch {
            gate: "review".into(),
            from: Rows::Stdin,
            by: "agent".into(),
            run: Some("run-1".into()),
            repo: None,
        }
    }

    fn why(input: &str) -> String {
        match parse(&batch(), input) {
            Ok(rows) => format!("not refused: {} rows", rows.len()),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn rows_parse_with_their_line_numbers() -> Result<()> {
        let input = "{\"item\":\"errors\",\"outcome\":\"supported\",\"refs\":[\"gate:unit\"],\"note\":\"n\"}\n\n{\"item\":\"perf\",\"outcome\":\"not-applicable\",\"reason\":\"none\"}\n";
        let rows = parse(&batch(), input)?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, 1);
        assert_eq!(rows[0].1.references, vec!["gate:unit".to_owned()]);
        assert_eq!(rows[0].1.note.as_deref(), Some("n"));
        assert_eq!(rows[1].0, 3);
        assert_eq!(rows[1].1.outcome, Outcome::NotApplicable);
        assert_eq!(rows[1].1.reason.as_deref(), Some("none"));
        assert!(
            rows.iter()
                .all(|(_, c)| c.gate == "review" && c.by == "agent")
        );
        Ok(())
    }

    #[test]
    fn malformed_rows_are_refused_naming_the_line() {
        let ok = "{\"item\":\"errors\",\"outcome\":\"supported\",\"refs\":[\"gate:unit\"]}\n";
        for (bad, want) in [
            ("not json", "line 2: not JSON"),
            ("[1]", "line 2: not a JSON object"),
            ("{\"outcome\":\"supported\"}", "line 2: item is missing"),
            (
                "{\"item\":\"x\",\"outcome\":\"met\"}",
                "line 2: outcome is not",
            ),
            (
                "{\"item\":\"x\",\"outcome\":\"supported\",\"refs\":\"gate:unit\"}",
                "refs is not a list",
            ),
            (
                "{\"item\":\"x\",\"outcome\":\"supported\",\"refs\":[1]}",
                "not a string",
            ),
            (
                "{\"item\":\"x\",\"outcome\":\"supported\",\"by\":\"me\"}",
                "unknown key \"by\"",
            ),
            (
                "{\"item\":\"x\",\"outcome\":\"unsupported\",\"reason\":3}",
                "reason is not a string",
            ),
            (
                "{\"item\":\"errors\",\"outcome\":\"unsupported\",\"reason\":\"r\"}",
                "line 2: item errors is already covered on line 1",
            ),
        ] {
            let got = why(&format!("{ok}{bad}\n"));
            assert!(got.starts_with("refused: --from"), "{bad}: {got}");
            assert!(got.contains(want), "{bad}: {got}");
        }
        assert!(why("").contains("holds no rows"));
        assert!(why("\n  \n").contains("holds no rows"));
    }

    #[test]
    fn input_reads_stdin_or_a_file_and_bounds_it() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("writfold-batch-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("rows.jsonl"), "file rows\n")?;
        let file = Rows::File("rows.jsonl".into());
        assert_eq!(input(&file, &dir, std::io::empty())?, "file rows\n");
        assert_eq!(
            input(&Rows::Stdin, &dir, &b"piped rows\n"[..])?,
            "piped rows\n"
        );
        let big = vec![b'x'; (INPUT_LIMIT + 1) as usize];
        let e = input(&Rows::Stdin, &dir, &big[..])
            .err()
            .map(|e| e.to_string());
        assert!(e.is_some_and(|e| e.contains("larger than")));
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
