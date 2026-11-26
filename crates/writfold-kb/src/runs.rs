//! KB runs: `runs/<id>/manifest.json` (the shape the prototype reads, which
//! rejects unknown keys and knows only the state `open`), Writfold's
//! sidecar `runs/<id>/writfold-run.json` (the working repository, branch,
//! worktree and base commit, and the `closed` state), `checkpoint.md`, and
//! the local current-run pointers under `<git dir>/writfold/`
//! ([`crate::seat`]).
use crate::emit::{More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec};
use crate::parse::json::line_of;
use crate::session::Session;
use crate::verbs::record_row;
use crate::workspace::Workspace;
use serde_json::{Map, Value};
use std::path::Path;

pub const SIDECAR: &str = "writfold-run.json";
/// Sidecar fields shown with a run, in order.
pub const PLACE: [&str; 4] = ["repo", "branch", "worktree", "base"];

/// The run's directory, root-relative.
pub fn dir(rec: &Rec) -> String {
    rec.path.rsplit_once('/').map_or("", |(d, _)| d).to_owned()
}

/// The sidecar's text and fields, if present and an object.
pub fn sidecar(session: &Session<'_>, rec: &Rec) -> Option<(String, Map<String, Value>)> {
    let text = session.text(&format!("{}/{SIDECAR}", dir(rec)))?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(map)) => Some((text, map)),
        _ => None,
    }
}

/// The effective state: the sidecar's, else the manifest's, else `open`.
pub fn state(session: &Session<'_>, rec: &Rec) -> String {
    sidecar(session, rec)
        .and_then(|(_, map)| map.get("state")?.as_str().map(str::to_owned))
        .or_else(|| rec.status.clone())
        .unwrap_or_else(|| "open".into())
}

/// The checkpoint file's path and its entries (`## <time>` sections).
pub fn entries(session: &Session<'_>, rec: &Rec) -> (String, Vec<crate::model::Section>) {
    let path = format!("{}/checkpoint.md", dir(rec));
    let text = session.text(&path).unwrap_or_default();
    let found = crate::parse::sections(&text);
    (path, found)
}

/// The current run of the workspace's seat ([`crate::seat`]): its
/// session's pointer, else its work tree's, else the shared one.
pub fn current_in(ws: &Workspace) -> Option<String> {
    crate::seat::current(ws, &ws.seat).map(|(found, _)| found.id)
}

/// Point the clone's shared pointer at `id` (local, untracked state).
/// A seat making a run current goes through [`crate::seat::make_current`].
pub fn set_current(state: &Path, id: &str) -> Result<()> {
    crate::seat::make_current(state, &crate::seat::Seat::default(), id)
}

/// The current run of this session's seat.
pub fn current_of(session: &Session<'_>) -> Option<String> {
    current_in(session.ws)
}

/// The current run of this session's seat acting in `dir` (a guarded
/// command is judged where it acts).
pub fn current_at(session: &Session<'_>, dir: &Path) -> Option<String> {
    let seat = session.ws.seat.moved_to(session.ws, dir);
    crate::seat::current(session.ws, &seat).map(|(found, _)| found.id)
}

/// The run row (effective state, place fields) and the newest checkpoint
/// entry as a second row.
pub fn rows(session: &Session<'_>, rec: &Rec) -> Result<Vec<Row>> {
    let mut row = record_row(session, rec);
    row.kind = "run";
    row.status = Some(state(session, rec));
    for key in ["project", "mode", "created"] {
        if let Some(value) = rec.field(key) {
            row.more.push(field(
                key,
                &value,
                &rec.path,
                line_of(&rec.extra_raw(session), key),
            ));
        }
    }
    if let Some((text, map)) = sidecar(session, rec) {
        let path = format!("{}/{SIDECAR}", dir(rec));
        for key in PLACE {
            if let Some(value) = map.get(key).and_then(Value::as_str) {
                row.more.push(field(key, value, &path, line_of(&text, key)));
            }
        }
    }
    if current_of(session).as_deref() == Some(rec.id.as_str()) {
        row.flags.push("current".into());
    }
    row.more
        .extend(crate::agents::shown::summary(&session.ws.root, rec));
    let mut out = vec![row];
    let (path, found) = entries(session, rec);
    if let Some(entry) = found.last() {
        let mut tail = Row {
            kind: "checkpoint",
            id: entry.name.clone(),
            text: format!("entry {} of {}", found.len(), found.len()),
            path: Some(path.clone()),
            line: Some(entry.first),
            ..Row::default()
        };
        let lines = session.lines(&path, entry.first + 1, entry.last);
        for (offset, line) in lines.iter().enumerate() {
            if !line.trim().is_empty() && tail.more.len() < 12 {
                tail.more.push(More {
                    extra: Vec::new(),
                    status: None,
                    label: "note",
                    text: line.trim().to_owned(),
                    path: Some(path.clone()),
                    line: Some(entry.first + 1 + u32::try_from(offset).unwrap_or(0)),
                    flags: Vec::new(),
                });
            }
        }
        out.push(tail);
    }
    Ok(out)
}

fn field(key: &str, value: &str, path: &str, line: u32) -> More {
    More {
        extra: Vec::new(),
        status: None,
        label: "field",
        text: format!("{key}={value}"),
        path: Some(path.to_owned()),
        line: Some(line),
        flags: Vec::new(),
    }
}

impl Rec {
    /// The record file's current text (for field lines).
    pub fn extra_raw(&self, session: &Session<'_>) -> String {
        session.text(&self.path).unwrap_or_default()
    }
}

/// Every run, newest activity first: the last checkpoint heading, else
/// `created`.
pub fn all(session: &Session<'_>) -> Result<Vec<(Rec, String)>> {
    let found = query::records(&session.conn, "kind = 'run' ORDER BY id", &[])?;
    let mut out: Vec<(Rec, String)> = found
        .into_iter()
        .map(|rec| {
            let (_, entries) = entries(session, &rec);
            let at = entries
                .last()
                .map(|e| e.name.clone())
                .or_else(|| rec.created.clone())
                .unwrap_or_default();
            (rec, at)
        })
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.id.cmp(&a.0.id)));
    Ok(out)
}

/// A state word, classified for the kernel: `open`, `closed`, else other.
pub fn classify(state: &str) -> writfold_core::runs::State {
    use writfold_core::runs::State;
    match state {
        "open" => State::Open,
        "closed" => State::Closed,
        _ => State::Other,
    }
}

/// What the run list cannot show (#98): whether it may claim to be
/// complete ([`writfold_core::runs::complete`]), the `no_manifest` count,
/// and a note naming each run directory without `manifest.json` and each
/// `manifest.json` outside `runs/<id>/`.
pub fn inventory(session: &Session<'_>, answer: &mut crate::emit::Answer) -> Result<()> {
    let bare = query::runs_without_manifest(&session.conn)?;
    let stray = query::stray_manifests(&session.conn)?;
    let unreadable = query::unreadable_runs(&session.conn)?;
    if !writfold_core::runs::complete(unreadable, bare.len()) {
        answer.partial = true;
    }
    if !bare.is_empty() {
        answer.counts.push(("no_manifest", bare.len()));
        answer.notes.push(format!(
            "no manifest.json, so not listed: {}",
            bare.join(" ")
        ));
    }
    if !stray.is_empty() {
        answer.notes.push(format!(
            "only runs/<id>/manifest.json is a run; not runs: {}",
            stray.join(" ")
        ));
    }
    Ok(())
}
