//! Finding a local clone of a cited repository. v1 never clones or fetches.
//!
//! In order: the local repository map (`<git dir>/writfold/repos`, lines of
//! `<name> <path>`), the search roots in `$WRITFOLD_KB_REPOS` (`:`-separated
//! directories holding `<name>` or `<org>/<name>`), a `repo=` that is itself
//! a path, and a `file://` URL. A partial clone (a promisor remote) is
//! told apart by reading its config file, without spawning Git.
use crate::error::Result;
use crate::session::Session;
use crate::workspace::{CANCELLED, Workspace, budgets};
use std::path::{Path, PathBuf};
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_commit};

/// Which rule found a repository's clone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Via {
    /// The KB's local repository map.
    Map,
    /// A directory under one of the `$WRITFOLD_KB_REPOS` roots.
    SearchRoot,
    /// A `repo=` that is an absolute path.
    Path,
    /// A `file://` `url=`.
    Url,
}

impl Via {
    pub fn name(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::SearchRoot => "search-root",
            Self::Path => "path",
            Self::Url => "url",
        }
    }
}

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
pub fn locate(ws: Option<&Workspace>, repo: &str, cwd: &Path) -> Option<PathBuf> {
    let direct = cwd.join(repo);
    [mapped(ws, repo), searched(repo), Some(direct)]
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
    resolve_via(ws, repo, url).map(|(dir, _)| dir)
}

/// [`resolve_in`], with the rule that found the clone.
pub fn resolve_via(
    ws: Option<&Workspace>,
    repo: &str,
    url: Option<&str>,
) -> Option<(PathBuf, Via)> {
    if repo.is_empty() || repo.contains("..") {
        return None;
    }
    let candidates = [
        (mapped(ws, repo), Via::Map),
        (searched(repo), Via::SearchRoot),
        (
            Path::new(repo).is_absolute().then(|| PathBuf::from(repo)),
            Via::Path,
        ),
        (
            url.and_then(|u| u.strip_prefix("file://"))
                .map(PathBuf::from),
            Via::Url,
        ),
    ];
    candidates
        .into_iter()
        .find_map(|(path, via)| Some((git_dir(&path?)?, via)))
}

/// The directory holding a Git directory's shared files (its config and
/// objects): itself, or what a linked worktree's `commondir` names.
fn common_dir(git_dir: &Path) -> PathBuf {
    std::fs::read_to_string(git_dir.join("commondir"))
        .ok()
        .map(|text| git_dir.join(text.trim()))
        .unwrap_or_else(|| git_dir.to_path_buf())
}

/// Whether the clone in `git_dir` is a partial clone: its config sets
/// `extensions.partialClone` or marks a remote `promisor`. Its missing
/// blobs would need a fetch, which KB reads never make.
pub fn partial(git_dir: &Path) -> bool {
    let config = common_dir(git_dir).join("config");
    promisor(&std::fs::read_to_string(config).unwrap_or_default())
}

/// [`partial`] on the text of a Git config file. `include` directives are
/// not followed.
pub fn promisor(config: &str) -> bool {
    let mut section = String::new();
    for line in config.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if let Some(header) = line.strip_prefix('[') {
            let name = header.split([']', ' ', '\t', '"', '.']).next();
            section = name.unwrap_or_default().to_ascii_lowercase();
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, "true"));
        let key = key.trim().to_ascii_lowercase();
        let value = value.split(['#', ';']).next().unwrap_or_default();
        let value = value.trim().trim_matches('"').to_ascii_lowercase();
        let on = matches!(value.as_str(), "true" | "yes" | "on" | "1");
        match (section.as_str(), key.as_str()) {
            ("extensions", "partialclone") if !value.is_empty() => return true,
            ("remote", "promisor") if on => return true,
            _ => {}
        }
    }
    false
}

/// When the clone last fetched: the modification time of its `FETCH_HEAD`
/// (seconds since the epoch), if it has one.
pub fn fetched(git_dir: &Path) -> Option<i64> {
    [git_dir.to_path_buf(), common_dir(git_dir)]
        .iter()
        .find_map(|dir| std::fs::metadata(dir.join("FETCH_HEAD")).ok())
        .and_then(|meta| meta.modified().ok())
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_secs()).ok())
}

/// The commit citations are checked against: `origin/HEAD`, else `HEAD`,
/// with the name of the ref that gave it.
pub fn target(
    git: &GitExecutable<'_>,
    git_dir: &Path,
) -> Result<Option<(GitObjectId, &'static str)>> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    for (spec, named) in [
        ("refs/remotes/origin/HEAD", "origin/HEAD"),
        ("HEAD", "HEAD"),
    ] {
        if let Some(commit) = kb_commit(git, reads, spec, budgets()?, &CANCELLED)? {
            return Ok(Some((commit, named)));
        }
    }
    Ok(None)
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

#[cfg(test)]
mod tests {
    use super::promisor;

    #[test]
    fn promisor_remotes_and_partial_clone_extensions() {
        let marked = [
            "[extensions]\n\tpartialClone = origin\n",
            "[remote \"origin\"]\n\turl = x\n\tpromisor = true\n",
            "[remote \"o\"]\n\tPromisor = yes ; trailing\n",
            "[remote \"o\"]\n\tpromisor\n",
            "[EXTENSIONS]\npartialclone=\"o\"\n",
        ];
        for text in marked {
            assert!(promisor(text), "{text}");
        }
        let plain = [
            "",
            "[core]\n\tbare = true\n",
            "[remote \"origin\"]\n\tpromisor = false\n",
            "[core]\n\tpromisor = true\n",
            "[extensions]\n\tpartialclone =\n",
            "# [extensions]\n# partialclone = origin\n",
            "[remote \"origin\"]\n\t# promisor = true\n",
            "[include]\n\tpath = promisor.config\n",
        ];
        for text in plain {
            assert!(!promisor(text), "{text}");
        }
    }
}
