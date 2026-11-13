//! `status <id> --set STATE [--by WHO] [--superseded-by ID] [--note TEXT]`:
//! one front-matter change (and `updated`), re-checked against the bytes
//! read. Tickets and investigations change `status`; decisions follow the
//! lifecycle the prototype enforces (and get a decision-log line); a run
//! is opened or closed in its Writfold sidecar, never in the manifest.
use super::json::{Json, object};
use super::record::TICKET_STATES;
use super::{Op, Plan, atomic::digest, one_line};
use crate::error::{KbError, Result};
use crate::index::query::{self, Rec, Resolved};
use crate::runs::{self, SIDECAR};
use crate::session::Session;
use crate::time;
use writfold_sources::documents::{MarkdownDocument, parse_document};

#[derive(Clone, Debug, Default)]
pub struct SetStatus {
    pub target: String,
    pub state: String,
    pub by: Option<String>,
    pub superseded_by: Option<String>,
    pub note: Option<String>,
}

const INVESTIGATION_STATES: [&str; 3] = ["open", "monitoring", "abandoned"];

/// Decision transitions Writfold writes. `accepted`, `landing`, `applied`,
/// `verified` and `regressed` carry design-binding fields the prototype
/// computes and checks; v1 refuses them rather than write a record the
/// prototype would reject.
fn transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("draft", "proposed")
            | ("proposed", "draft")
            | ("proposed", "rejected")
            | ("accepted", "deferred")
    ) || (to == "superseded" && from != "superseded")
}

fn refused(text: String) -> KbError {
    KbError::Refused(text)
}

pub fn plan(session: &Session<'_>, s: &SetStatus) -> Result<Plan> {
    let rec = match query::resolve(&session.conn, &s.target)? {
        Resolved::One(rec) => *rec,
        Resolved::Many(found) => {
            let ids: Vec<&str> = found.iter().map(|r| r.id.as_str()).collect();
            return Err(refused(format!(
                "{:?} names {}; pass one ID",
                s.target,
                ids.join(", ")
            )));
        }
        Resolved::None => return Err(refused(format!("no record {:?} in the KB", s.target))),
    };
    let state = one_line("--set", &s.state)?;
    let op = match rec.kind.as_str() {
        "run" => run(session, &rec, &state)?,
        "ticket" if TICKET_STATES.contains(&state.as_str()) => fields(session, &rec, &state, &[])?,
        "investigation" if INVESTIGATION_STATES.contains(&state.as_str()) => {
            fields(session, &rec, &state, &[])?
        }
        "investigation" if state == "concluded" => {
            let (_, doc) = document(session, &rec)?;
            super::conclude::check(&rec.id, &doc)?;
            fields(session, &rec, &state, &[])?
        }
        "decision" => decision(session, &rec, &state, s)?,
        "ticket" | "investigation" => {
            return Err(KbError::Usage(format!(
                "{state:?} is not a {} state",
                rec.kind
            )));
        }
        other => {
            return Err(refused(format!(
                "status --set does not write {other} records in v1"
            )));
        }
    };
    Ok(Plan {
        ops: vec![op],
        show: Some(rec.id.clone()),
        notes: Vec::new(),
        current: None,
        verb: "status-set",
        args: vec![rec.id, state],
    })
}

fn document(session: &Session<'_>, rec: &Rec) -> Result<(String, MarkdownDocument)> {
    let text = session
        .text(&rec.path)
        .ok_or_else(|| refused(format!("{} cannot be read", rec.path)))?;
    let doc = parse_document(text.clone())
        .map_err(|e| refused(format!("{} does not parse: {e}", rec.path)))?;
    Ok((text, doc))
}

/// `status`, `updated` and any `extra` fields, in place.
fn fields(session: &Session<'_>, rec: &Rec, state: &str, extra: &[(&str, &str)]) -> Result<Op> {
    let (text, doc) = document(session, rec)?;
    let date = time::date(session.now);
    let mut changes: Vec<(&str, Option<&str>)> =
        vec![("status", Some(state)), ("updated", Some(&date))];
    changes.extend(extra.iter().map(|(k, v)| (*k, Some(*v))));
    let edited = doc
        .update_fields(&changes)
        .map_err(|e| KbError::Usage(format!("cannot edit {}: {e}", rec.path)))?;
    Ok(Op::Replace {
        path: rec.path.clone(),
        expected: digest(text.as_bytes()),
        bytes: edited.as_bytes().to_vec(),
    })
}

