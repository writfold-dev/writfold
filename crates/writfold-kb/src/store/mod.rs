//! Attached stores: companion Git repositories of disposable evidence
//! (probe scripts, data pulls, review notes, reproducers) that the KB
//! cites and never curates, declared one per file in
//! `.writfold/stores/<name>.toml`:
//!
//! ```text
//! schema = "writfold-store/1"
//! name = "scratch"                 # what citations carry in repo=
//! url = "git@example.org:me/scratch.git"
//! role = "attached"                # cited, never curated
//! freshness = "pinned"             # the default; "head" keeps the rule of any repository
//! paths = ["~/src/scratch"]        # optional hints, tried in order on each machine
//! recorded_max_age = "30d"         # optional: use recorded verdicts where the clone is missing
//! ```
//!
//! A citation into a pinned store is judged at its own revision
//! ([`writfold_core::store::pinned`], proved), so reorganising the store
//! never makes sound evidence stale. Where the store's clone is on this
//! machine is [`writfold_core::store::resolve`] (proved): the local map
//! first, whatever its origin; then a `$WRITFOLD_KB_REPOS` root or a path
//! hint whose clone's `origin` is the declared URL. This module reads the
//! files, the clones' origins and normalises URLs (tested, not proved).
use crate::error::{KbError, Result};
use crate::stale::repos;
use crate::workspace::Workspace;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use writfold_core::store::{self as kernel, Candidate, Resolved};

pub mod check;
pub mod material;
pub mod snapshot;

/// Where store declarations live, under the KB root.
pub const DIR: &str = ".writfold/stores";
pub const SCHEMA: &str = "writfold-store/1";

/// How a store's citations are judged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// At the citation's own revision.
    Pinned,
    /// At the store's head, as for any repository.
    Head,
}

/// One declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Store {
    pub name: String,
    pub url: String,
    pub freshness: Freshness,
    pub paths: Vec<String>,
    pub description: Option<String>,
    /// How old a recorded verdict a reader without the clone may use, as
    /// written (`30d`); `None`: receipts are never used (#4).
    pub recorded_max_age: Option<String>,
}

impl Store {
    /// [`Store::recorded_max_age`] in seconds.
    pub fn recorded_seconds(&self) -> Option<i64> {
        match crate::time::ttl(Some(self.recorded_max_age.as_deref()?))? {
            crate::time::Ttl::Seconds(seconds) => Some(seconds),
            crate::time::Ttl::Never => None,
        }
    }

    /// The check-receipt kind its recorded verdicts are filed under: a
    /// name no declared kind can take (`@` is not in a kind name).
    pub fn receipt_kind(&self) -> String {
        format!("store@{}", self.name)
    }
}

/// Every declaration of a KB, by name.
#[derive(Clone, Debug, Default)]
pub struct Stores(BTreeMap<String, Store>);

impl Stores {
    pub fn get(&self, name: &str) -> Option<&Store> {
        self.0.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Store> {
        self.0.values()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Every declaration under the KB root, or a refusal naming the first file
/// that cannot be read. No directory means no stores.
pub fn read(root: &Path) -> Result<Stores> {
    let entries = match std::fs::read_dir(root.join(DIR)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Stores::default()),
        Err(e) => return Err(e.into()),
    };
    let mut names = Vec::new();
    for entry in entries {
        let file = entry?.file_name().to_string_lossy().into_owned();
        if let Some(name) = file.strip_suffix(".toml") {
            names.push(name.to_owned());
        }
    }
    names.sort();
    let mut stores = BTreeMap::new();
    for name in names {
        let path = format!("{DIR}/{name}.toml");
        let text = std::fs::read_to_string(root.join(&path))
            .map_err(|e| KbError::Refused(format!("{path}: {e}")))?;
        let store = check::check(&text, &name, &path)
            .map_err(|report| KbError::Definition(Box::new(report)))?;
        stores.insert(name, store);
    }
    Ok(Stores(stores))
}

/// A remote URL reduced to `host/path` for comparison: the scheme, a
/// user, a trailing `/` and `.git` dropped, the host lowercased, and the
/// `host:path` form read as `host/path`. A local path stays a path.
pub fn normalise(url: &str) -> String {
    let url = url.trim();
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, after)| after);
    let local = url.starts_with('/') || url.starts_with("file://");
    let rest = if local {
        rest.to_owned()
    } else if !url.contains("://") {
        rest.replacen(':', "/", 1)
    } else {
        rest.to_owned()
    };
    let rest = rest.trim_end_matches('/');
    let rest = rest
        .strip_suffix(".git")
        .unwrap_or(rest)
        .trim_end_matches('/');
    match rest.split_once('/') {
        Some((host, path)) if !local => format!("{}/{path}", host.to_ascii_lowercase()),
        _ => rest.to_owned(),
    }
}

