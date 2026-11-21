//! Finding gate definitions: every `.writfold/gates/<id>.toml` of the KB
//! root and of the subject repository, parsed and checked ([`super::def`]),
//! with the files that do not parse kept as problems, each with every
//! fault it has. Finding a definition only reads files; nothing is run.
use super::def::{DIR, Definition, Origin};
use crate::defs::{Problem, read_text};
use crate::error::{KbError, Result};
use std::path::{Path, PathBuf};
use writfold_core::inventory::{self, Root};

/// The roots definitions are read from: the KB root, then the subject
/// repository's top when it is another directory, with the prefix its
/// files are shown with (`<repo>:` for the repository).
fn roots(kb: &Path, repo: Option<&Path>) -> Vec<(Origin, PathBuf, String)> {
    let mut out = vec![(Origin::Kb, kb.join(DIR), String::new())];
    if let Some(top) = repo {
        let dir = top.join(DIR);
        let same = match (
            std::fs::canonicalize(&out[0].1),
            std::fs::canonicalize(&dir),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !same {
            let name = top
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            out.push((Origin::Repo, dir, format!("{name}:")));
        }
    }
    out
}

/// A gate definition file that does not parse or check.
pub type Unusable = (Origin, Problem);

/// Every definition, every file that does not parse, and how each root
/// was read (#56).
pub struct Inventory {
    pub definitions: Vec<Definition>,
    pub problems: Vec<Unusable>,
    pub roots: Vec<Root>,
}

impl Inventory {
    /// Whether no root was unreadable ([`inventory::complete`]).
    pub fn complete(&self) -> bool {
        inventory::complete(&self.roots)
    }
}

/// Lists the entry names of a directory: the outer error when it cannot
/// be opened, an inner one for an entry that fails part-way.
pub type Lister = fn(&Path) -> std::io::Result<Vec<std::io::Result<String>>>;

/// [`Lister`] over the filesystem.
pub fn list(dir: &Path) -> std::io::Result<Vec<std::io::Result<String>>> {
    let entries = std::fs::read_dir(dir)?;
    Ok(entries
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect())
}

/// How a root reads: absent (not found), or its names and, when it could
/// not be listed in full, why. A dangling link and a non-directory are
/// unreadable, not absent.
pub fn classify(dir: &Path, lister: Lister) -> (Root, Vec<String>, Option<String>) {
    match std::fs::symlink_metadata(dir) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (Root::Absent, Vec::new(), None);
        }
        Err(error) => return (Root::Unreadable, Vec::new(), Some(error.to_string())),
        Ok(_) => {}
    }
    match std::fs::metadata(dir) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return (Root::Unreadable, Vec::new(), Some("not a directory".into())),
        Err(error) => {
            return (
                Root::Unreadable,
                Vec::new(),
                Some(format!("a link whose target cannot be read: {error}")),
            );
        }
    }
    let entries = match lister(dir) {
        Ok(entries) => entries,
        Err(error) => return (Root::Unreadable, Vec::new(), Some(error.to_string())),
    };
    let mut names = Vec::new();
    let mut failed = None;
    for entry in entries {
        match entry {
            Ok(name) => names.push(name),
            Err(error) => failed = failed.or(Some(format!("an entry failed part-way: {error}"))),
        }
    }
    match failed {
        Some(why) => (Root::Unreadable, names, Some(why)),
        None => (Root::Listed, names, None),
    }
}

/// The problem an unreadable directory adds (`path` relative to its root,
/// `shown` with the root's prefix): one per directory, counted as invalid
/// wherever problems are, so no aggregate answer is positive meanwhile.
pub fn unlisted(path: &str, shown: &str, why: &str) -> Problem {
    Problem::whole(
        path,
        shown,
        "file.unlisted",
        format!(
            "{shown} could not be listed ({why}); what it defines is not known, so no answer that needs all of it can be complete: make it a readable directory, or remove it if it defines nothing"
        ),
    )
}

/// Every definition, and every file that does not parse.
pub fn all(kb: &Path, repo: Option<&Path>) -> (Vec<Definition>, Vec<Unusable>) {
    let found = inventory(kb, repo, list);
    (found.definitions, found.problems)
}

/// Every definition, every problem, and each root's state; `lister` reads
/// a directory (the filesystem's, or a test's that fails part-way).
pub fn inventory(kb: &Path, repo: Option<&Path>, lister: Lister) -> Inventory {
    let mut found: Vec<Definition> = Vec::new();
    let mut problems = Vec::new();
    let mut states = Vec::new();
    for (origin, dir, prefix) in roots(kb, repo) {
        let (state, names, why) = classify(&dir, lister);
        if let Some(why) = why {
            problems.push((origin, unlisted(DIR, &format!("{prefix}{DIR}"), &why)));
        }
        states.push(state);
        let mut names: Vec<String> = (names.into_iter())
            .filter(|n| n.ends_with(".toml") && !n.starts_with('.'))
            .collect();
        names.sort();
        for name in names {
            let stem = name.trim_end_matches(".toml");
            let path = format!("{DIR}/{name}");
            let shown = format!("{prefix}{path}");
            let parsed = read_text(&dir.join(&name), &shown).and_then(|text| {
                super::def_check::check(&text, stem, origin, &dir, &shown).map_err(|report| {
                    Problem {
                        path: path.clone(),
                        report,
                    }
                })
            });
            let problem = match parsed {
                Ok(d) if found.iter().any(|f| f.id == d.id) => {
                    let why = format!(
                        "gate {} is defined in both the KB root and the repository",
                        d.id
                    );
                    Problem::whole(&path, &shown, "file.twice", why)
                }
                Ok(d) => {
                    found.push(d);
                    continue;
                }
                Err(mut problem) => {
                    problem.path = path;
                    problem
                }
            };
            problems.push((origin, problem));
        }
    }
    Inventory {
        definitions: found,
        problems,
        roots: states,
    }
}

/// The definition of `id`, refusing a missing, malformed or doubly defined
/// gate.
pub fn find(kb: &Path, repo: Option<&Path>, id: &str) -> Result<Definition> {
    let found = inventory(kb, repo, list);
    let complete = found.complete();
    let name = format!("{DIR}/{id}.toml");
    if let Some((_, problem)) = (found.problems.into_iter()).find(|(_, p)| p.path == name) {
        return Err(KbError::Definition(Box::new(problem.report)));
    }
    found.definitions.into_iter().find(|d| d.id == id).ok_or_else(|| {
        let unread = match complete {
            true => "",
            false => " (a gates directory could not be listed, so it may be defined there: writfold kb lint)",
        };
        KbError::Refused(format!(
            "no gate {id}: define {name} in the KB root or the repository{unread}"
        ))
    })
}

/// The repository gate `id` (for `gate run`, `gate attest`, `gate
/// waive`): a candidate gate is refused, naming `deliver`.
pub fn find_repository(kb: &Path, repo: Option<&Path>, id: &str) -> Result<Definition> {
    let d = find(kb, repo, id)?;
    if d.on_candidate() {
        return Err(KbError::Refused(format!(
            "gate {id} judges delivery candidates (subject = \"candidate\"): name it in a delivery mode's gates and use writfold kb deliver check or deliver attest"
        )));
    }
    Ok(d)
}