fn decision(session: &Session<'_>, rec: &Rec, to: &str, s: &SetStatus) -> Result<Op> {
    let (text, doc) = document(session, rec)?;
    let from = rec.status.as_deref().unwrap_or("draft");
    if doc.field("workflow_version").map(str::trim) != Some("3") {
        return Err(refused(format!(
            "{} is not a workflow 3 record; adopt it with the prototype first",
            rec.id
        )));
    }
    if !transition(from, to) {
        return Err(refused(format!(
            "{} cannot go from {from} to {to} with v1 writes",
            rec.id
        )));
    }
    let by = s.by.as_deref().map(str::trim).unwrap_or_default();
    let date = time::date(session.now);
    let mut changes: Vec<(&str, Option<&str>)> = vec![
        ("status", Some(to)),
        ("updated", Some(&date)),
        ("last_actor", Some(by)),
    ];
    if to == "rejected" {
        if by.is_empty() {
            return Err(KbError::Usage("rejecting a decision needs --by WHO".into()));
        }
        if doc.field("decided_by").is_none_or(|v| v.trim().is_empty()) {
            changes.push(("decided_by", Some(by)));
        }
    }
    let successor;
    if to == "superseded" {
        successor = successor_of(session, rec, s.superseded_by.as_deref())?;
        changes.push(("superseded_by", Some(&successor)));
    } else if s.superseded_by.is_some() {
        return Err(KbError::Usage(
            "--superseded-by goes with --set superseded".into(),
        ));
    }
    if to == "proposed" {
        ready(&doc)?;
    }
    let mut line = format!("- {date}: {to}");
    if !by.is_empty() {
        line.push_str(&format!(" by {by}"));
    }
    if let Some(note) = s.note.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        line.push_str(&format!(" \u{2014} {}", note.replace(['\n', '\r'], " ")));
    }
    let edited = doc
        .update_fields(&changes)
        .and_then(|d| d.append_section("Decision log", &line))
        .map_err(|e| KbError::Usage(format!("cannot edit {}: {e}", rec.path)))?;
    Ok(Op::Replace {
        path: rec.path.clone(),
        expected: digest(text.as_bytes()),
        bytes: edited.as_bytes().to_vec(),
    })
}

/// The successor must be another decision, and the chain from it must not
/// come back to `rec`.
fn successor_of(session: &Session<'_>, rec: &Rec, named: Option<&str>) -> Result<String> {
    let named =
        named.ok_or_else(|| KbError::Usage("--set superseded needs --superseded-by ID".into()))?;
    let bare = named.strip_prefix("proposal/").unwrap_or(named);
    let mut next = Some(bare.to_owned());
    let mut seen = 0;
    while let Some(id) = next {
        let found = query::record(&session.conn, &format!("proposal/{id}"))?
            .ok_or_else(|| refused(format!("no decision {id} in the KB")))?;
        if found.id == rec.id || seen > 64 {
            return Err(refused(format!(
                "superseding {} by {bare} makes a cycle",
                rec.id
            )));
        }
        seen += 1;
        next = found.field("superseded_by");
    }
    Ok(bare.to_owned())
}

/// The prototype's review readiness for `proposed`: research written,
/// two alternatives, and an acceptance-gates table.
fn ready(doc: &MarkdownDocument) -> Result<()> {
    let section = |name: &str| doc.section(name).unwrap_or_default().trim().to_owned();
    let alternatives = section("Alternatives considered")
        .lines()
        .filter(|l| {
            let l = l.trim_start();
            l.starts_with("- ")
                || l.starts_with("* ")
                || l.split_once(['.', ')'])
                    .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .count();
    let gates = section("Acceptance gates");
    let header = gates.lines().next().unwrap_or_default().to_lowercase();
    let table = ["id", "when", "claim", "check", "by"]
        .iter()
        .all(|c| header.split('|').any(|cell| cell.trim() == *c));
    if section("Research").is_empty() || alternatives < 2 || !table {
        return Err(refused(
            "not ready for review: fill Research, at least two Alternatives considered, and an Acceptance gates table (id | when | claim | check | by)".into(),
        ));
    }
    Ok(())
}

/// Open or close a run in its sidecar.
fn run(session: &Session<'_>, rec: &Rec, to: &str) -> Result<Op> {
    if !matches!(to, "open" | "closed") {
        return Err(KbError::Usage("a run is open or closed".into()));
    }
    let path = format!("{}/{SIDECAR}", runs::dir(rec));
    let stamp = time::stamp(session.now);
    let Some((text, map)) = runs::sidecar(session, rec) else {
        let mut fields = vec![
            ("schema", Json::Text("writfold-run/1".into())),
            ("run_id", Json::Text(rec.id.clone())),
            ("state", Json::Text(to.into())),
        ];
        if to == "closed" {
            fields.push(("closed", Json::Text(stamp)));
        }
        return Ok(Op::Create {
            path,
            bytes: object(&fields).into_bytes(),
        });
    };
    let mut fields: Vec<(String, Json)> = Vec::new();
    for key in [
        "schema", "run_id", "state", "repo", "branch", "worktree", "base",
    ] {
        if key == "state" {
            fields.push((key.into(), Json::Text(to.into())));
        } else if let Some(value) = map.get(key).and_then(|v| v.as_str()) {
            fields.push((key.into(), Json::Text(value.into())));
        }
    }
    if to == "closed" {
        fields.push(("closed".into(), Json::Text(stamp)));
    }
    let borrowed: Vec<(&str, Json)> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    Ok(Op::Replace {
        path,
        expected: digest(text.as_bytes()),
        bytes: object(&borrowed).into_bytes(),
    })
}
