//! The `paths` key of a guard rule: which file a tool call edits, as a
//! path relative to that file's repository. Read from the event alone
//! (`tool_input.file_path`, else `tool_input.notebook_path`) and the
//! filesystem (the nearest ancestor holding `.git`, file or directory);
//! nothing is opened and nothing is spawned, so a rule with `paths` is
//! decided before the index is.
use serde_json::{Map, Value};
use std::path::{Component, Path, PathBuf};
use writfold_core::scope::in_scope;

/// The tool-input fields that name the edited file, in order.
const FIELDS: [&str; 2] = ["file_path", "notebook_path"];

/// `path` made absolute against `cwd` and with `.` and `..` resolved by
/// name: the file may not exist yet (a `Write`), so nothing is resolved
/// through the filesystem.
fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in cwd.join(path).components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The edited file of a tool call, relative to the nearest directory at
/// or above it that holds `.git`, `/`-separated. `None` when the call
/// names no file or the file is in no repository.
pub fn target(input: &Map<String, Value>, cwd: &Path) -> Option<String> {
    let tool_input = input.get("tool_input")?;
    let named = FIELDS
        .iter()
        .find_map(|f| tool_input.get(*f).and_then(Value::as_str))
        .filter(|p| !p.is_empty())?;
    let file = absolute(cwd, Path::new(named));
    let top = file.ancestors().skip(1).find(|d| d.join(".git").exists())?;
    let rest = file.strip_prefix(top).ok()?;
    let parts: Vec<String> = rest
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some(parts.join("/"))
}

/// Whether a rule's `paths` admit the call's file: always with no
/// `paths`; otherwise only a named file in a repository that some
/// pattern matches and no `!` pattern does.
pub fn admits(paths: &[String], file: Option<&str>) -> bool {
    if paths.is_empty() {
        return true;
    }
    let Some(file) = file else {
        return false;
    };
    let patterns: Vec<&str> = paths.iter().map(String::as_str).collect();
    in_scope(&patterns, file)
}
