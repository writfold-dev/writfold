//! `writfold trial retain`: exact bytes of a file at a pinned commit, retained
//! in the trial state and bound as a `writfold-citation/1` binding descriptor.
//! The binding digest is what a Weft report cites (`asset x = citation
//! "<digest>"`); `prepare` refuses a report citing anything not retained here.
//!
//! The selection is exact retained bytes at a byte range, not a search result,
//! not a summary and not a claim that the bytes are true.
use super::state::{CANCELLED, Trial};
use super::{Result, budgets, limits};
use std::fs;
use std::path::{Path, PathBuf};
use writfold_domain::citation::{CoordinateUnit, Selector};
use writfold_domain::{ByteOffset, GitObjectFormat, GitObjectId};
use writfold_runtime::git::trial_blob;
use writfold_wire::citation::{
    CitationDescriptor, GitSource, binding_fingerprint, decode_descriptor, encode_descriptor,
    selection_digest,
};

/// A retained binding: its digest and what it selects.
pub struct Retained {
    pub binding: String,
    pub path: String,
    pub commit: String,
    pub range: (u64, u64),
}

fn evidence_dir(trial: &Trial) -> PathBuf {
    trial.root.join("evidence")
}

fn commit_id(text: &str) -> Result<GitObjectId> {
    let format = match text.len() {
        40 => GitObjectFormat::Sha1,
        64 => GitObjectFormat::Sha256,
        _ => return Err("--commit must be a full commit id (no refs, no HEAD)".into()),
    };
    Ok(GitObjectId::new(format, text.to_owned())?)
}

fn descriptor(
    trial: &Trial,
    commit: &GitObjectId,
    path: &str,
    blob: GitObjectId,
    content: &[u8],
    range: (u64, u64),
) -> Result<CitationDescriptor> {
    let source = GitSource::new(
        trial.realm.clone(),
        "repo".parse()?,
        commit.clone(),
        path.to_owned(),
        blob,
        selection_digest(content)?,
    )?;
    let selector = Selector::new(
        ByteOffset::new(range.0)?,
        ByteOffset::new(range.1)?,
        CoordinateUnit::Byte,
    )
    .map_err(|e| format!("byte range refused: {e:?}"))?;
    let selected = content
        .get(range.0 as usize..range.1 as usize)
        .ok_or("byte range is outside the file")?;
    let short = commit.as_str().get(..12).ok_or("commit id")?;
    Ok(CitationDescriptor::new(
        "trial-retain".parse()?,
        format!("commit-{short}").parse()?,
        "trial".parse()?,
        "excerpt".parse()?,
        source,
        selector,
        selection_digest(selected)?,
    )?)
}

/// Retain `[lo, hi)` of `path` at `commit` in `repo`.
pub fn retain(
    state: &Path,
    repo: &Path,
    commit: &str,
    path: &str,
    range: (u64, u64),
) -> Result<Retained> {
    let trial = super::refused::before_effects(Trial::open(state))?;
    // Reading the pinned blob and binding the selection write nothing.
    let admitted = super::refused::before_effects((|| {
        let executable = trial.executable()?;
        let git_dir = fs::canonicalize(repo.join(".git"))?;
        let commit = commit_id(commit)?;
        let (blob, content) =
            trial_blob(&executable, &git_dir, &commit, path, budgets()?, &CANCELLED)?;
        let descriptor = descriptor(&trial, &commit, path, blob, &content, range)?;
        let binding = binding_fingerprint(&descriptor, limits()?)?
            .as_str()
            .to_owned();
        Ok((commit, content, descriptor, binding))
    })());
    let (commit, content, descriptor, binding) = admitted?;
    let selected = content
        .get(range.0 as usize..range.1 as usize)
        .ok_or("byte range is outside the file")?;
    trial.retain(selected)?;
    let dir = evidence_dir(&trial);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{binding}.bytes")), selected)?;
    fs::write(
        dir.join(format!("{binding}.json")),
        encode_descriptor(&descriptor, super::BUDGET)?,
    )?;
    Ok(Retained {
        binding,
        path: path.to_owned(),
        commit: commit.as_str().to_owned(),
        range,
    })
}

/// Whether `binding` names evidence retained in this state: its descriptor
/// decodes, re-fingerprints to exactly `binding`, and its retained bytes match
/// the descriptor's selection digest and length.
pub fn retained(trial: &Trial, binding: &str) -> Result<bool> {
    if binding.len() != 64 || !binding.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let dir = evidence_dir(trial);
    let (Ok(encoded), Ok(bytes)) = (
        fs::read(dir.join(format!("{binding}.json"))),
        fs::read(dir.join(format!("{binding}.bytes"))),
    ) else {
        return Ok(false);
    };
    let descriptor = decode_descriptor(&encoded, limits()?)?;
    if binding_fingerprint(&descriptor, limits()?)?.as_str() != binding {
        return Ok(false);
    }
    Ok(descriptor.bind(&bytes).is_ok())
}
