//! `writfold kb lint [PATH…] [--repo DIR] [--deny-warnings] [--json]`:
//! every definition file a person writes, in the KB root's `.writfold/`
//! and the repository's, checked with every fault reported, then the
//! references between them ([`super::lint_refs`]). It reads files only:
//! no command, no checker, no Git, no index, nothing written. Exit 0
//! without errors; 2 with an error (or a warning under
//! `--deny-warnings`); 1 for a usage or IO error; never 3.
use super::Problem;
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::gate::def::Origin;
use std::path::{Path, PathBuf};
use writfold_core::diag::Report;

/// What a file is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Gate,
    Guard,
    Template,
    Required,
    Mode,
    Deny,
    Delivery,
    Source,
    Queries,
}

/// One file read: where it is, how it is shown, its text and its faults.
pub struct File {
    pub kind: Kind,
    pub origin: Origin,
    /// The directory holding it.
    pub dir: PathBuf,
    pub stem: String,
    /// As rows show it: `.writfold/gates/unit.toml`, `<repo>:.writfold/…`.
    pub shown: String,
    pub text: String,
    pub report: Report,
}

const DIRS: [(&str, Kind); 5] = [
    (crate::gate::def::DIR, Kind::Gate),
    (crate::guard::rules::DIR, Kind::Guard),
    (crate::plan::template::DIR, Kind::Template),
    (crate::deliver::config::DIR, Kind::Mode),
    (crate::stale::kinds::DIR, Kind::Source),
];

fn check(kind: Kind, origin: Origin, dir: &Path, stem: &str, shown: &str, text: &str) -> Report {
    let failed = match kind {
        Kind::Gate => crate::gate::def_check::check(text, stem, origin, dir, shown).err(),
        Kind::Guard => crate::guard::rules::parse(text, stem, shown).err(),
        Kind::Template => crate::plan::template::check(text, Some(stem), shown).err(),
        Kind::Required => match crate::plan::required::parse_shown(text.as_bytes(), shown) {
            Err(KbError::Definition(report)) => Some(*report),
            _ => None,
        },
        Kind::Source => crate::stale::kind_check::check(text, stem, shown).err(),
        Kind::Queries => crate::verbs::eval::lint(text, shown),
        Kind::Mode | Kind::Deny | Kind::Delivery => {
            crate::deliver::config::check_file(dir, stem, shown, text).err()
        }
    };
    let mut report = failed.unwrap_or_default();
    if report.sources.is_empty() {
        report.sources.push(writfold_core::diag::Source {
            file: shown.to_owned(),
            text: text.to_owned(),
        });
    }
    report
}

/// Every definition file under `top`'s `.writfold/`, read and checked.
pub fn files(top: &Path, origin: Origin, prefix: &str) -> Vec<File> {
    let mut out = Vec::new();
    for (rel, kind) in DIRS {
        let dir = top.join(rel);
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
            let stem = name.trim_end_matches(".toml").to_owned();
            let kind = match (kind, stem.as_str()) {
                (Kind::Template, crate::plan::template::RESERVED) => Kind::Required,
                (Kind::Mode, "guard") => Kind::Delivery,
                (Kind::Mode, s) if s.starts_with("deny-") => Kind::Deny,
                (Kind::Source | Kind::Mode, _) if origin == Origin::Repo => continue,
                _ => kind,
            };
            let shown = format!("{prefix}{rel}/{name}");
            let (text, report) = match super::read_text(&dir.join(&name), &shown) {
                Ok(text) => {
                    let report = check(kind, origin, &dir, &stem, &shown, &text);
                    (text, report)
                }
                Err(Problem { report, .. }) => (String::new(), report),
            };
            out.push(File {
                kind,
                origin,
                dir: dir.clone(),
                stem,
                shown,
                text,
                report,
            });
        }
    }
    out
}

