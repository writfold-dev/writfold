//! `timeline <id>`: the target's own dates, then the investigations, ticket
//! records and decisions linked to it (either direction, any origin), and
//! the run checkpoint entries that mention it, in date order. Commits
//! touching it are M4.
use super::{BOUNDARY, ambiguous, record_row, unknown};
use crate::emit::{Answer, Row};
use crate::error::Result;
use crate::index::query::{self, Rec, Resolved};
use crate::parse::{field_line, json::line_of};
use crate::runs;
use crate::session::Session;
use std::collections::BTreeSet;

/// Kinds whose dates belong on another record's timeline.
const RELATED: [&str; 3] = ["investigation", "ticket", "decision"];

type Event = (String, String, Row);

fn line(session: &Session<'_>, rec: &Rec, key: &str) -> u32 {
    let text = session.text(&rec.path).unwrap_or_default();
    if rec.path.ends_with(".json") {
        line_of(&text, key)
    } else {
        field_line(&text, key)
    }
}

/// `created` and (when it differs) `updated` events of one record.
fn dated(session: &Session<'_>, rec: &Rec) -> Vec<Event> {
    let mut out = Vec::new();
    let created = rec.created.clone();
    let updated = rec.updated.clone().filter(|u| Some(u) != created.as_ref());
    // An asset's `updated` is its `verified_at`.
    let changed = if rec.kind == "asset" {
        ("verified", "verified_at")
    } else {
        ("updated", "updated")
    };
    for ((event, key), date) in [(("created", "created"), created), (changed, updated)] {
        let Some(date) = date else { continue };
        let mut row = record_row(session, rec);
        row.text = format!("{date} {event} {}", rec.label());
        row.line = Some(line(session, rec, key));
        out.push((date, rec.id.clone(), row));
    }
    out
}

/// Checkpoint entries whose text mentions `id`: the entry heading is its
/// timestamp.
fn checkpoints(session: &Session<'_>, id: &str, own_path: Option<&str>) -> Result<Vec<Event>> {
    let mut out = Vec::new();
    for edge in query::edges(&session.conn, id, false)? {
        let Some(path) = edge.src.strip_prefix("doc:") else {
            continue;
        };
        if !(path.starts_with("runs/") && path.ends_with("/checkpoint.md")) {
            continue;
        }
        if own_path == Some(path) {
            continue;
        }
        let entry: Option<String> = rusqlite::OptionalExtension::optional(session.conn.query_row(
            "SELECT name FROM sections WHERE path = ?1 AND line_first <= ?2 AND line_last >= ?2",
            (path, edge.line),
            |r| r.get(0),
        ))?;
        let Some(stamp) = entry else { continue };
        let run = path.split('/').nth(1).unwrap_or_default().to_owned();
        let text = session
            .lines(path, edge.line, edge.line)
            .first()
            .map(|l| l.trim().chars().take(100).collect::<String>())
            .unwrap_or_default();
        let row = Row {
            kind: "checkpoint",
            id: run.clone(),
            text: format!("{stamp} checkpoint {text}"),
            path: Some(path.to_owned()),
            line: Some(edge.line),
            ..Row::default()
        };
        out.push((stamp, run, row));
    }
    Ok(out)
}

/// Every own entry is an event, even when it mentions no other record.
fn own_checkpoints(session: &Session<'_>, rec: &Rec) -> (String, Vec<Event>) {
    let (path, entries) = runs::entries(session, rec);
    let total = entries.len();
    let events = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let lines = session.lines(&path, entry.first + 1, entry.last);
            let preview: String = lines
                .iter()
                .map(|line| line.trim())
                .find(|line| !line.is_empty())
                .unwrap_or_default()
                .chars()
                .take(100)
                .collect();
            let row = Row {
                kind: "checkpoint",
                id: entry.name.clone(),
                text: format!("entry {} of {total}: {preview}", index + 1),
                path: Some(path.clone()),
                line: Some(entry.first),
                ..Row::default()
            };
            (entry.name.clone(), entry.name, row)
        })
        .collect();
    (path, events)
}

pub fn timeline(session: &Session<'_>, target: &str) -> Result<Answer> {
    let rec = match query::resolve(&session.conn, target)? {
        Resolved::One(rec) => *rec,
        Resolved::Many(candidates) => return Ok(ambiguous(session, target, &candidates)),
        Resolved::None => return unknown(session, "unresolved-id", target, &[]),
    };
    let mut related = BTreeSet::new();
    for edge in query::edges(&session.conn, &rec.id, false)? {
        related.insert(edge.src);
    }
    for edge in query::edges(&session.conn, &rec.id, true)? {
        related.insert(edge.dst);
    }
    if rec.kind == "run" {
        related.extend(query::run_destinations(&session.conn, &rec.id)?);
    }
    related.remove(&rec.id);
    let mut events = dated(session, &rec);
    let mut undated = usize::from(events.is_empty());
    for id in related {
        let Some(other) = query::record(&session.conn, &id)? else {
            continue;
        };
        if !RELATED.contains(&other.kind.as_str()) {
            continue;
        }
        let found = dated(session, &other);
        undated += usize::from(found.is_empty());
        events.extend(found);
    }
    let own = (rec.kind == "run").then(|| own_checkpoints(session, &rec));
    let checkpoint_count = own.as_ref().map(|(_, entries)| entries.len());
    events.extend(checkpoints(
        session,
        &rec.id,
        own.as_ref().map(|(path, _)| path.as_str()),
    )?);
    if let Some((_, entries)) = own {
        events.extend(entries);
    }
    events.sort_by(|a, b| (&a.0, &a.1, &a.2.text).cmp(&(&b.0, &b.1, &b.2.text)));
    let mut answer = Answer {
        rows: events.into_iter().map(|(_, _, row)| row).collect(),
        ..Answer::default()
    };
    if let Some(count) = checkpoint_count {
        answer.counts.push(("checkpoints", count));
    }
    if undated > 0 {
        answer.counts.push(("undated", undated));
    }
    if answer.rows.is_empty() {
        answer.notes.push(BOUNDARY.to_owned());
    }
    Ok(answer)
}
