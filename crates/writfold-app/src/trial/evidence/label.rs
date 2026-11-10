//! Local labels: a mutable, per-state name for a retained binding.
//!
//! A label lives in `<state>/labels/<name>`, apart from the immutable
//! evidence, and holds one full binding digest. Setting it again moves it.
//! Following the shared-client rule for scoped aliases, a label is an
//! interaction convenience only: it is never exported as durable identity,
//! never written into a descriptor or report, and `show` prints the digest it
//! resolved to, which is what a citation names.
use super::super::Result;
use super::is_digest;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

fn dir(state: &Path) -> PathBuf {
    state.join("labels")
}

/// A label is 1..=64 of `[a-z0-9._-]`, starts with a letter or digit, and is
/// not 8 or more hex digits alone (those read as a digest prefix).
pub fn check(name: &str) -> Result<()> {
    let charset = name
        .bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'));
    let first = name
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric());
    let hexlike = name.len() >= 8 && name.bytes().all(|b| b.is_ascii_hexdigit());
    if name.len() > 64 || !charset || !first || hexlike {
        return Err(format!(
            "label {name:?} refused: use 1-64 of [a-z0-9._-], starting with a letter or digit, \
not 8+ hex digits alone"
        )
        .into());
    }
    Ok(())
}

/// Point `name` at `binding`, replacing any earlier target (write then
/// rename, so a reader sees the old or the new digest, never a mix).
pub fn set(state: &Path, name: &str, binding: &str) -> Result<()> {
    check(name)?;
    if !is_digest(binding) {
        return Err(format!("{binding} is not a binding digest").into());
    }
    super::load(state, binding)?;
    let dir = dir(state);
    fs::create_dir_all(&dir)?;
    let temporary = dir.join(format!(".{name}.{}", std::process::id()));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(format!("{binding}\n").as_bytes())?;
    file.sync_all()?;
    fs::rename(&temporary, dir.join(name))?;
    Ok(())
}

/// The digest `name` points at, if it is a label in this state.
pub fn get(state: &Path, name: &str) -> Result<Option<String>> {
    if check(name).is_err() {
        return Ok(None);
    }
    match fs::read_to_string(dir(state).join(name)) {
        Ok(text) if is_digest(text.trim_end()) => Ok(Some(text.trim_end().to_owned())),
        Ok(_) => Err(format!("label {name} does not hold a binding digest").into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// `name -> digest` for every label, sorted by name.
pub fn render(state: &Path) -> Result<String> {
    let mut rows = Vec::new();
    if let Ok(read) = fs::read_dir(dir(state)) {
        for item in read {
            let name = item?.file_name().to_string_lossy().into_owned();
            if let Some(binding) = get(state, &name)? {
                rows.push(format!("{name} -> {binding}"));
            }
        }
    }
    rows.sort();
    rows.push(format!(
        "# labels={} (local to this state; cite the digest)",
        rows.len()
    ));
    Ok(rows.join("\n"))
}
