//! `status <id> --set STATE [--by WHO] [--superseded-by ID] [--commit
//! REPO@SHA] [--evidence REF] [--note TEXT]`: one front-matter change (and
//! `updated`), re-checked against the bytes read. Tickets and
//! investigations change `status`; decisions follow their lifecycle
//! ([`super::decide`]) and get a decision-log line; a run is opened or
//! closed in its Writfold sidecar, never in the manifest.
use super::json::{Json, object};
use super::record::TICKET_STATES;
use super::{Op, Plan, atomic::digest, one_line};
use crate::error::{KbError, Result};
use crate::index::query::{self, Rec, Resolved};
use crate::runs::{self, SIDECAR};
use crate::session::Session;
use crate::time;
use std::path::{Path, PathBuf};
use writfold_sources::documents::{MarkdownDocument, parse_document};

#[derive(Clone, Debug, Default)]
pub struct SetStatus {
    pub target: String,
    pub state: String,
    pub by: Option<String>,
    pub superseded_by: Option<String>,
    pub note: Option<String>,
    /// `--set applied`: `<repo>@<sha>` of the commit that landed it.
    pub commit: Option<String>,
    /// `--set verified`: a gate receipt of the run, or `gate:<id>`.
    pub evidence: Option<String>,
    /// `--set verified`: the run whose receipts count (else the current run).
    pub run: Option<String>,
    /// `--set verified`: the subject repository (else the working directory).
    pub repo: Option<PathBuf>,
}

const INVESTIGATION_STATES: [&str; 3] = ["open", "monitoring", "abandoned"];

fn refused(text: String) -> KbError {
    KbError::Refused(text)
}

pub fn plan(session: &Session<'_>, s: &SetStatus, cwd: &Path) -> Result<Plan> {
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
    if rec.kind != "decision" && (s.commit.is_some() || s.evidence.is_some()) {
        return Err(KbError::Usage(
            "--commit and --evidence go with a decision's --set applied or verified".into(),
        ));
    }
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
        "decision" => {
            let (text, doc) = document(session, &rec)?;
            super::decide::op(session, (&rec, &text, &doc), &state, s, cwd)?
        }
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
