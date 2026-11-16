//! `guard log [--rule ID] [--since AGE]`: what this machine's guard log
//! says about each pre-tool-use rule. Counts are over the entries for the
//! rule file's current bytes, so editing a rule restarts its count; older
//! entries are reported as `earlier`, never added in. An audit rule that
//! would have refused, or never matched, is not a clean answer (exit 2).
//! Writfold never promotes a rule.
use super::log;
use super::rules::{self, Event, Rule};
use crate::Request;
use crate::emit::{Answer, Frame, More, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use serde_json::Value;

/// The newest refusals shown per rule without `--rule`.
const SHOWN: usize = 5;

fn text(entry: &Value, key: &str) -> String {
    entry[key].as_str().unwrap_or("").to_owned()
}

/// One refusal as a continuation line.
fn refusal(entry: &Value, label: &'static str) -> More {
    let owes: Vec<&str> = entry["owes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let repo = entry["repo"].as_str().unwrap_or("an unknown repository");
    More {
        label,
        text: format!(
            "{} {} in {repo}; owes: {} (session {})",
            text(entry, "at"),
            text(entry, "command"),
            owes.join("; "),
            text(entry, "session")
        ),
        ..More::default()
    }
}

/// The row for one rule over its entries; whether it is not clean.
fn rule_row(rule: &Rule, current: &str, mine: &[&Value], all: bool) -> (Row, bool) {
    let counted: Vec<&Value> = mine
        .iter()
        .copied()
        .filter(|e| e["rule_digest"] == current)
        .collect();
    let earlier = mine.len() - counted.len();
    let decided = |d: &str| counted.iter().filter(|e| e["decided"] == d).count();
    let (deny, ask, pass) = (decided("deny"), decided("ask"), decided("pass"));
    let since = counted
        .first()
        .map_or_else(|| "never".to_owned(), |e| text(e, "at"));
    let audit = rule.mode.name() == "audit";
    let mut row = Row {
        kind: "guard-log",
        id: rule.id.clone(),
        status: Some(rule.mode.name().into()),
        path: Some(rule.path.clone()),
        line: Some(1),
        ..Row::default()
    };
    let waived = counted
        .iter()
        .filter(|e| e["decided"] == "pass" && e["waived"].as_array().is_some_and(|w| !w.is_empty()))
        .count();
    row.text = match audit {
        true => format!(
            "matched={} would-deny={deny} would-ask={ask} pass={pass} since {since}",
            counted.len()
        ),
        false => format!("deny={deny} ask={ask} passed-on-waiver={waived} since {since}"),
    };
    let refused: Vec<&&Value> = counted
        .iter()
        .rev()
        .filter(|e| e["decided"] != "pass")
        .collect();
    for e in refused.iter().take(if all { usize::MAX } else { SHOWN }) {
        let label = match (audit, e["decided"].as_str()) {
            (true, Some("ask")) => "would-ask",
            (true, _) => "would-deny",
            (false, Some("ask")) => "ask",
            (false, _) => "deny",
        };
        row.more.push(refusal(e, label));
    }
    if earlier > 0 {
        row.more.push(More {
            label: "earlier",
            text: format!(
                "{earlier} entries are for an earlier version of the rule file and are not counted"
            ),
            ..More::default()
        });
    }
    let unmatched = audit && counted.is_empty();
    if audit {
        row.flags.push("audit".into());
    }
    for (flag, n) in [("would-deny", deny), ("would-ask", ask)] {
        if audit && n > 0 {
            row.flags.push(flag.into());
        }
    }
    if unmatched {
        row.flags.push("unmatched".into());
    } else if audit && deny + ask == 0 {
        row.more.push(More {
            label: "next",
            text: format!("remove mode = \"audit\" from {} to enforce it", rule.path),
            ..More::default()
        });
    }
    (row, audit && (unmatched || deny + ask > 0))
}

/// Rows for rule IDs in the log whose file is gone.
fn removed(answer: &mut Answer, entries: &[&Value], found: &[Rule]) {
    let mut ids: Vec<String> = entries
        .iter()
        .map(|e| text(e, "rule"))
        .filter(|id| !found.iter().any(|r| &r.id == id))
        .collect();
    ids.sort();
    ids.dedup();
    for id in ids {
        let n = entries.iter().filter(|e| e["rule"] == id.as_str()).count();
        answer.rows.push(Row {
            kind: "guard-log",
            id,
            status: Some("removed".into()),
            text: format!("{n} entries; the rule file is gone"),
            ..Row::default()
        });
    }
}

/// The trailer counts.
fn counts(entries: &[&Value], rules: usize, unparsed: usize) -> Vec<(&'static str, usize)> {
    let total = |d: &str, mode: &str| {
        entries
            .iter()
            .filter(|e| e["decided"] == d && e["mode"] == mode)
            .count()
    };
    vec![
        ("rules", rules),
        ("entries", entries.len()),
        ("would-deny", total("deny", "audit")),
        ("would-ask", total("ask", "audit")),
        ("deny", total("deny", "enforce")),
        ("ask", total("ask", "enforce")),
        ("unparsed", unparsed),
    ]
}

/// Answer `guard log`.
pub fn execute(
    ws: &Workspace,
    request: &Request,
    (only, since): (Option<&str>, Option<i64>),
    now: i64,
) -> Result<Output> {
    let path = log::path(ws)
        .ok_or_else(|| KbError::Refused("the guard log needs a KB under Git".into()))?;
    let tool = GitTool::locate()?;
    let session = Session::open(ws, Some(&tool), false, now)?;
    let (entries, unparsed) = log::read(&path);
    let recent = |e: &&Value| match (since, e["at"].as_str().and_then(crate::time::parse)) {
        (None, _) => true,
        (Some(age), Some(at)) => now - at <= age,
        (Some(_), None) => false,
    };
    let entries: Vec<&Value> = entries
        .iter()
        .filter(recent)
        .filter(|e| only.is_none_or(|id| e["rule"] == id))
        .collect();
    let (mut found, _) = rules::all(&ws.root, &request.cwd);
    found.retain(|r| r.event == Event::PreToolUse && only.is_none_or(|id| r.id == id));
    found.sort_by(|a, b| a.id.cmp(&b.id));
    let mut answer = Answer::default();
    let mut unclean = false;
    for rule in &found {
        let mine: Vec<&Value> = entries
            .iter()
            .copied()
            .filter(|e| e["rule"] == rule.id.as_str())
            .collect();
        let (row, bad) = rule_row(
            rule,
            &log::digest(&log::file(ws, rule)),
            &mine,
            only.is_some(),
        );
        unclean |= bad;
        answer.rows.push(row);
    }
    removed(&mut answer, &entries, &found);
    answer.counts = counts(&entries, found.len(), unparsed);
    answer.notes.push(format!(
        "from the local log {}: this machine only, never committed; a guard, not enforcement",
        path.display()
    ));
    if found.is_empty() && answer.rows.is_empty() {
        answer.unknown = Some("no-guards");
    }
    let frame = Frame {
        verb: "guard log".into(),
        args: [only.map(str::to_owned), since.map(|s| s.to_string())]
            .into_iter()
            .flatten()
            .collect(),
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    let mut output = session.emit(&answer, &frame)?;
    output.exit = if unclean { 2 } else { 0 };
    Ok(output)
}