/// `.writfold/queries.jsonl` of the KB root, when it exists.
fn queries(kb: &Path) -> Option<File> {
    let shown = crate::verbs::eval::DEFAULT;
    let path = kb.join(shown);
    if !path.is_file() {
        return None;
    }
    let (text, report) = match super::read_text(&path, shown) {
        Ok(text) => {
            let dir = path.parent().unwrap_or(kb);
            let report = check(Kind::Queries, Origin::Kb, dir, "queries", shown, &text);
            (text, report)
        }
        Err(Problem { report, .. }) => (String::new(), report),
    };
    let dir = kb.join(".writfold");
    Some(File {
        kind: Kind::Queries,
        origin: Origin::Kb,
        dir,
        stem: "queries".into(),
        shown: shown.into(),
        text,
        report,
    })
}

/// The repository in view: `--repo`, else the repository holding `cwd`
/// (found by `stat`, never by Git) when it is not the KB root.
fn repository(kb: &Path, cwd: &Path, repo: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(repo) = repo {
        let repo = cwd.join(repo);
        return match repo.is_dir() {
            true => Ok(Some(repo)),
            false => Err(KbError::Usage(format!(
                "--repo {}: not a directory",
                repo.display()
            ))),
        };
    }
    Ok(enclosing(kb, cwd))
}

/// The top of the Git repository holding `cwd` (the nearest directory
/// with a `.git` entry), when it is not the KB root's.
pub fn enclosing(kb: &Path, cwd: &Path) -> Option<PathBuf> {
    let kb = std::fs::canonicalize(kb).ok();
    let top = cwd.ancestors().find(|d| d.join(".git").exists())?;
    (std::fs::canonicalize(top).ok() != kb).then(|| top.to_path_buf())
}

/// Run the lint over the KB root and the repository in view.
pub fn execute(
    kb: &Path,
    request: &crate::Request,
    (paths, repo, deny): (&[PathBuf], Option<&Path>, bool),
) -> Result<Output> {
    let repo = repository(kb, &request.cwd, repo)?;
    let mut all = files(kb, Origin::Kb, "");
    all.extend(queries(kb));
    if let Some(top) = &repo {
        let name = top
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        all.extend(files(top, Origin::Repo, &format!("{name}:")));
    }
    let mut report = super::lint_refs::check(&all, repo.is_some());
    let wanted = wanted(kb, repo.as_deref(), &request.cwd, paths, &all)?;
    let all: Vec<File> = all
        .into_iter()
        .filter(|f| wanted.contains(&f.shown))
        .collect();
    report.diagnostics.retain(|d| wanted.contains(&d.file));
    for f in &all {
        report.extend(f.report.clone());
    }
    let order: Vec<&str> = all.iter().map(|f| f.shown.as_str()).collect();
    report
        .sources
        .sort_by_key(|s| order.iter().position(|o| *o == s.file));
    report.sort();
    Ok(super::lint_out::output(&all, report, request.json, deny))
}

/// The shown names of the files `paths` select (every file without).
fn wanted(
    kb: &Path,
    repo: Option<&Path>,
    cwd: &Path,
    paths: &[PathBuf],
    all: &[File],
) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(all.iter().map(|f| f.shown.clone()).collect());
    }
    let mut out = Vec::new();
    for path in paths {
        let real = std::fs::canonicalize(cwd.join(path))
            .map_err(|e| KbError::Usage(format!("{}: {e}", path.display())))?;
        let found = all.iter().find(|f| {
            let top = match f.origin {
                Origin::Kb => Some(kb),
                Origin::Repo => repo,
            };
            let rel = f.shown.split_once(':').map_or(f.shown.as_str(), |(_, r)| r);
            top.and_then(|t| std::fs::canonicalize(t.join(rel)).ok()) == Some(real.clone())
        });
        match found {
            Some(f) => out.push(f.shown.clone()),
            None => {
                return Err(KbError::Usage(format!(
                    "{}: not a definition file kb lint reads",
                    path.display()
                )));
            }
        }
    }
    Ok(out)
}
