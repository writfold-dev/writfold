//! Retained gate evidence (RFC 2026-11-24-retained-gate-evidence): a command
//! gate may declare `retain = ["report.json", …]`, files its command writes
//! into `{artifacts}`. After the command ends each is read and, when it is
//! a regular file within the caps, published with the receipt under
//! `runs/<run>/evidence/<receipt-stem>/<name>`. Whether every declared file
//! was kept is the kernel's input ([`writfold_core::gate::outcome_kept`]);
//! everything here is tested IO and plumbing.
//!
//! On every read a receipt that records retained files is intact only when
//! each is present with the recorded size and digest ([`intact`]); a
//! receipt that is not intact is invalid, so it is never bound.
use crate::defs::{Check, Field};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use writfold_core::gate::Kept;

/// The run directory that holds retained evidence, one directory per
/// receipt stem.
pub const DIR: &str = "evidence";
/// At most this many files per gate.
pub const MAX_FILES: usize = 8;
/// At most this many bytes per retained file.
pub const MAX_FILE: u64 = 1024 * 1024;
/// At most this many bytes retained by one receipt.
pub const MAX_TOTAL: u64 = 4 * 1024 * 1024;

/// A file name `retain` may hold: 1 to 64 of `A-Z a-z 0-9 . _ -`, not
/// starting with `.`.
pub fn well_named(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// `retain`: for a command gate on a repository subject only.
pub fn check(c: &mut Check<'_>, class: &str) -> Option<Vec<String>> {
    let candidate = matches!(c.doc.get("subject").map(|e| &e.value),
        Some(crate::defs::Slot::Value(v)) if matches!(&v.value, crate::defs::Value::Str(t) if t.text == "candidate"));
    if candidate || !matches!(class, "mechanical" | "observed") {
        let why = "`retain` is for mechanical and observed gates on a repository".to_owned();
        return (!c.not_here("retain", why, "only a repository command gate takes it"))
            .then(Vec::new);
    }
    let list = match c.list("retain") {
        Field::Absent => return Some(Vec::new()),
        Field::Bad => return None,
        Field::Got(list) => list,
    };
    let mut ok = true;
    if list.value.len() > MAX_FILES {
        let why = format!("`retain` names at most {MAX_FILES} files");
        c.error("value.form", Some(list.span), why)
            .label("too many files");
        ok = false;
    }
    for (n, name) in list.value.iter().enumerate() {
        if !well_named(name.str()) {
            let why = format!(
                "retain {:?} must be a file name in {{artifacts}} (A-Z a-z 0-9 . _ -, not starting with .)",
                name.str()
            );
            c.error("value.form", Some(name.span), why)
                .label("not a plain file name");
            ok = false;
        } else if list.value[..n].iter().any(|o| o.str() == name.str()) {
            let why = format!("retain names {:?} twice", name.str());
            c.error("value.form", Some(name.span), why)
                .label("repeated");
            ok = false;
        }
    }
    ok.then(|| list.value.into_iter().map(|t| t.value.text).collect())
}

/// One retained file as a receipt records it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Retained {
    pub name: String,
    pub bytes: u64,
    pub digest: String,
}

/// What `gate run` kept of the declared files.
#[derive(Default)]
pub struct Gathered {
    /// Kept files, in declared order, with their bytes.
    pub kept: Vec<(String, Vec<u8>)>,
    /// Declared files that were absent, not regular files, or over a cap.
    pub missing: Vec<String>,
}

impl Gathered {
    /// The kernel's view: nothing declared, all kept, or some missing.
    pub fn kept(&self) -> Kept {
        kept_of(
            self.kept.is_empty() && self.missing.is_empty(),
            self.missing.is_empty(),
        )
    }

    /// The receipt's fields: `retained` and `result.evidence_missing`.
    pub fn record(&self, value: &mut Value) {
        let retained: Vec<Value> = (self.retained().iter())
            .map(|r| json!({"name": r.name, "bytes": r.bytes, "digest": r.digest}))
            .collect();
        if let Value::Object(map) = value {
            map.insert("retained".into(), Value::Array(retained));
        }
        if let Some(Value::Object(result)) = value.get_mut("result") {
            result.insert("evidence_missing".into(), json!(self.missing));
        }
    }

