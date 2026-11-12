//! Finding a local clone of a cited repository. v1 never clones or fetches.
//!
//! In order: the local repository map (`<git dir>/writfold/repos`, lines of
//! `<name> <path>`), the search roots in `$WRITFOLD_KB_REPOS` (`:`-separated
//! directories holding `<name>` or `<org>/<name>`), a `repo=` that is itself
//! a path, and a `file://` URL.
use crate::session::Session;
use std::path::{Path, PathBuf};

/// The Git directory of a repository directory (work tree or bare).
pub fn git_dir(path: &Path) -> Option<PathBuf> {
    let dot = path.join(".git");
    if dot.is_dir() {
        return std::fs::canonicalize(dot).ok();
    }
    if dot.is_file() {
        let text = std::fs::read_to_string(&dot).ok()?;
        let target = text.lines().find_map(|l| l.strip_prefix("gitdir: "))?;
        return std::fs::canonicalize(path.join(target.trim())).ok();
    }
    (path.join("HEAD").is_file() && path.join("objects").is_dir())
        .then(|| std::fs::canonicalize(path).ok())
        .flatten()
}

fn mapped(session: &Session<'_>, repo: &str) -> Option<PathBuf> {
    let map = session
        .ws
        .repo
        .as_ref()?
        .git_dir
        .join("writfold")
        .join("repos");
    let text = std::fs::read_to_string(map).ok()?;
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .find_map(|line| {
            let (name, path) = line.trim().split_once(char::is_whitespace)?;
            (name == repo).then(|| PathBuf::from(path.trim()))
        })
}

fn searched(repo: &str) -> Option<PathBuf> {
    let roots = std::env::var_os("WRITFOLD_KB_REPOS")?;
    let name = repo.rsplit('/').next().unwrap_or(repo);
    std::env::split_paths(&roots)
        .flat_map(|root| [root.join(repo), root.join(name)])
        .find(|candidate| git_dir(candidate).is_some())
}

/// The directory (work tree or bare) a repository name or path names: a
/// mapped or searched name, else a path (relative to `cwd`).
pub fn locate(session: &Session<'_>, repo: &str, cwd: &Path) -> Option<PathBuf> {
    let direct = cwd.join(repo);
    [mapped(session, repo), searched(repo), Some(direct)]
        .into_iter()
        .flatten()
        .find(|path| git_dir(path).is_some())
        .and_then(|path| std::fs::canonicalize(path).ok())
}

/// The Git directory for a citation's `repo=` (and optional `url=`).
pub fn resolve(session: &Session<'_>, repo: &str, url: Option<&str>) -> Option<PathBuf> {
    if repo.is_empty() || repo.contains("..") {
        return None;
    }
    let candidates = [
        mapped(session, repo),
        searched(repo),
        Path::new(repo).is_absolute().then(|| PathBuf::from(repo)),
        url.and_then(|u| u.strip_prefix("file://"))
            .map(PathBuf::from),
    ];
    candidates
        .into_iter()
        .flatten()
        .find_map(|path| git_dir(&path))
}
