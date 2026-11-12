//! Finding a local clone of a cited repository. v1 never clones or fetches.
//!
//! In order: the local repository map (`<git dir>/writfold/repos`, lines of
//! `<name> <path>`), the search roots in `$WRITFOLD_KB_REPOS` (`:`-separated
//! directories holding `<name>` or `<org>/<name>`), a `repo=` that is itself
//! a path, and a `file://` URL.
use crate::session::Session;
use crate::workspace::Workspace;
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

/// The `(name, path)` entries of the KB's local repository map.
fn map(ws: Option<&Workspace>) -> Vec<(String, PathBuf)> {
    let Some(repo) = ws.and_then(|ws| ws.repo.as_ref()) else {
        return Vec::new();
    };
    let file = repo.git_dir.join("writfold").join("repos");
    let text = std::fs::read_to_string(file).unwrap_or_default();
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| {
            let (name, path) = line.trim().split_once(char::is_whitespace)?;
            Some((name.to_owned(), PathBuf::from(path.trim())))
        })
        .collect()
}

fn mapped(ws: Option<&Workspace>, repo: &str) -> Option<PathBuf> {
    map(ws)
        .into_iter()
        .find_map(|(name, path)| (name == repo).then_some(path))
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
    [mapped(Some(session.ws), repo), searched(repo), Some(direct)]
        .into_iter()
        .flatten()
        .find(|path| git_dir(path).is_some())
        .and_then(|path| std::fs::canonicalize(path).ok())
}

/// The Git directory for a citation's `repo=` (and optional `url=`).
pub fn resolve(session: &Session<'_>, repo: &str, url: Option<&str>) -> Option<PathBuf> {
    resolve_in(Some(session.ws), repo, url)
}

/// The Git directory a repository name (or absolute path) names, through
/// the KB's map and `$WRITFOLD_KB_REPOS`.
pub fn resolve_in(ws: Option<&Workspace>, repo: &str, url: Option<&str>) -> Option<PathBuf> {
    if repo.is_empty() || repo.contains("..") {
        return None;
    }
    let candidates = [
        mapped(ws, repo),
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

/// The name the KB's local map gives the repository in `dir`, if any.
pub fn mapped_name(ws: Option<&Workspace>, dir: &Path) -> Option<String> {
    let target = git_dir(dir)?;
    map(ws)
        .into_iter()
        .find_map(|(name, path)| (git_dir(&path).as_ref() == Some(&target)).then_some(name))
}

/// The `repo=` name to cite the repository in `dir` by: a name in the KB's
/// map that points at it, else its directory name. The flag says whether
/// `stale` finds `dir` again from that name (and `url`); when it does not,
/// the citation checks as unverifiable until the name is mapped.
pub fn name_for(ws: Option<&Workspace>, dir: &Path, url: &str) -> (String, bool) {
    let target = git_dir(dir);
    let same = |path: &Path| target.is_some() && git_dir(path) == target;
    if let Some((name, _)) = map(ws).into_iter().find(|(_, path)| same(path)) {
        return (name, true);
    }
    let name = dir
        .file_name()
        .map_or_else(|| "repo".to_owned(), |n| n.to_string_lossy().into_owned());
    let found = resolve_in(ws, &name, Some(url)).is_some_and(|found| Some(found) == target);
    (name, found)
}