    /// The receipt's record of each kept file.
    pub fn retained(&self) -> Vec<Retained> {
        self.kept
            .iter()
            .map(|(name, bytes)| Retained {
                name: name.clone(),
                bytes: bytes.len() as u64,
                digest: sha(bytes),
            })
            .collect()
    }
}

fn kept_of(none: bool, all: bool) -> Kept {
    match (none, all) {
        (true, true) => Kept::Nothing,
        (false, true) => Kept::All,
        (true | false, false) => Kept::Missing,
    }
}

/// A receipt's retained files and the kernel's view of them, as read
/// back. A receipt of a gate without `retain` has neither field.
pub fn read(value: &Value) -> Result<(Vec<Retained>, Kept), String> {
    let missing = match value.get("result").and_then(|r| r.get("evidence_missing")) {
        None => Vec::new(),
        Some(Value::Array(items)) => (items.iter())
            .map(|i| i.as_str().map(str::to_owned))
            .collect::<Option<Vec<String>>>()
            .ok_or("result.evidence_missing is not a list of names")?,
        Some(_) => return Err("result.evidence_missing is not a list of names".into()),
    };
    let retained = match value.get("retained") {
        None => Vec::new(),
        Some(Value::Array(items)) => (items.iter())
            .map(|i| {
                let name = i
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|n| well_named(n));
                let bytes = i.get("bytes").and_then(Value::as_u64);
                let digest = i.get("digest").and_then(Value::as_str);
                Some(Retained {
                    name: name?.to_owned(),
                    bytes: bytes?,
                    digest: digest?.to_owned(),
                })
            })
            .collect::<Option<Vec<Retained>>>()
            .ok_or("retained is not a list of name, bytes and digest")?,
        Some(_) => return Err("retained is not a list".into()),
    };
    let kept = kept_of(
        retained.is_empty() && missing.is_empty(),
        missing.is_empty(),
    );
    Ok((retained, kept))
}

pub fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// Read each declared file from the `{artifacts}` directory. A symlink,
/// anything but a regular file, an unreadable file or one past a cap is
/// missing; nothing is followed out of the directory.
pub fn gather(artifacts: &Path, names: &[String]) -> Gathered {
    let mut out = Gathered::default();
    let mut total = 0u64;
    for name in names {
        let path = artifacts.join(name);
        let regular = std::fs::symlink_metadata(&path)
            .is_ok_and(|m| m.file_type().is_file() && m.len() <= MAX_FILE);
        let bytes = regular.then(|| std::fs::read(&path).ok()).flatten();
        match bytes {
            Some(b) if (b.len() as u64) <= MAX_FILE && total + b.len() as u64 <= MAX_TOTAL => {
                total += b.len() as u64;
                out.kept.push((name.clone(), b));
            }
            _ => out.missing.push(name.clone()),
        }
    }
    out
}

/// Where one retained file lives, KB-relative.
pub fn path(run_dir: &str, stem: &str, name: &str) -> String {
    format!("{run_dir}/{DIR}/{stem}/{name}")
}

/// Whether every file a receipt records as retained is present under the
/// run directory `run` (on disk) with the recorded size and digest. The
/// error is the reason the receipt is invalid.
pub fn intact(run: &Path, stem: &str, retained: &[Retained]) -> Result<(), String> {
    for r in retained {
        let at = run.join(DIR).join(stem).join(&r.name);
        let regular = std::fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_file());
        let bytes = regular.then(|| std::fs::read(&at).ok()).flatten();
        match bytes {
            None => return Err(format!("retained evidence {} is not here", r.name)),
            Some(b) if b.len() as u64 != r.bytes || sha(&b) != r.digest => {
                return Err(format!(
                    "retained evidence {} does not match its receipt",
                    r.name
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::well_named;

    #[test]
    fn names_are_plain_files() {
        for good in ["report.json", "a", "x_1-2.txt", "R.JSON"] {
            assert!(well_named(good), "{good}");
        }
        for bad in ["", ".hidden", "a/b", "../x", "a b", "a\\b", &"x".repeat(65)] {
            assert!(!well_named(bad), "{bad}");
        }
    }
}
