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
use writfold_runtime::git::{GitExecutable, trial_blob};
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
    /// Exactly the retained bytes (the file bytes at `range`).
    pub bytes: Vec<u8>,
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
    // Reading the pinned blob writes nothing.
    let file = super::refused::before_effects((|| {
        let executable = trial.executable()?;
        read_pinned(&executable, repo, commit, path)
    })())?;
    retain_read(&trial, &file, &[range])?
        .pop()
        .ok_or_else(|| "no binding retained".into())
}

/// Retain several ranges of one file at `commit`, reading the pinned blob
/// once. Each range becomes its own binding, retained exactly as `retain`
/// does (Tuft's `retain` uses this path).
pub fn retain_ranges(
    trial: &Trial,
    executable: &GitExecutable<'_>,
    repo: &Path,
    commit: &str,
    path: &str,
    ranges: &[(u64, u64)],
) -> Result<Vec<Retained>> {
    retain_read(trial, &read_pinned(executable, repo, commit, path)?, ranges)
}

/// The exact bytes of `path` at the full commit id `commit`, read through
/// `trial_blob` (a ref or `HEAD` is refused). Reading takes no trial state,
/// so a caller may read several files concurrently before retaining them.
pub struct PinnedBlob {
    commit: GitObjectId,
    path: String,
    blob: GitObjectId,
    pub content: Vec<u8>,
}

pub fn read_pinned(
    executable: &GitExecutable<'_>,
    repo: &Path,
    commit: &str,
    path: &str,
) -> Result<PinnedBlob> {
    let git_dir = fs::canonicalize(repo.join(".git"))?;
    let commit = commit_id(commit)?;
    let (blob, content) = trial_blob(executable, &git_dir, &commit, path, budgets()?, &CANCELLED)?;
    Ok(PinnedBlob {
        commit,
        path: path.to_owned(),
        blob,
        content,
    })
}

/// Retain `ranges` of a blob read by `read_pinned`, one binding per range.
/// Every selection is bound first; nothing is written unless all bind, and
/// a binding refusal is marked as decided before any write.
pub fn retain_read(
    trial: &Trial,
    file: &PinnedBlob,
    ranges: &[(u64, u64)],
) -> Result<Vec<Retained>> {
    let bound = super::refused::before_effects(
        ranges
            .iter()
            .map(|&range| {
                let descriptor = descriptor(
                    trial,
                    &file.commit,
                    &file.path,
                    file.blob.clone(),
                    &file.content,
                    range,
                )?;
                let binding = binding_fingerprint(&descriptor, limits()?)?
                    .as_str()
                    .to_owned();
                Ok((range, descriptor, binding))
            })
            .collect::<Result<Vec<_>>>(),
    )?;
    let dir = evidence_dir(trial);
    fs::create_dir_all(&dir)?;
    let mut out = Vec::with_capacity(bound.len());
    for (range, descriptor, binding) in bound {
        let selected = file
            .content
            .get(range.0 as usize..range.1 as usize)
            .ok_or("byte range is outside the file")?;
        trial.retain(selected)?;
        fs::write(dir.join(format!("{binding}.bytes")), selected)?;
        fs::write(
            dir.join(format!("{binding}.json")),
            encode_descriptor(&descriptor, super::BUDGET)?,
        )?;
        out.push(Retained {
            binding,
            path: file.path.clone(),
            commit: file.commit.as_str().to_owned(),
            range,
            bytes: selected.to_vec(),
        });
    }
    Ok(out)
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