/// The `url` of `[remote "origin"]` in a Git config file's text.
pub fn origin_in(config: &str) -> Option<String> {
    let mut inside = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            let header: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            inside = header == "[remote\"origin\"]";
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim().eq_ignore_ascii_case("url")
        {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// The origin URL of the clone whose Git directory is `git_dir`.
pub fn origin(git_dir: &Path) -> Option<String> {
    let config = repos::common_dir(git_dir).join("config");
    origin_in(&std::fs::read_to_string(config).ok()?)
}

/// `~/x` against `$HOME`.
fn expand(hint: &str) -> PathBuf {
    match (hint.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(hint),
    }
}

/// Where a declared store resolves on this machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Found {
    /// The Git directory to read, when one may be read.
    pub git_dir: Option<PathBuf>,
    /// `map`, `search-root`, `hint`, `mismatch` or `missing`.
    pub how: &'static str,
    /// Set when the clone read (through the map) or the clone found (a
    /// mismatch) has another origin: that origin.
    pub other_origin: Option<String>,
}

fn candidate(dir: Option<PathBuf>, want: &str) -> (Candidate, Option<(PathBuf, String)>) {
    let Some(git_dir) = dir.as_deref().and_then(repos::git_dir) else {
        return (Candidate::None, None);
    };
    // The store is the clone when any remote is at the declared URL (#198).
    let urls = crate::declared::remote_urls(&git_dir);
    let at = urls.iter().find(|u| normalise(u) == want);
    let same = at.is_some();
    let had = match at {
        Some(u) => u.clone(),
        None => origin(&git_dir)
            .or_else(|| urls.first().cloned())
            .unwrap_or_default(),
    };
    (Candidate::Clone(same), Some((git_dir, had)))
}

/// Resolve a declared store: the local map, a search root, then the first
/// path hint that names a clone (preferring one with the declared origin).
pub fn resolve(ws: Option<&Workspace>, store: &Store) -> Found {
    let want = normalise(&store.url);
    let map = candidate(repos::mapped(ws, &store.name), &want);
    let search = candidate(repos::searched(&store.name), &want);
    let mut hints: Vec<_> = store
        .paths
        .iter()
        .map(|h| candidate(Some(expand(h)), &want))
        .collect();
    let pick = hints
        .iter()
        .position(|(c, _)| matches!(c, Candidate::Clone(true)))
        .or_else(|| {
            hints
                .iter()
                .position(|(c, _)| matches!(c, Candidate::Clone(_)))
        });
    let hint = match pick {
        Some(index) => hints.swap_remove(index),
        None => (Candidate::None, None),
    };
    let other = |found: &Option<(PathBuf, String)>| found.as_ref().map(|(_, o)| o.clone());
    match kernel::resolve(&map.0, &search.0, &hint.0) {
        Resolved::Found { via, flagged } => {
            let (how, found) = match via {
                kernel::Via::Map => ("map", &map.1),
                kernel::Via::SearchRoot => ("search-root", &search.1),
                kernel::Via::Hint => ("hint", &hint.1),
            };
            Found {
                git_dir: found.as_ref().map(|(dir, _)| dir.clone()),
                how,
                other_origin: if flagged { other(found) } else { None },
            }
        }
        Resolved::Mismatch => Found {
            git_dir: None,
            how: "mismatch",
            other_origin: other(&search.1).or_else(|| other(&hint.1)),
        },
        Resolved::Missing => Found {
            git_dir: None,
            how: "missing",
            other_origin: None,
        },
    }
}

#[cfg(test)]
mod tests;
