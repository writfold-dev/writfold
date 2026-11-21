//! The operator's required set at a policy revision (#57): the IDs
//! `.writfold/handoff.toml` requires and each one's gate definition, read
//! from Git at that commit, never from the work tree or the candidate.
//! Reading is tested IO; the decision is `writfold_core::handoff`.
use crate::error::{KbError, Result};
use crate::gate::def::{DIR, Definition, Origin};
use crate::gate::subject::Subject;
use crate::gate::toml::{Value, parse};
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_domain::GitObjectId;
use writfold_runtime::git::GitExecutable;
use writfold_runtime::git::{gate_ls_tree, pinned_blobs};

/// Where the required set lives, relative to the repository's top.
pub const FILE: &str = ".writfold/handoff.toml";

/// The required set at the policy revision.
pub struct Required {
    /// The required IDs, in the file's order.
    pub ids: Vec<String>,
    /// Each ID's approved definition (`None`: no file at the revision, or
    /// it did not parse, with why).
    pub definitions: Vec<std::result::Result<Definition, String>>,
    /// Whether the revision's gate definitions could all be read.
    pub complete: bool,
}

/// The bytes of `paths` in `commit`'s tree (`None` for a path not there).
fn files(
    git: &GitExecutable<'_>,
    subject: &Subject,
    commit: &GitObjectId,
    paths: &[String],
) -> Result<Vec<Option<Vec<u8>>>> {
    let (budgets, cancelled) = (budgets()?, &CANCELLED);
    let tree = gate_ls_tree(git, &subject.git_dir, commit, budgets, cancelled)?;
    let mut ids = Vec::new();
    let mut at = Vec::new();
    for path in paths {
        match tree.iter().find(|e| &e.path == path) {
            Some(entry) => {
                let id = GitObjectId::new(commit.format(), entry.id.clone())
                    .map_err(|_| KbError::Refused(format!("{path}: malformed object ID")))?;
                at.push(Some(ids.len()));
                ids.push(id);
            }
            None => at.push(None),
        }
    }
    let mut blobs = pinned_blobs(git, &subject.git_dir, &ids, budgets, cancelled)?;
    let mut out = Vec::new();
    for slot in at {
        out.push(slot.and_then(|i| blobs.get_mut(i).map(std::mem::take)));
    }
    Ok(out)
}

/// The IDs a `handoff.toml` requires, or why it does not parse.
pub fn ids(text: &str) -> std::result::Result<Vec<String>, String> {
    let map = parse(text)?;
    match map.get("schema") {
        Some(Value::Str(s)) if s == "writfold-handoff/1" => {}
        _ => return Err("schema must be \"writfold-handoff/1\"".into()),
    }
    if let Some(key) = map
        .keys()
        .find(|k| !matches!(k.as_str(), "schema" | "required"))
    {
        return Err(format!("unknown key `{key}`"));
    }
    match map.get("required") {
        Some(Value::List(ids)) => Ok(ids.clone()),
        _ => Err("`required` must be a list of gate IDs".into()),
    }
}

/// Read the required set and its definitions at `commit`.
pub fn read(git: &GitExecutable<'_>, subject: &Subject, commit: &GitObjectId) -> Result<Required> {
    let [handoff] = files(git, subject, commit, &[FILE.to_owned()])?
        .try_into()
        .map_err(|_| KbError::Refused("one file expected".into()))?;
    let Some(bytes) = handoff else {
        return Err(KbError::Refused(format!(
            "no required set: {FILE} does not exist at the policy revision {}; an operator lists the gates a handoff needs there (schema = \"writfold-handoff/1\", required = [\"unit\", …])",
            short(commit.as_str())
        )));
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| KbError::Refused(format!("{FILE} at the policy revision is not UTF-8")))?;
    let ids = ids(&text)
        .map_err(|why| KbError::Refused(format!("{FILE} at the policy revision: {why}")))?;
    let paths: Vec<String> = ids.iter().map(|id| format!("{DIR}/{id}.toml")).collect();
    let (texts, complete) = match files(git, subject, commit, &paths) {
        Ok(texts) => (texts, true),
        Err(_) => (vec![None; paths.len()], false),
    };
    let dir = subject.top.join(DIR);
    let definitions = (ids.iter().zip(paths.iter()).zip(texts))
        .map(|((id, path), bytes)| definition(id, path, bytes, &dir))
        .collect();
    Ok(Required {
        ids,
        definitions,
        complete,
    })
}

fn definition(
    id: &str,
    path: &str,
    bytes: Option<Vec<u8>>,
    dir: &Path,
) -> std::result::Result<Definition, String> {
    let bytes = bytes.ok_or_else(|| format!("{path} does not exist at the policy revision"))?;
    let text = String::from_utf8(bytes).map_err(|_| format!("{path} is not UTF-8"))?;
    crate::gate::def_check::check(&text, id, Origin::Repo, dir, path)
        .map_err(|report| crate::defs::why(&report))
}

pub fn short(text: &str) -> &str {
    text.get(..12).unwrap_or(text)
}
