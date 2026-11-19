//! Snapshots in a store (stores-next spec, section 2): the default layout
//! of a copied file, the provenance file written beside it
//! (`writfold-snapshot/1`, never changed) and the clone's work tree. The
//! flow and the decision are [`crate::write::snapshot`]. Tested, not
//! proved.
use crate::observe::Observation;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-snapshot/1";
/// The provenance file of `<file>` is `<file>.snapshot.json`.
pub const SUFFIX: &str = ".snapshot.json";

/// The work tree of the clone whose Git directory is `git_dir`, if any.
pub fn work_tree(git_dir: &Path) -> Option<PathBuf> {
    if git_dir.file_name()? == ".git" {
        return std::fs::canonicalize(git_dir.parent()?).ok();
    }
    let linked = std::fs::read_to_string(git_dir.join("gitdir")).ok()?;
    std::fs::canonicalize(Path::new(linked.trim()).parent()?).ok()
}

/// A path segment: lower-case letters and digits, runs of anything else as
/// one `-`, at most 40 characters.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c.is_ascii_alphanumeric() {
            true => out.push(c.to_ascii_lowercase()),
            false if !out.ends_with('-') => out.push('-'),
            false => {}
        }
    }
    let out = out.trim_matches('-');
    out.get(..40).unwrap_or(out).trim_matches('-').to_owned()
}

/// Where a copied file goes:
/// `snapshots/<date>-<source>-<locator slug>-<8 hex of its SHA-256>/<name>`.
pub fn layout(o: &Observation, name: &str, sha: &str) -> String {
    let date = o.observed.get(..10).unwrap_or(&o.observed);
    let locator = (o.locator.split_once("://")).map_or(o.locator.as_str(), |(_, r)| r);
    format!(
        "snapshots/{date}-{}-{}-{}/{name}",
        slug(&o.source),
        slug(locator),
        sha.get(..8).unwrap_or(sha)
    )
}

/// The provenance file's bytes: two-space JSON ending in a line break. It
/// says these bytes were handed to Writfold then, for this look; Writfold
/// did not fetch them.
pub fn provenance(o: &Observation, name: &str, sha: &str, size: usize) -> Vec<u8> {
    let value = serde_json::json!({
        "schema": SCHEMA, "file": name, "sha256": sha, "bytes": size,
        "source": o.source, "locator": o.locator, "revision": o.revision,
        "status": o.status.name(), "taken": o.observed, "run": o.run,
        "for": o.item.as_ref().map(|(g, i)| format!("{g}.{i}")), "note": o.note,
    });
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

/// The `sha256` a provenance file names.
pub fn named(bytes: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if value.get("schema")?.as_str()? != SCHEMA {
        return None;
    }
    value.get("sha256")?.as_str().map(str::to_owned)
}

/// `text`, safe for a POSIX shell.
pub fn quote(text: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "/._-:@=+,~".contains(c);
    match !text.is_empty() && text.chars().all(plain) {
        true => text.to_owned(),
        false => format!("'{}'", text.replace('\'', "'\\''")),
    }
}

#[cfg(test)]
mod tests {
    use super::{named, quote, slug};

    #[test]
    fn slugs_quotes_and_provenance() {
        assert_eq!(
            slug("https://Example.org/spec/v2?x=1"),
            "https-example-org-spec-v2-x-1"
        );
        assert_eq!(slug("--a--"), "a");
        assert_eq!(quote("snapshots/a-b/x.html"), "snapshots/a-b/x.html");
        assert_eq!(quote("it's here"), "'it'\\''s here'");
        assert_eq!(quote(""), "''");
        let text = br#"{"schema": "writfold-snapshot/1", "sha256": "ab"}"#;
        assert_eq!(named(text).as_deref(), Some("ab"));
        assert_eq!(named(br#"{"schema": "other", "sha256": "ab"}"#), None);
    }
}
