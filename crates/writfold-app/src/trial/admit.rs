//! `prepare`'s checks that write nothing: path, repository, recipe and report.
//! A refusal here is decided before any effect.
use super::state::Trial;
use super::{BUDGET, Result, limits};
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(BUDGET as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > BUDGET {
        return Err(format!("{} is larger than {BUDGET} bytes", path.display()).into());
    }
    Ok(bytes)
}

fn relative(path: &str) -> Result<()> {
    let p = Path::new(path);
    if path.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!("{path}: use a plain relative path inside the repository").into());
    }
    Ok(())
}

/// A bound report must be a complete local Weft report, and every evidence
/// binding it cites must be retained in this state.
fn check_report(trial: &Trial, bytes: &[u8]) -> Result<()> {
    let parsed = writfold_wire::weft_report::parse_report(bytes, limits()?)
        .map_err(|e| format!("report refused: {e}"))?;
    for asset in &parsed.assets {
        if !super::retain::retained(trial, asset)? {
            return Err(format!(
                "report refused: it cites {asset:?}, which is not evidence retained by `writfold trial retain`"
            )
            .into());
        }
    }
    Ok(())
}

pub(super) type Admitted = (Trial, std::path::PathBuf, Vec<u8>, Vec<u8>);

/// Every check that writes nothing: path, repository, recipe, report.
pub(super) fn admit(
    state: &Path,
    repo: &Path,
    path: &str,
    recipe: &Path,
    report: Option<&Path>,
) -> Result<Admitted> {
    relative(path)?;
    let git_dir = repo.join(".git");
    if !git_dir.is_dir() {
        return Err(format!(
            "{} is not a Git repository with a .git directory",
            repo.display()
        )
        .into());
    }
    let recipe_bytes = read_bounded(recipe)?;
    let report_bytes = report.map(read_bounded).transpose()?.unwrap_or_default();
    let trial = Trial::open(state)?;
    if report.is_some() {
        check_report(&trial, &report_bytes)?;
    }
    Ok((
        trial,
        fs::canonicalize(&git_dir)?,
        recipe_bytes,
        report_bytes,
    ))
}
