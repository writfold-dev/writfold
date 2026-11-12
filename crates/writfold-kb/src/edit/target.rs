//! Where an edit applies: the run's recorded worktree (its sidecar's
//! `worktree`), or `--worktree DIR` when the run records none, which is then
//! recorded in the sidecar with the repository, branch and base observed
//! there. The worktree must exist on this machine, be the top of a Git work
//! tree, and belong to the repository the run records.
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::runs::{self, SIDECAR};
use crate::session::Session;
use crate::workspace::find_repo;
use crate::write::Op;
use crate::write::atomic::digest;
use crate::write::json::{Json, object};
use crate::write::run::observe_place;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The located worktree.
pub struct Target {
    pub top: PathBuf,
    pub git_dir: PathBuf,
    /// The run's (or observed) repository name.
    pub repo: String,
    /// The run's base commit, when it records one.
    pub base: Option<String>,
    /// The worktree's HEAD now.
    pub head: Option<String>,
    /// The sidecar write that records `--worktree`, when the run had none.
    pub record: Option<Op>,
}

fn field(map: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<String> {
    map?.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Resolve and check the worktree of `rec`.
pub fn resolve(
    session: &Session<'_>,
    rec: &Rec,
    explicit: Option<&Path>,
    cwd: &Path,
) -> Result<Target> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("edits need a KB under Git".into()))?;
    let sidecar = runs::sidecar(session, rec);
    let map = sidecar.as_ref().map(|(_, m)| m);
    let recorded = field(map, "worktree");
    let dir = match (&recorded, explicit) {
        (None, None) => {
            return Err(KbError::Refused(format!(
                "run {} records no worktree; pass --worktree DIR (it is then recorded in the run)",
                rec.id
            )));
        }
        (Some(path), None) => PathBuf::from(path),
        (_, Some(dir)) => cwd.join(dir),
    };
    let top = std::fs::canonicalize(&dir)
        .map_err(|_| KbError::Refused(format!("the worktree {} is missing here", dir.display())))?;
    if let (Some(path), Some(_)) = (&recorded, explicit)
        && std::fs::canonicalize(path).ok().as_ref() != Some(&top)
    {
        return Err(KbError::Refused(format!(
            "run {} records the worktree {path}, not {}",
            rec.id,
            top.display()
        )));
    }
    let repo = find_repo(&top)?
        .filter(|r| r.work_tree == top)
        .ok_or_else(|| {
            KbError::Refused(format!(
                "{} is not the top of a Git work tree",
                top.display()
            ))
        })?;
    let place = observe_place(git, &top)?
        .ok_or_else(|| KbError::Refused(format!("{} is not a Git work tree", top.display())))?;
    let observed = |key: &str| {
        place
            .fields
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    let name = observed("repo").unwrap_or_default();
    if let Some(wanted) = field(map, "repo")
        && wanted != name
    {
        return Err(KbError::Refused(format!(
            "{} is a work tree of {name}, but run {} records the repository {wanted}",
            top.display(),
            rec.id
        )));
    }
    let record = match recorded {
        Some(_) => None,
        None => Some(record(rec, sidecar.as_ref(), &place.fields)),
    };
    Ok(Target {
        base: field(map, "base").or_else(|| observed("base")),
        head: observed("base"),
        repo: field(map, "repo").unwrap_or(name),
        top,
        git_dir: repo.git_dir,
        record,
    })
}

/// The sidecar with the observed place added: keys in the order the other
/// sidecar writes use, existing values kept.
fn record(
    rec: &Rec,
    sidecar: Option<&(String, serde_json::Map<String, Value>)>,
    place: &[(&'static str, String)],
) -> Op {
    let path = format!("{}/{SIDECAR}", runs::dir(rec));
    let map = sidecar.map(|(_, m)| m);
    let mut fields: Vec<(&str, Json)> = Vec::new();
    for key in [
        "schema", "run_id", "state", "repo", "branch", "worktree", "base", "closed",
    ] {
        let value = field(map, key)
            .or_else(|| {
                place
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.clone())
            })
            .or_else(|| match key {
                "schema" => Some("writfold-run/1".into()),
                "run_id" => Some(rec.id.clone()),
                "state" => Some("open".into()),
                _ => None,
            });
        if let Some(value) = value {
            fields.push((key, Json::Text(value)));
        }
    }
    let bytes = object(&fields).into_bytes();
    match sidecar {
        Some((text, _)) => Op::Replace {
            path,
            expected: digest(text.as_bytes()),
            bytes,
        },
        None => Op::Create { path, bytes },
    }
}
