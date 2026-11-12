//! `checkpoint`: append one entry to a run's `checkpoint.md`, or start a
//! run (`--new-run`) whose first entry is the note.
//!
//! An entry is a blank line, `## <UTC time>`, a blank line and the note.
//! A new run is one directory staged and renamed into place: the manifest
//! in the prototype's shape (it rejects unknown keys and knows only the
//! state `open`), the checkpoint, the sidecar with the working repository,
//! branch, worktree and base commit (recorded, never created), and empty
//! `observations/` and `receipts/` directories.
use super::json::{Json, object};
use super::{Op, Plan, atomic::digest, one_line};
use crate::error::{KbError, Result};
use crate::index::query::{self, Rec};
use crate::runs::{self, SIDECAR};
use crate::session::Session;
use crate::time;
use crate::workspace::{CANCELLED, budgets, find_repo};
use sha2::{Digest, Sha256};
use std::path::Path;
use writfold_runtime::git::{KbRepository, kb_config, kb_status};

pub const MODES: [&str; 3] = ["advice", "authoring", "review"];

/// `--new-run --project P --task T [--mode M]`.
#[derive(Clone, Debug)]
pub struct NewRun {
    pub project: String,
    pub task: String,
    pub mode: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub run: Option<String>,
    pub new_run: Option<NewRun>,
    pub note: String,
}

/// The bytes of one entry.
pub fn entry(stamp: &str, note: &str) -> String {
    format!("\n## {stamp}\n\n{}\n", note.trim())
}

pub fn plan(session: &Session<'_>, request: &Checkpoint, cwd: &Path) -> Result<Plan> {
    let note = request.note.trim();
    if note.is_empty() {
        return Err(KbError::Usage("checkpoint needs a non-empty --note".into()));
    }
    let stamp = time::stamp(session.now);
    if let Some(new) = &request.new_run {
        return new_run(session, new, note, &stamp, cwd);
    }
    let rec = target(session, request.run.as_deref())?;
    if runs::state(session, &rec) == "closed" {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    let (path, entries) = runs::entries(session, &rec);
    let old = session.text(&path);
    if let Some(last) = entries.last() {
        let body = session.lines(&path, last.first + 1, last.last).join("\n");
        if body.trim() == note {
            return Err(KbError::Refused(format!(
                "the last entry of {path} ({}) already holds this note",
                last.name
            )));
        }
    }
    let addition = entry(&stamp, note);
    let op = match old {
        None => Op::Create {
            path: path.clone(),
            bytes: addition.into_bytes(),
        },
        Some(text) => Op::Replace {
            path: path.clone(),
            expected: digest(text.as_bytes()),
            bytes: format!("{text}{addition}").into_bytes(),
        },
    };
    Ok(Plan {
        ops: vec![op],
        show: Some(rec.id.clone()),
        notes: Vec::new(),
        current: Some(rec.id.clone()),
        verb: "checkpoint",
        args: vec![rec.id, stamp],
    })
}

/// The run named, else the current-run pointer.
fn target(session: &Session<'_>, named: Option<&str>) -> Result<Rec> {
    let id = match named {
        Some(id) => id.to_owned(),
        None => runs::current_of(session).ok_or_else(|| {
            KbError::Refused(
                "no current run: pass --run ID, or start one with --new-run --project P --task T"
                    .into(),
            )
        })?,
    };
    match query::record(&session.conn, &id)? {
        Some(rec) if rec.kind == "run" => Ok(rec),
        _ => Err(KbError::Refused(format!("no run {id} in the KB"))),
    }
}

fn new_run(
    session: &Session<'_>,
    new: &NewRun,
    note: &str,
    stamp: &str,
    cwd: &Path,
) -> Result<Plan> {
    let project = one_line("--project", &new.project)?;
    let task = one_line("--task", &new.task)?;
    let mode = new.mode.clone().unwrap_or_else(|| "authoring".into());
    if !MODES.contains(&mode.as_str()) {
        return Err(KbError::Usage(format!(
            "--mode must be one of {}",
            MODES.join(", ")
        )));
    }
    let compact: String = stamp.chars().filter(|c| *c != '-' && *c != ':').collect();
    let mut salt = Sha256::new();
    salt.update(format!("{stamp}\0{}\0{project}\0{task}", std::process::id()).as_bytes());
    salt.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
            .to_le_bytes(),
    );
    let id = format!(
        "run-{compact}-{}",
        &crate::index::hex(&salt.finalize())[..6]
    );
    let manifest = object(&[
        ("schema_version", Json::Int(1)),
        ("run_id", Json::Text(id.clone())),
        ("task", Json::Text(task.clone())),
        ("project", Json::Text(project.clone())),
        ("mode", Json::Text(mode.clone())),
        ("created", Json::Text(stamp.to_owned())),
        ("state", Json::Text("open".into())),
    ]);
    let mut extra = vec![
        ("schema", Json::Text("writfold-run/1".into())),
        ("run_id", Json::Text(id.clone())),
        ("state", Json::Text("open".into())),
    ];
    extra.extend(place(session, cwd)?);
    let mut notes = Vec::new();
    if query::record(&session.conn, &format!("project:{project}"))?.is_none() {
        notes.push(format!("no project {project} in the KB yet"));
    }
    let files = vec![
        ("manifest.json".to_owned(), manifest.into_bytes()),
        ("checkpoint.md".to_owned(), entry(stamp, note).into_bytes()),
        (SIDECAR.to_owned(), object(&extra).into_bytes()),
        ("observations/".to_owned(), Vec::new()),
        ("receipts/".to_owned(), Vec::new()),
    ];
    Ok(Plan {
        ops: vec![Op::CreateDir {
            path: format!("runs/{id}"),
            files,
        }],
        show: Some(id.clone()),
        notes,
        current: Some(id.clone()),
        verb: "checkpoint",
        args: vec![id, stamp.to_owned(), project, task, mode],
    })
}

/// Where the work happens: the repository enclosing `cwd`, its branch,
/// worktree and HEAD. Recorded only; nothing is created.
fn place(session: &Session<'_>, cwd: &Path) -> Result<Vec<(&'static str, Json)>> {
    let (Some(git), Ok(Some(repo))) = (&session.git, find_repo(cwd)) else {
        return Ok(Vec::new());
    };
    let reads = KbRepository {
        git_dir: &repo.git_dir,
        work_tree: Some(&repo.work_tree),
    };
    let status = kb_status(git, reads, ".", budgets()?, &CANCELLED)?;
    let origin = kb_config(git, reads, "remote.origin.url", budgets()?, &CANCELLED)?;
    let name = origin
        .as_deref()
        .and_then(|url| url.trim_end_matches('/').rsplit(['/', ':']).next())
        .map(|n| n.trim_end_matches(".git").to_owned())
        .or_else(|| {
            repo.work_tree
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let mut out = vec![("repo", Json::Text(name))];
    if let Some(branch) = status.branch {
        out.push(("branch", Json::Text(branch)));
    }
    out.push((
        "worktree",
        Json::Text(repo.work_tree.to_string_lossy().into_owned()),
    ));
    if let Some(head) = status.head {
        out.push(("base", Json::Text(head.as_str().to_owned())));
    }
    Ok(out)
}
