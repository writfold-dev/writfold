//! Finding gate definitions: every `.writfold/gates/<id>.toml` of the KB
//! root and of the subject repository, parsed and checked ([`super::def`]),
//! with the files that do not parse kept as problems. Finding a definition
//! only reads files; nothing is run.
use super::def::{DIR, Definition, Origin, parse};
use crate::error::{KbError, Result};
use std::path::{Path, PathBuf};

/// The roots definitions are read from: the KB root, then the subject
/// repository's top when it is another directory.
fn roots(kb: &Path, repo: Option<&Path>) -> Vec<(Origin, PathBuf)> {
    let mut out = vec![(Origin::Kb, kb.join(DIR))];
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
            out.push((Origin::Repo, dir));
        }
    }
    out
}

/// Every definition, and every file that does not parse (path, why).
pub fn all(kb: &Path, repo: Option<&Path>) -> (Vec<Definition>, Vec<(Origin, String, String)>) {
    let mut found: Vec<Definition> = Vec::new();
    let mut problems = Vec::new();
    for (origin, dir) in roots(kb, repo) {
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
            let parsed = std::fs::read_to_string(dir.join(&name))
                .map_err(KbError::from)
                .and_then(|text| parse(&text, stem, origin, &dir));
            match parsed {
                Ok(d) if found.iter().any(|f| f.id == d.id) => problems.push((
                    origin,
                    path,
                    format!("gate {} is defined in both the KB and the repository", d.id),
                )),
                Ok(d) => found.push(d),
                Err(error) => problems.push((origin, path, error.to_string())),
            }
        }
    }
    (found, problems)
}

/// The definition of `id`, refusing a missing, malformed or doubly defined
/// gate.
pub fn find(kb: &Path, repo: Option<&Path>, id: &str) -> Result<Definition> {
    let (found, problems) = all(kb, repo);
    let name = format!("{DIR}/{id}.toml");
    if let Some((_, path, why)) = problems.iter().find(|(_, path, _)| *path == name) {
        return Err(KbError::Refused(format!("{path}: {why}")));
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
