//! `status <id|project>` (read): the state of one record and, for a ticket
//! or any record others point at, the records that track it. Tracker links
//! come from fields (`prs`); commit trailers are M4.
use super::{BOUNDARY, ambiguous, link, node_row, record_row, unknown};
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec, Resolved};
use crate::index::search::distance;
use crate::parse::{field_line, json::line_of, ticket};
use crate::session::Session;
use std::collections::BTreeMap;

/// The fields that state a record's lifecycle, by kind. A project shows
/// every `STATUS.md` field.
fn keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "ticket" => &["status", "prs", "created", "updated", "promoted_to"],
        "decision" => &[
            "status",
            "created",
            "updated",
            "decided_by",
            "superseded_by",
            "landed",
            "target",
        ],
        "asset" => &["lifecycle", "owner", "verified_at", "ttl"],
        "investigation" => &["status", "created", "updated", "tickets", "signatures"],
        "signature" => &[
            "status",
            "first_seen",
            "last_seen",
            "fixed_at",
            "verified_at",
            "ttl",
        ],
        "run" => &["state", "mode", "created"],
        "relationship" => &["status", "applicability"],
        _ => &["status", "created", "updated"],
    }
}

fn fields(session: &Session<'_>, rec: &Rec) -> Vec<More> {
    let text = session.text(&rec.path).unwrap_or_default();
    let json = rec.path.ends_with(".json");
    let names: Vec<String> = if rec.kind == "project" {
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&rec.extra)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        keys(&rec.kind).iter().map(|k| (*k).to_owned()).collect()
    };
    names
        .iter()
        .filter_map(|key| {
            let value = rec.field(key)?;
            let line = if json {
                line_of(&text, key)
            } else {
                field_line(&text, key)
            };
            Some(More {
                label: "field",
                text: format!("{key}={value}"),
                path: Some(rec.path.clone()),
                line: Some(line),
                flags: Vec::new(),
            })
        })
        .collect()
}

pub fn status(session: &Session<'_>, target: &str) -> Result<Answer> {
    let rec = match query::resolve(&session.conn, target)? {
        Resolved::One(rec) => *rec,
        Resolved::Many(candidates) => return Ok(ambiguous(session, target, &candidates)),
        Resolved::None => return no_record(session, target),
    };
    let mut row = record_row(session, &rec);
    row.more.extend(fields(session, &rec));
    if rec.kind == "decision" && rec.status.as_deref() == Some("superseded") {
        let (live, _) = super::why::follow(session, rec.clone())?;
        if live.id != rec.id {
            row.more.push(More {
                label: "via",
                text: format!(
                    "live successor {} [{}]",
                    live.id,
                    live.status.clone().unwrap_or_default()
                ),
                path: Some(live.path.clone()),
                line: Some(live.line),
                flags: Vec::new(),
            });
        }
    }
    let mut answer = Answer::default();
    answer.rows.push(row);
    answer.rows.extend(trackers(session, &rec)?);
    Ok(answer)
}

/// Records that point at `rec` (by field, citation or authored link, then
/// prose mentions flagged `inferred`), one row each with its links.
fn trackers(session: &Session<'_>, rec: &Rec) -> Result<Vec<Row>> {
    let mut by_src: BTreeMap<(bool, String), Vec<More>> = BTreeMap::new();
    for edge in query::edges(&session.conn, &rec.id, false)? {
        let inferred = edge.origin == "mention" || edge.status == "inferred";
        let mut more = link(&edge);
        if inferred {
            more.flags.push("inferred".into());
        }
        by_src
            .entry((inferred, edge.src.clone()))
            .or_default()
            .push(more);
    }
    let mut rows = Vec::new();
    for ((inferred, src), links) in by_src {
        let mut row = node_row(session, &src)?;
        row.section = Some(if inferred { "mentions" } else { "tracked-by" });
        row.more.extend(links.into_iter().take(3));
        rows.push(row);
    }
    Ok(rows)
}

/// No record: up to three ticket IDs of similar spelling, else the nearest
/// full-text matches.
fn no_record(session: &Session<'_>, target: &str) -> Result<Answer> {
    let wanted = ticket(target).unwrap_or_else(|| target.to_uppercase());
    let tickets = query::records(&session.conn, "kind = 'ticket' ORDER BY id", &[])?;
    let mut close: Vec<(usize, Rec)> = tickets
        .into_iter()
        .map(|rec| (distance(&wanted, &rec.id), rec))
        .filter(|(d, _)| *d <= 2)
        .collect();
    close.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    if close.is_empty() {
        return unknown(session, "no-record", target, &[]);
    }
    let rows = close
        .into_iter()
        .take(3)
        .map(|(d, rec)| Row {
            kind: "nearest",
            id: rec.id.clone(),
            status: rec.status.clone(),
            text: format!("{} (d={d})", rec.label()),
            path: Some(rec.path.clone()),
            line: Some(rec.line),
            ..Row::default()
        })
        .collect();
    Ok(Answer {
        rows,
        notes: vec![BOUNDARY.to_owned()],
        unknown: Some("no-record"),
        ..Answer::default()
    })
}
