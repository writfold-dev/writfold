//! `find --eval [FILE]`: how well `find` answers a team's known phrases.
//! Each line of the file (default `.writfold/queries.jsonl`) is
//! `{"query": …, "expect": [IDs], "note": …}`; the query runs through the
//! same code as `find` and is a `hit@1`, a `hit@5` or a `miss`. Nothing is
//! written. An evaluation with no queries is not a pass.
use super::find::{self, Filter};
use crate::defs::{Field, Sink, jsonl};
use crate::emit::{Answer, More, Row};
use crate::error::{KbError, Result};
use crate::index::query;
use crate::session::Session;
use serde_json::Value;
use std::path::Path;

/// Where the queries are, relative to the KB root.
pub const DEFAULT: &str = ".writfold/queries.jsonl";
const KEYS: [&str; 3] = ["query", "expect", "note"];
/// Trailer names for the tier of each first expected hit.
const BY: [(&str, &str); 10] = [
    ("signature", "by-signature"),
    ("exact", "by-exact"),
    ("normalized", "by-normalized"),
    ("prefix", "by-prefix"),
    ("heading", "by-heading"),
    ("text", "by-text"),
    ("substring", "by-substring"),
    ("typo", "by-typo"),
    ("text-any", "by-text-any"),
    ("listed", "by-listed"),
];

/// One query line.
struct Query {
    line: u32,
    text: String,
    wanted: Vec<String>,
}

/// One query line, or nothing (its faults reported).
fn query(sink: &mut Sink<'_>, l: &jsonl::Line<'_>) -> Option<Query> {
    let mut ok = jsonl::unknown(sink, l, &KEYS, "not a query key");
    let text = match jsonl::text(sink, l, "query") {
        Field::Got(q) if !q.trim().is_empty() => Some(q),
        Field::Got(_) => {
            let why = "`query` must not be empty".into();
            sink.error("key.range", l.value("query"), why)
                .label("empty");
            None
        }
        Field::Absent => {
            jsonl::missing(
                sink,
                l,
                "query",
                "a query line",
                "\"query\": \"the words to find\"",
            );
            None
        }
        Field::Bad => None,
    };
    let wanted: Option<Vec<String>> = match l.map.get("expect") {
        None => {
            jsonl::missing(
                sink,
                l,
                "expect",
                "a query line",
                "\"expect\": [\"<record ID>\"]",
            );
            None
        }
        Some(Value::Array(items)) if items.is_empty() => {
            let why = "`expect` needs at least one record ID".into();
            sink.error("key.range", l.value("expect"), why)
                .label("empty");
            None
        }
        Some(Value::Array(items)) if items.iter().all(Value::is_string) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        Some(_) => {
            let why = "`expect` must be an array of record IDs".into();
            sink.error("key.type", l.value("expect"), why)
                .label("expected an array of strings");
            None
        }
    };
    ok &= jsonl::text(sink, l, "note") != Field::Bad;
    let line = u32::try_from(l.number).unwrap_or(u32::MAX);
    let (text, wanted) = (text?, wanted?);
    ok.then_some(Query { line, text, wanted })
}

/// Every query of the file, or every fault of it.
fn parse(text: &str, shown: &str) -> Result<Vec<Query>> {
    let mut sink = Sink::new(shown, text);
    let lines = jsonl::objects(&mut sink, text, "a query line");
    let queries: Vec<Query> = lines.iter().filter_map(|l| query(&mut sink, l)).collect();
    match sink.found.is_empty() {
        true => Ok(queries),
        false => Err(KbError::Definition(Box::new(sink.report()))),
    }
}

/// The faults of a query file's text, for `kb lint` (its `expect` IDs
/// need the index and are judged by `find --eval`).
pub fn lint(text: &str, shown: &str) -> Option<writfold_core::diag::Report> {
    match parse(text, shown) {
        Err(KbError::Definition(report)) => Some(*report),
        _ => None,
    }
}

