//! Declared repositories (#197, RFC 2026-11-27-declared-repositories): what
//! a cited repository is and which line of it counts as now, in Git text,
//! one per file in `.writfold/repos/<slug>.toml`:
//!
//! ```text
//! schema = "writfold-repo/1"
//! name = "org/service"             # what citations carry in repo=
//! url = "https://example.org/org/service.git"
//! mirrors = ["git@example.org:org/service.git"]   # optional, the same repository
//! aliases = ["service"]            # optional, other repo= spellings
//! ref = "main"                     # optional; default: the remote's default branch
//! ```
//!
//! With a declaration the resolver takes only a remote at the declared URL
//! (or a mirror) as a candidate ([`writfold_core::target::choose_in`],
//! proved). Reading and checking the files, the alias lookup and the strings
//! are tested, not proved.
use crate::error::{KbError, Result};
use std::path::Path;

pub mod check;

/// Where declarations live, under the KB root.
pub const DIR: &str = ".writfold/repos";
pub const SCHEMA: &str = "writfold-repo/1";

/// One declaration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Declared {
    pub name: String,
    pub url: String,
    pub mirrors: Vec<String>,
    pub aliases: Vec<String>,
    /// A branch name; `None`: the remote's default branch.
    pub reference: Option<String>,
    pub description: Option<String>,
    /// The file, as rows show it.
    pub path: String,
}

impl Declared {
    /// Whether a token's `repo=` names this repository.
    pub fn names(&self, repo: &str) -> bool {
        self.name == repo || self.aliases.iter().any(|a| a == repo)
    }

    /// The URLs a remote may be at: the URL and its mirrors.
    pub fn urls(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.url.as_str()).chain(self.mirrors.iter().map(String::as_str))
    }
}

/// Every declaration, sorted by file; a name or alias declared twice, or
/// one a store also declares, refuses the set.
pub fn read(root: &Path) -> Result<Vec<Declared>> {
    let entries = match std::fs::read_dir(root.join(DIR)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut slugs: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".toml").map(str::to_owned)
        })
        .filter(|s| !s.starts_with('.'))
        .collect();
    slugs.sort();
    let mut out: Vec<Declared> = Vec::new();
    for slug in slugs {
        let path = format!("{DIR}/{slug}.toml");
        let text = std::fs::read_to_string(root.join(&path))
            .map_err(|e| KbError::Refused(format!("{path}: {e}")))?;
        let found = check::check(&text, &slug, &path)
            .map_err(|report| KbError::Definition(Box::new(report)))?;
        out.push(found);
    }
    if let Some(why) = clash(root, &out)? {
        return Err(KbError::Refused(why));
    }
    Ok(out)
}

/// A name declared twice, here or as a store.
fn clash(root: &Path, all: &[Declared]) -> Result<Option<String>> {
    let stores = crate::store::read(root)?;
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for d in all {
        for n in std::iter::once(d.name.as_str()).chain(d.aliases.iter().map(String::as_str)) {
            if let Some((_, other)) = seen.iter().find(|(m, _)| *m == n) {
                return Ok(Some(format!("{}: {n} is also declared in {other}", d.path)));
            }
            if stores.get(n).is_some() {
                return Ok(Some(format!(
                    "{}: {n} is also declared as a store in {}/{n}.toml",
                    d.path,
                    crate::store::DIR
                )));
            }
            seen.push((n, &d.path));
        }
    }
    Ok(None)
}

/// The URLs of every remote of the clone whose Git directory is `git_dir`.
pub fn remote_urls(git_dir: &Path) -> Vec<String> {
    let config = crate::stale::repos::common_dir(git_dir).join("config");
    let text = std::fs::read_to_string(config).unwrap_or_default();
    crate::stale::target::remotes(&text)
        .into_iter()
        .filter_map(|(_, url)| url)
        .collect()
}

/// Whether some remote of the clone is at `url` (normalised).
pub fn has_remote_at(git_dir: &Path, url: &str) -> bool {
    let want = crate::stale::target::normalize(url);
    want.is_some()
        && remote_urls(git_dir)
            .iter()
            .any(|u| crate::stale::target::normalize(u) == want)
}

/// The declaration one of the clone's remotes is at, if any (#198).
pub fn of_clone<'a>(all: &'a [Declared], git_dir: &Path) -> Option<&'a Declared> {
    let urls: Vec<Option<String>> = remote_urls(git_dir)
        .iter()
        .map(|u| crate::stale::target::normalize(u))
        .collect();
    all.iter().find(|d| {
        d.urls()
            .map(crate::stale::target::normalize)
            .any(|u| u.is_some() && urls.contains(&u))
    })
}

/// The name a clone goes by when no map names it: its declaration's, else
/// the last segment of its first remote's URL, else `None` (#198).
pub fn clone_name(root: &Path, git_dir: &Path) -> Option<String> {
    let all = read(root).unwrap_or_default();
    if let Some(d) = of_clone(&all, git_dir) {
        return Some(d.name.clone());
    }
    remote_name(git_dir)
}

/// The last segment of the clone's first remote's URL.
pub fn remote_name(git_dir: &Path) -> Option<String> {
    remote_urls(git_dir).first().and_then(|url| {
        let last = url.trim_end_matches('/').rsplit(['/', ':']).next()?;
        let name = last.trim_end_matches(".git");
        (!name.is_empty()).then(|| name.to_owned())
    })
}

/// The commit an attached store is read at: the line of the remote at its
/// declared URL when the clone has one, else the clone's own rule (#198).
pub fn store_commit(
    git: &writfold_runtime::git::GitExecutable<'_>,
    git_dir: &Path,
    store: &crate::store::Store,
) -> Result<Option<writfold_domain::GitObjectId>> {
    let as_declared = Declared {
        name: store.name.clone(),
        url: store.url.clone(),
        ..Declared::default()
    };
    let ask = crate::stale::target::Asked {
        repo: &store.name,
        url: Some(&store.url),
        pin: None,
        named: true,
        declared: has_remote_at(git_dir, &store.url).then_some(&as_declared),
    };
    Ok(crate::stale::target::resolve(git, git_dir, &ask)?.commit)
}

/// The declaration a token's `repo=` names, if any.
pub fn find<'a>(all: &'a [Declared], repo: &str) -> Option<&'a Declared> {
    all.iter().find(|d| d.names(repo))
}
