//! `writfold trial evidence`: read back evidence retained in this trial state,
//! so a later session can cite and read exact bytes without searching again.
//!
//! Every entry is checked before it is shown: its descriptor decodes,
//! re-fingerprints to exactly its file name, and its retained bytes match the
//! descriptor's selection digest. `list` counts entries that fail as
//! `refused`; `show` refuses them. Reading writes nothing.
//!
//! The durable identity is the full binding digest. A 12-hex abbreviation in
//! a `list` row and a [`label`] are local interaction conveniences resolved
//! against this state only; `show` prints the full digest they resolved to,
//! and only that digest is a citation.
mod cli;
pub mod label;
mod show;
#[cfg(test)]
mod tests;

use super::{Result, limits};
use std::fs;
use std::path::{Path, PathBuf};
use writfold_wire::citation::{binding_fingerprint, decode_descriptor};

/// One verified retained binding.
pub struct Entry {
    pub binding: String,
    pub path: String,
    pub commit: String,
    pub range: (u64, u64),
    /// The 1-based line of `range.0`, recorded at retention; `None` for
    /// evidence retained before the line was recorded.
    pub line: Option<u64>,
    pub bytes: Vec<u8>,
}

pub(super) fn dir(state: &Path) -> PathBuf {
    state.join("evidence")
}

fn is_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Load and verify `binding`; a refusal names what failed.
pub fn load(state: &Path, binding: &str) -> Result<Entry> {
    if !is_digest(binding) {
        return Err(format!("{binding} is not a binding digest").into());
    }
    let dir = dir(state);
    let encoded = fs::read(dir.join(format!("{binding}.json")))
        .map_err(|e| format!("binding {binding} is not retained in this state: {e}"))?;
    let bytes = fs::read(dir.join(format!("{binding}.bytes")))
        .map_err(|e| format!("binding {binding}: retained bytes missing: {e}"))?;
    let descriptor = decode_descriptor(&encoded, limits()?)?;
    if binding_fingerprint(&descriptor, limits()?)?.as_str() != binding {
        return Err(format!("binding {binding}: descriptor does not fingerprint to it").into());
    }
    if descriptor.bind(&bytes).is_err() {
        return Err(format!(
            "binding {binding}: retained bytes do not match the descriptor digest (tampered); refusing"
        )
        .into());
    }
    let line = fs::read_to_string(dir.join(format!("{binding}.line")))
        .ok()
        .and_then(|l| l.trim().parse().ok());
    let reference = descriptor.source().reference();
    let selector = descriptor.selector();
    Ok(Entry {
        binding: binding.to_owned(),
        path: reference.path().clone(),
        commit: reference.revision().as_str().to_owned(),
        range: (selector.first().get(), selector.last().get()),
        line,
        bytes,
    })
}

/// Every retained binding digest in this state, sorted.
pub fn bindings(state: &Path) -> Result<Vec<String>> {
    if !state.join("git.sha256").is_file() {
        return Err(format!("{} is not a trial state", state.display()).into());
    }
    let read = match fs::read_dir(dir(state)) {
        Ok(read) => read,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut out = Vec::new();
    for item in read {
        let name = item?.file_name();
        if let Some(binding) = name.to_str().and_then(|n| n.strip_suffix(".json"))
            && is_digest(binding)
        {
            out.push(binding.to_owned());
        }
    }
    out.sort();
    Ok(out)
}

/// Resolve a full digest, a label, or a unique digest prefix of at least 8
/// hex digits to a full digest. The second value names how it resolved when
/// that was not a full digest.
pub fn resolve(state: &Path, text: &str) -> Result<(String, Option<String>)> {
    if is_digest(text) {
        return Ok((text.to_owned(), None));
    }
    if let Some(binding) = label::get(state, text)? {
        return Ok((binding, Some(format!("label \"{text}\""))));
    }
    if text.len() >= 8 && text.len() < 64 && is_digest(&format!("{text:0<64}")) {
        let found: Vec<String> = bindings(state)?
            .into_iter()
            .filter(|b| b.starts_with(text))
            .collect();
        return match found.as_slice() {
            [one] => Ok((one.clone(), Some(format!("prefix {text}")))),
            [] => Err(format!("no retained binding starts with {text}").into()),
            _ => Err(format!("{text} is ambiguous: {} bindings", found.len()).into()),
        };
    }
    Err(format!("{text} is neither a binding digest, a label nor a digest prefix").into())
}

/// A string literal for text that would be ambiguous raw.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

const EXCERPT_CHARS: usize = 160;

/// The first line of the retained bytes, cut at `EXCERPT_CHARS`, with
/// `…+NB` counting the retained bytes not shown.
fn excerpt(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let first = text.split_inclusive('\n').next().unwrap_or("");
    let line = first.strip_suffix('\n').unwrap_or(first);
    let shown: String = line.chars().take(EXCERPT_CHARS).collect();
    let hidden = bytes
        .len()
        .saturating_sub(shown.len() + (first.len() - line.len()));
    let raw = !shown.starts_with('"')
        && !shown.contains('…')
        && !shown.chars().any(|c| c.is_control() && c != '\t')
        && std::str::from_utf8(bytes).is_ok();
    let mut out = if raw { shown.clone() } else { quote(&shown) };
    if hidden > 0 {
        out.push_str(&format!("…+{hidden}B"));
    }
    out
}

/// One compact row: `binding12 path@commit7:line: excerpt`.
pub fn row(entry: &Entry) -> String {
    let commit = entry.commit.get(..7).unwrap_or(&entry.commit);
    let line = entry.line.map_or("?".to_owned(), |l| l.to_string());
    let path = if entry.path.contains(':') || entry.path.contains('@') {
        quote(&entry.path)
    } else {
        entry.path.clone()
    };
    format!(
        "{} {path}@{commit}:{line}: {}",
        &entry.binding[..12],
        excerpt(&entry.bytes)
    )
}

/// The page size of a `list` that names none (CLI and `serve` alike).
pub const DEFAULT_LIMIT: usize = 100;

/// The filters and page of one `list`.
pub struct Query<'a> {
    pub path: Option<&'a str>,
    pub commit: Option<&'a str>,
    pub limit: usize,
}

/// Rows of the matching verified entries, ordered by path, commit and start,
/// and the trailer `# count=M shown=S omitted=O[ refused=R]`.
pub fn list(state: &Path, query: &Query<'_>) -> Result<(Vec<String>, String)> {
    let mut entries = Vec::new();
    let mut refused = 0usize;
    for binding in bindings(state)? {
        match load(state, &binding) {
            Ok(entry) => entries.push(entry),
            Err(_) => refused += 1,
        }
    }
    entries.retain(|e| {
        query.path.is_none_or(|p| e.path.starts_with(p))
            && query.commit.is_none_or(|c| e.commit.starts_with(c))
    });
    entries.sort_by(|a, b| (&a.path, &a.commit, a.range).cmp(&(&b.path, &b.commit, b.range)));
    let rows: Vec<String> = entries.iter().take(query.limit).map(row).collect();
    let mut trailer = format!(
        "# count={} shown={} omitted={}",
        entries.len(),
        rows.len(),
        entries.len() - rows.len()
    );
    if refused > 0 {
        trailer.push_str(&format!(" refused={refused}"));
    }
    Ok((rows, trailer))
}

pub use cli::main;
pub use show::{Shown, show};
