//! The placeholders a gate's `argv` may hold, and the files a definition
//! names beside itself.
//!
//! - `{artifacts}`: a fresh local directory for the command's own output
//!   files (repository gates only).
//! - `{gate}`: the directory holding the definition (`.writfold/gates/` of
//!   the KB root or of the subject repository), so a shared gate can ship
//!   its own script. It is followed by nothing, or by `/` and a path inside
//!   that directory; a named file must exist there, and its bytes enter the
//!   definition digest, so editing the script makes receipts stale.
//! - `{candidate}`: the delivery candidate's payload file (candidate gates
//!   only, where it is required).
//!
//! Any other `{name}` of lower-case letters and `_` is refused, so a typo
//! cannot pass through as text. Expansion is one pass: text a placeholder
//! expands to is never expanded again. Nothing here runs anything: reading
//! a definition only reads files.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const ARTIFACTS: &str = "{artifacts}";
pub const GATE: &str = "{gate}";
pub const CANDIDATE: &str = "{candidate}";
const KNOWN: [&str; 3] = [ARTIFACTS, GATE, CANDIDATE];

/// The values the placeholders expand to for one evaluation.
pub struct Places<'a> {
    pub gate: &'a Path,
    pub artifacts: Option<&'a Path>,
    pub candidate: Option<&'a Path>,
}

/// One placeholder found in an argument: its text and byte offset.
fn found(arg: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(open) = arg[from..].find('{').map(|i| i + from) {
        let Some(close) = arg[open..].find('}').map(|i| i + open) else {
            break;
        };
        let name = &arg[open + 1..close];
        if !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
            out.push((open, &arg[open..=close]));
            from = close + 1;
        } else {
            from = open + 1;
        }
    }
    out
}

/// The placeholders `arg` uses, or why it is refused.
pub fn used(arg: &str) -> Result<Vec<&'static str>, String> {
    let mut out = Vec::new();
    for (at, text) in found(arg) {
        let Some(known) = KNOWN.iter().find(|k| **k == text) else {
            return Err(format!(
                "{text} is not a placeholder (only {})",
                KNOWN.join(", ")
            ));
        };
        if *known == GATE {
            gate_file(&arg[at + GATE.len()..])?;
        }
        out.push(*known);
    }
    Ok(out)
}

/// What follows `{gate}`: nothing, or `/` and a path inside the gate
/// directory (no `..`, no empty segment, no other placeholder).
pub fn gate_file(rest: &str) -> Result<Option<&str>, String> {
    if rest.is_empty() {
        return Ok(None);
    }
    match rest.strip_prefix('/') {
        Some(path) if contained(path) && !path.contains('{') => Ok(Some(path)),
        _ => Err(format!(
            "{GATE} must end its argument or be followed by / and a path inside the gate directory (got {GATE}{rest})"
        )),
    }
}

/// A relative path that stays inside its root.
pub fn contained(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains(['\\', '\0'])
        && path.split('/').all(|part| part != ".." && !part.is_empty())
}

/// `relative` resolved inside `dir`: a file whose real path (symbolic
/// links followed) is still inside `dir`.
pub fn resolve(dir: &Path, relative: &str) -> Result<PathBuf, String> {
    if !contained(relative) {
        return Err(format!(
            "{relative} is not a relative path inside {}",
            dir_name(dir)
        ));
    }
    let base = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir_name(dir)))?;
    let real = std::fs::canonicalize(dir.join(relative))
        .map_err(|_| format!("{relative} is not a file beside the definition"))?;
    if !real.starts_with(&base) {
        return Err(format!("{relative} escapes the definition's directory"));
    }
    if !real.is_file() {
        return Err(format!("{relative} is not a file beside the definition"));
    }
    Ok(real)
}

fn dir_name(dir: &Path) -> String {
    dir.file_name().map_or_else(
        || dir.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// `sha256:<hex>` of a file's bytes.
pub fn digest_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(&bytes))
    ))
}

/// Every file `argv` names through `{gate}/…`, with its digest, in order.
pub fn gate_files(argv: &[String], dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    for arg in argv {
        for (at, text) in found(arg) {
            if text != GATE {
                continue;
            }
            if let Some(path) = gate_file(&arg[at + GATE.len()..])? {
                let real = resolve(dir, path)?;
                if !out.iter().any(|(p, _)| p == path) {
                    out.push((path.to_owned(), digest_file(&real)?));
                }
            }
        }
    }
    Ok(out)
}

/// The argv with every placeholder replaced, in one pass. A placeholder
/// with no value here is left as written (the definition check refuses
/// those before anything runs).
pub fn expand(argv: &[String], places: &Places<'_>) -> Vec<String> {
    let value = |text: &str| -> Option<String> {
        let path = match text {
            GATE => Some(places.gate),
            ARTIFACTS => places.artifacts,
            CANDIDATE => places.candidate,
            _ => None,
        };
        path.map(|p| p.to_string_lossy().into_owned())
    };
    argv.iter()
        .map(|arg| {
            let mut out = String::new();
            let mut from = 0;
            for (at, text) in found(arg) {
                out.push_str(&arg[from..at]);
                out.push_str(&value(text).unwrap_or_else(|| text.to_owned()));
                from = at + text.len();
            }
            out.push_str(&arg[from..]);
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Places, contained, expand, gate_files, resolve, used};
    use std::path::Path;

    #[test]
    fn placeholders_are_known_and_expand_once() {
        assert_eq!(used("{gate}/check.sh"), Ok(vec!["{gate}"]));
        assert_eq!(used("--file={candidate}"), Ok(vec!["{candidate}"]));
        assert_eq!(used("{}"), Ok(vec![]));
        assert_eq!(used("{1}"), Ok(vec![]));
        assert!(used("{receipt}").is_err());
        for bad in [
            "{gate}x",
            "{gate}/../up.sh",
            "{gate}/a//b",
            "{gate}/",
            "{gate}/{candidate}",
        ] {
            assert!(used(bad).is_err(), "{bad}");
        }
        let places = Places {
            gate: Path::new("/kb/.writfold/gates"),
            artifacts: None,
            candidate: Some(Path::new("/r/{gate}/candidate.md")),
        };
        let argv = vec![
            "{gate}/a.sh".to_owned(),
            "--in={candidate}".to_owned(),
            "{artifacts}".to_owned(),
        ];
        assert_eq!(
            expand(&argv, &places),
            [
                "/kb/.writfold/gates/a.sh",
                "--in=/r/{gate}/candidate.md",
                "{artifacts}"
            ]
        );
        assert!(
            contained("a/b.md") && !contained("../a") && !contained("/a") && !contained("a\\b")
        );
    }

    #[test]
    fn gate_files_stay_inside_the_directory() -> Result<(), String> {
        let base = std::env::temp_dir().join(format!("writfold-places-{}", std::process::id()));
        let dir = base.join("gates");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("check.sh"), "exit 0\n").map_err(|e| e.to_string())?;
        std::fs::write(base.join("outside.sh"), "exit 0\n").map_err(|e| e.to_string())?;
        let linked = std::os::unix::fs::symlink(base.join("outside.sh"), dir.join("link.sh"));
        let files = gate_files(&["{gate}/check.sh".into()], &dir)?;
        assert_eq!(files.len(), 1);
        assert!(gate_files(&["{gate}/missing.sh".into()], &dir).is_err());
        assert!(resolve(&dir, "../outside.sh").is_err());
        if linked.is_ok() {
            let refused = resolve(&dir, "link.sh");
            assert!(refused.is_err_and(|e| e.contains("escapes")));
        }
        let _ = std::fs::remove_dir_all(&base);
        Ok(())
    }
}
