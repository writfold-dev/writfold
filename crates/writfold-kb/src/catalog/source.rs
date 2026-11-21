//! Reading a catalog's input (#86): one Git-tracked JSON file (one object,
//! or an array at `items`) or a directory of directly contained tracked
//! `.json` files, one object each. Paths are checked (relative, no `..`,
//! no Git internals, no symlink component, not an output or declaration
//! directory), duplicate JSON keys are refused, and every configured
//! pointer must hold a value of the right type. Anything missing, partly
//! listed or malformed is a fault, never an empty inventory. Tested IO.
use super::decl::{Decl, FIELDS};
use super::render::{Value, record};
use crate::workspace::{CANCELLED, Workspace, budgets};
use std::collections::BTreeSet;
use std::path::Path;
use writfold_runtime::git::{GitExecutable, kb_ls_files, kb_ls_tracked};

/// The most bytes one input file may hold.
const LIMIT: u64 = 16 * 1024 * 1024;

/// One wanted asset record.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    /// The rendered record.
    pub bytes: String,
}

/// What a catalog's input gave: its rows, every fault, and the files read
/// with their bytes (for the re-check).
#[derive(Debug, Default)]
pub struct Read {
    pub rows: Vec<Row>,
    pub faults: Vec<String>,
    pub read: Vec<(String, Vec<u8>)>,
}

/// Why `rel` may not be a catalog input, if it may not.
pub fn unsafe_path(root: &Path, rel: &str) -> Option<String> {
    let parts: Vec<&str> = rel.split('/').collect();
    if rel.is_empty() || rel.starts_with('/') || rel.contains('\\') || rel.contains("://") {
        return Some("must be a KB-relative path".into());
    }
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return Some("must not hold empty, . or .. components".into());
    }
    if parts[0] == ".git" || rel.starts_with("inventory/") || rel.starts_with(".writfold/catalogs")
    {
        return Some("must not be inside .git, inventory/ or .writfold/catalogs/".into());
    }
    let mut at = root.to_path_buf();
    for part in parts {
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(m) if m.file_type().is_symlink() => {
                return Some(format!("{} is a symbolic link", at.display()));
            }
            Ok(_) => {}
            Err(_) => return Some("does not exist".into()),
        }
    }
    None
}

