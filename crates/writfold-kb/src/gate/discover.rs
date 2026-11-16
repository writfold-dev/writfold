//! Finding gate definitions: every `.writfold/gates/<id>.toml` of the KB
//! root and of the subject repository, parsed and checked ([`super::def`]),
//! with the files that do not parse kept as problems, each with every
//! fault it has. Finding a definition only reads files; nothing is run.
use super::def::{DIR, Definition, Origin};
use crate::defs::{Problem, read_text};
use crate::error::{KbError, Result};
use std::path::{Path, PathBuf};

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

/// Every definition, and every file that does not parse.
pub fn all(kb: &Path, repo: Option<&Path>) -> (Vec<Definition>, Vec<Unusable>) {
    let mut found: Vec<Definition> = Vec::new();
    let mut problems = Vec::new();
    for (origin, dir, prefix) in roots(kb, repo) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
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
    (found, problems)
}

/// The definition of `id`, refusing a missing, malformed or doubly defined
/// gate.
pub fn find(kb: &Path, repo: Option<&Path>, id: &str) -> Result<Definition> {
    let (found, problems) = all(kb, repo);
    let name = format!("{DIR}/{id}.toml");
    if let Some((_, problem)) = problems.into_iter().find(|(_, p)| p.path == name) {
        return Err(KbError::Definition(Box::new(problem.report)));
    }
    found.into_iter().find(|d| d.id == id).ok_or_else(|| {
        KbError::Refused(format!(
            "no gate {id}: define {name} in the KB root or the repository"
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
