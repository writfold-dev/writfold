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
use serde_json::json;
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
    Ok(output(&all, report, request.json, deny))
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

fn output(all: &[File], report: Report, json: bool, deny: bool) -> Output {
    let invalid: Vec<&str> = all
        .iter()
        .filter(|f| {
            report
                .diagnostics
                .iter()
                .any(|d| d.file == f.shown && d.is_error())
        })
        .map(|f| f.shown.as_str())
        .collect();
    let (errors, warnings) = (report.errors(), report.warnings());
    let exit = match errors > 0 || (deny && warnings > 0) {
        true => 2,
        false => 0,
    };
    let counts = (all.len(), all.len() - invalid.len(), invalid.len());
    let trailer = format!(
        "# coverage=complete files={} clean={} invalid={} errors={errors} warnings={warnings}",
        counts.0, counts.1, counts.2
    );
    if !json {
        return Output {
            text: format!("{trailer}\n"),
            exit,
            report: None,
            shown: Some(report),
        };
    }
    let rows: Vec<serde_json::Value> = all
        .iter()
        .map(|f| {
            let status = if invalid.contains(&f.shown.as_str()) { "invalid" } else { "clean" };
            json!({"kind": "definition", "id": f.shown, "status": status, "text": "", "path": f.shown,
                   "line": null, "flags": [], "more": [], "section": null})
        })
        .collect();
    let body = json!({"writfold_kb": 1, "verb": "lint", "rows": rows, "notes": [],
        "trailer": {"coverage": "complete", "files": counts.0, "clean": counts.1, "invalid": counts.2,
                    "errors": errors, "warnings": warnings},
        "diagnostics": super::json(&report)});
    Output {
        text: format!("{body}\n"),
        exit,
        report: None,
        shown: None,
    }
}