/// Duplicate object keys in valid JSON `text`, as the first one found.
pub fn duplicate_key(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut stack: Vec<(bool, BTreeSet<String>, bool)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                if let Some((true, keys, want)) = stack.last_mut().filter(|f| f.0 && f.2) {
                    let raw = text.get(start..=i).unwrap_or_default();
                    let key: String = serde_json::from_str(raw).unwrap_or_default();
                    if !keys.insert(key.clone()) {
                        return Some(key);
                    }
                    *want = false;
                }
            }
            b'{' => stack.push((true, BTreeSet::new(), true)),
            b'[' => stack.push((false, BTreeSet::new(), false)),
            b'}' | b']' => {
                stack.pop();
            }
            b',' => {
                if let Some(frame) = stack.last_mut().filter(|f| f.0) {
                    frame.2 = true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The KB-relative files Git tracks under `rel`, and the untracked ones
/// (the runtime passes `--literal-pathspecs`, so `rel` is never a glob).
fn listed(
    ws: &Workspace,
    git: &GitExecutable<'_>,
    rel: &str,
) -> Result<(Vec<String>, Vec<String>), String> {
    let repo = ws
        .repo
        .as_ref()
        .ok_or("the KB is not in a Git repository")?;
    let spec = format!("{}{rel}", repo.prefix);
    let strip = |all: Vec<String>| -> Vec<String> {
        (all.into_iter())
            .filter_map(|p| p.strip_prefix(repo.prefix.as_str()).map(str::to_owned))
            .collect()
    };
    let budget = budgets().map_err(|e| e.to_string())?;
    let tracked = strip(
        kb_ls_tracked(git, repo.reads(), &spec, budget, &CANCELLED).map_err(|e| e.to_string())?,
    );
    let all = strip(
        kb_ls_files(git, repo.reads(), &spec, budget, &CANCELLED).map_err(|e| e.to_string())?,
    );
    let untracked = all.into_iter().filter(|p| !tracked.contains(p)).collect();
    Ok((tracked, untracked))
}

/// The input files to read, or the faults that forbid reading.
fn files(ws: &Workspace, git: &GitExecutable<'_>, decl: &Decl, out: &mut Read) -> Vec<String> {
    let rel = decl.input.as_str();
    if let Some(why) = unsafe_path(&ws.root, rel) {
        out.faults.push(format!("input {rel}: {why}"));
        return Vec::new();
    }
    let (tracked, untracked) = match listed(ws, git, rel) {
        Ok(found) => found,
        Err(why) => {
            out.faults
                .push(format!("input {rel}: cannot be listed ({why})"));
            return Vec::new();
        }
    };
    if ws.root.join(rel).is_file() {
        if !tracked.iter().any(|p| p == rel) {
            out.faults.push(format!(
                "input {rel}: not tracked by Git; stage it (git add) before projecting"
            ));
            return Vec::new();
        }
        return vec![rel.to_owned()];
    }
    if decl.items.is_some() {
        out.faults.push(format!("input {rel}: `items` selects an array in one file; a directory holds one object per file"));
    }
    let direct =
        |p: &&String| p.ends_with(".json") && p.rsplit_once('/').is_some_and(|(d, _)| d == rel);
    for p in untracked.iter().filter(direct) {
        out.faults.push(format!(
            "input {p}: not tracked by Git; stage it or move it out of {rel}"
        ));
    }
    let found: Vec<String> = tracked.iter().filter(direct).cloned().collect();
    if found.is_empty() {
        out.faults.push(format!(
            "input {rel}: no tracked .json file; an empty inventory is an explicit [] in one file"
        ));
    }
    found
}

/// The value at `pointer` in `object` as a field value, or why not.
fn field(
    object: &serde_json::Value,
    (name, list): (&str, bool),
    pointer: &str,
) -> Result<Value, String> {
    let at = object
        .pointer(pointer)
        .ok_or_else(|| format!("field `{name}` ({pointer}) is missing"))?;
    match (list, at) {
        (false, serde_json::Value::String(s)) => Ok(Value::Text(s.clone())),
        (true, serde_json::Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str().map(str::to_owned).ok_or_else(|| {
                    format!("field `{name}` ({pointer}) must be an array of strings")
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::List),
        (false, _) => Err(format!("field `{name}` ({pointer}) must be a string")),
        (true, _) => Err(format!(
            "field `{name}` ({pointer}) must be an array of strings"
        )),
    }
}

/// One object as a row, or the fault at `at`.
fn row(decl: &Decl, object: &serde_json::Value, source: &str, at: &str) -> Result<Row, String> {
    if !object.is_object() {
        return Err(format!("{at}: not a JSON object"));
    }
    let mut values = Vec::new();
    for (name, pointer) in &decl.fields {
        let list = FIELDS.iter().any(|(n, l)| n == name && *l);
        let value = field(object, (name, list), pointer).map_err(|why| format!("{at}: {why}"))?;
        values.push((*name, value));
    }
    let id = match values.first() {
        Some(("id", Value::Text(id))) if crate::write::is_slug(id) => id.clone(),
        Some(("id", Value::Text(id))) => {
            return Err(format!("{at}: id {id:?} is not a slug ([a-z0-9-])"));
        }
        _ => return Err(format!("{at}: no id")),
    };
    Ok(Row {
        bytes: record(&values, &decl.name, source),
        id,
    })
}

/// The objects of one input file, each with where it is.
fn objects(
    decl: &Decl,
    path: &str,
    value: &serde_json::Value,
    single: bool,
) -> Result<Vec<(serde_json::Value, String)>, String> {
    let items = decl.items.as_deref().filter(|_| single);
    let Some(pointer) = items else {
        return Ok(vec![(value.clone(), path.to_owned())]);
    };
    match value.pointer(pointer) {
        Some(serde_json::Value::Array(list)) => Ok(list
            .iter()
            .enumerate()
            .map(|(n, v)| (v.clone(), format!("{path}#{pointer}/{n}")))
            .collect()),
        Some(_) => Err(format!("{path}: items ({pointer}) is not an array")),
        None => Err(format!("{path}: items ({pointer}) is missing")),
    }
}

/// Read one file's bytes: a regular file within the size limit.
fn bytes(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let file = root.join(path);
    match std::fs::symlink_metadata(&file) {
        Ok(m) if !m.file_type().is_file() => Err(format!("{path}: not a regular file")),
        Ok(m) if m.len() > LIMIT => Err(format!("{path}: larger than 16 MiB")),
        Ok(_) => std::fs::read(&file).map_err(|e| format!("{path}: unreadable ({e})")),
        Err(e) => Err(format!("{path}: unreadable ({e})")),
    }
}

/// Every row of `decl`'s input, and every fault.
pub fn read(ws: &Workspace, git: &GitExecutable<'_>, decl: &Decl) -> Read {
    let mut out = Read::default();
    let single = ws.root.join(&decl.input).is_file();
    for path in files(ws, git, decl, &mut out) {
        let raw = match bytes(&ws.root, &path) {
            Ok(raw) => raw,
            Err(why) => {
                out.faults.push(why);
                continue;
            }
        };
        let parsed = std::str::from_utf8(&raw)
            .map_err(|_| format!("{path}: not UTF-8"))
            .and_then(|text| match duplicate_key(text) {
                Some(key) => Err(format!("{path}: duplicate JSON key {key:?}")),
                None => serde_json::from_str::<serde_json::Value>(text)
                    .map_err(|e| format!("{path}: invalid JSON ({e})")),
            });
        out.read.push((path.clone(), raw));
        let listed = parsed.and_then(|value| objects(decl, &path, &value, single));
        match listed {
            Ok(list) => {
                for (object, at) in list {
                    match row(decl, &object, &path, &at) {
                        Ok(row) => out.rows.push(row),
                        Err(why) => out.faults.push(why),
                    }
                }
            }
            Err(why) => out.faults.push(why),
        }
    }
    let mut seen = BTreeSet::new();
    for row in &out.rows {
        if !seen.insert(row.id.clone()) {
            out.faults.push(format!(
                "asset {} is given twice by catalog {}",
                row.id, decl.name
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::duplicate_key;

    #[test]
    fn duplicate_keys_are_found_at_any_depth_and_only_in_objects() {
        assert_eq!(duplicate_key(r#"{"a":1,"a":2}"#).as_deref(), Some("a"));
        assert_eq!(
            duplicate_key(r#"{"x":{"b":1,"c":{"b":2},"b":3}}"#).as_deref(),
            Some("b")
        );
        assert_eq!(duplicate_key(r#"{"a":"a","b":["a","a"]}"#), None);
        assert_eq!(duplicate_key(r#"[{"a":1},{"a":1}]"#), None);
        assert_eq!(duplicate_key(r#"{"a":1,"a":2}"#).as_deref(), Some("a"));
        assert_eq!(
            duplicate_key(r#"{"q\"x":1,"q\"x":2}"#).as_deref(),
            Some("q\"x")
        );
    }
}
