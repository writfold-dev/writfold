//! Marks: `<KB Git directory>/writfold/marks/<NAME>.json`, the KB as it
//! was at a moment: `HEAD` and each dirty or untracked path with the blob
//! ID its bytes would have. Local, never committed; written whole (a
//! temporary, then a rename); marks older than 30 days are removed when a
//! mark is written.
use crate::error::{KbError, Result};
use crate::write::{Probe, Step};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-kb-mark/1";
const KEEP: i64 = 30 * 86_400;

/// A baseline: a commit, and what the dirty paths held then.
#[derive(Clone, Debug, Default)]
pub struct Mark {
    pub name: String,
    pub head: String,
    pub taken: String,
    /// Path (repository-relative) to its blob ID, `None` when deleted.
    pub dirty: Vec<(String, Option<String>)>,
}

pub fn dir(state: &Path) -> PathBuf {
    state.join("marks")
}

fn file(state: &Path, name: &str) -> Result<PathBuf> {
    if !crate::draft::valid_session(name) {
        return Err(KbError::Refused(format!(
            "{name:?} is not a mark name (letters, digits, '.', '-', '_')"
        )));
    }
    Ok(dir(state).join(format!("{name}.json")))
}

/// Whether the mark exists.
pub fn exists(state: &Path, name: &str) -> bool {
    file(state, name).is_ok_and(|f| f.is_file())
}

/// Write `mark` whole, then remove marks older than 30 days.
pub fn write(state: &Path, mark: &Mark, now: i64, probe: Probe) -> Result<()> {
    let target = file(state, &mark.name)?;
    std::fs::create_dir_all(dir(state))?;
    let dirty: Map<String, Value> = mark
        .dirty
        .iter()
        .map(|(p, id)| (p.clone(), json!(id)))
        .collect();
    let value = json!({"schema": SCHEMA, "name": mark.name, "head": mark.head,
        "taken": mark.taken, "dirty": dirty});
    let temporary = dir(state).join(format!(".{}-{}.tmp", mark.name, std::process::id()));
    std::fs::write(&temporary, format!("{value}\n"))?;
    probe(Step::TempWritten);
    std::fs::rename(&temporary, &target)?;
    probe(Step::Published);
    prune(state, now);
    Ok(())
}

fn prune(state: &Path, now: i64) {
    for entry in std::fs::read_dir(dir(state))
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        let old = read(state, stem)
            .ok()
            .and_then(|m| crate::time::parse(&m.taken))
            .is_some_and(|t| now - t > KEEP);
        if old {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "best-effort prune; retried next pass"
            )]
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Read a mark; refused when it is missing or not a mark.
pub fn read(state: &Path, name: &str) -> Result<Mark> {
    let path = file(state, name)?;
    let text = std::fs::read_to_string(&path).map_err(|_| {
        KbError::Refused(format!(
            "no mark named {name}: take one with delta --mark {name}"
        ))
    })?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| KbError::Refused(format!("mark {name} is not readable: {e}")))?;
    let text_of = |key: &str| value[key].as_str().map(str::to_owned);
    let (Some(head), Some(taken), true) =
        (text_of("head"), text_of("taken"), value["schema"] == SCHEMA)
    else {
        return Err(KbError::Refused(format!(
            "mark {name} is not a {SCHEMA} file"
        )));
    };
    let dirty = value["dirty"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(p, id)| (p.clone(), id.as_str().map(str::to_owned)))
        .collect();
    Ok(Mark {
        name: name.to_owned(),
        head,
        taken,
        dirty,
    })
}
