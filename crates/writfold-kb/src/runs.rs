//! KB runs: `runs/<id>/manifest.json` (the shape the prototype reads, which
//! rejects unknown keys and knows only the state `open`), Writfold's
//! sidecar `runs/<id>/writfold-run.json` (the working repository, branch,
//! worktree and base commit, and the `closed` state), `checkpoint.md`, and
//! the local current-run pointer `<git dir>/writfold/current-run`.
use crate::emit::{More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec};
use crate::parse::json::line_of;
use crate::session::Session;
use crate::verbs::record_row;
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

/// The current-run pointer, if set.
pub fn current(state: &Path) -> Option<String> {
    let text = std::fs::read_to_string(state.join("current-run")).ok()?;
    let id = text.trim();
    (!id.is_empty()).then(|| id.to_owned())
}

/// Point the current run at `id` (local, untracked state).
pub fn set_current(state: &Path, id: &str) -> Result<()> {
    std::fs::create_dir_all(state)?;
    let temporary = state.join(format!(".current-run-{}.tmp", std::process::id()));
    std::fs::write(&temporary, format!("{id}\n"))?;
    std::fs::rename(temporary, state.join("current-run"))?;
    Ok(())
}

/// The current-run pointer of this session's KB.
pub fn current_of(session: &Session<'_>) -> Option<String> {
    current(&session.ws.repo.as_ref()?.git_dir.join("writfold"))
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