/// Whether `id` names a record, or `doc:<path>` an indexed file.
fn known(session: &Session<'_>, id: &str) -> Result<bool> {
    if let Some(path) = id.strip_prefix("doc:") {
        return Ok(session.text(path).is_some());
    }
    Ok(query::record(&session.conn, id)?.is_some())
}

fn count(counts: &mut Vec<(&'static str, usize)>, name: &'static str) {
    match counts.iter_mut().find(|(n, _)| *n == name) {
        Some((_, c)) => *c += 1,
        None => counts.push((name, 1)),
    }
}

/// One query's row, and its status and tier.
fn judge(
    session: &Session<'_>,
    q: &Query,
    shown: &str,
) -> Result<(Row, &'static str, Option<&'static str>)> {
    let mut found = find::find(session, &q.text, &Filter::default(), false)?;
    // A suggestion is not a hit.
    found.rows.retain(|r| r.matched_on != Some("suggestion"));
    let at = found.rows.iter().position(|r| q.wanted.contains(&r.id));
    let mut unknown = false;
    for id in &q.wanted {
        unknown |= !known(session, id)?;
    }
    let (status, tier) = match at {
        _ if unknown => ("miss", None),
        Some(0) => ("hit@1", found.rows[0].matched_on),
        Some(i) if i < 5 => ("hit@5", found.rows[i].matched_on),
        _ => ("miss", None),
    };
    let text = match at {
        Some(i) => format!(
            "{} -> {} rank={} tier={}",
            q.text,
            found.rows[i].id,
            i + 1,
            found.rows[i].matched_on.unwrap_or("none")
        ),
        None => format!("{} -> none of {}", q.text, q.wanted.join(", ")),
    };
    let mut row = Row {
        kind: "query",
        id: format!("q{}", q.line),
        status: Some(status.to_owned()),
        text,
        path: Some(shown.to_owned()),
        line: Some(q.line),
        ..Row::default()
    };
    if unknown {
        row.flags.push("unknown-id".into());
    }
    if status == "miss" {
        let first = found.rows.first().map_or_else(
            || "nothing".to_owned(),
            |r| format!("{} by {}", r.id, r.matched_on.unwrap_or("none")),
        );
        row.more.push(More {
            label: "first",
            text: first,
            ..More::default()
        });
    }
    Ok((row, status, tier))
}

/// Evaluate the query file; `file` is absolute, else the default.
pub fn eval(session: &Session<'_>, file: Option<&Path>) -> Result<Answer> {
    let root = &session.ws.root;
    let path = file.map_or_else(|| root.join(DEFAULT), Path::to_path_buf);
    let shown = path
        .strip_prefix(root)
        .map_or_else(|_| path.display().to_string(), |p| p.display().to_string());
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(KbError::Refused(format!("cannot read {shown}: {e}"))),
    };
    let queries = parse(&text, &shown)?;
    let mut answer = Answer::default();
    if queries.is_empty() {
        answer.unknown = Some("no-queries");
        answer.notes.push(format!(
            "no queries in {shown}: an evaluation with no queries is not a pass"
        ));
        return Ok(answer);
    }
    let mut counts: Vec<(&'static str, usize)> = vec![
        ("queries", queries.len()),
        ("hit1", 0),
        ("hit5", 0),
        ("miss", 0),
    ];
    for q in &queries {
        let (row, status, tier) = judge(session, q, &shown)?;
        count(
            &mut counts,
            match status {
                "hit@1" => "hit1",
                "hit@5" => "hit5",
                _ => "miss",
            },
        );
        if let Some(by) = tier.and_then(|t| BY.iter().find(|(name, _)| *name == t)) {
            count(&mut counts, by.1);
        }
        answer.rows.push(row);
    }
    answer.counts = counts;
    answer.notes.push(format!(
        "queries from {shown}; each ran as find does; nothing was written"
    ));
    Ok(answer)
}
