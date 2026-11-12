//! Replay: each recipe names one file (`edit "<path>" { … }`); the pinned
//! Splicery kernel replays it against that file's current worktree bytes
//! (not HEAD, so an agent's uncommitted edits count). Several recipes form
//! one batch over distinct files, every one replayed against the original
//! bytes before anything is written. Any refusal refuses the batch.
use super::diff::{Splice, unified};
use crate::error::{KbError, Result};
use crate::workspace::{CANCELLED, budgets};
use crate::write::atomic::digest;
use std::path::{Component, Path, PathBuf};
use writfold_runtime::git::{GitExecutable, edit_hash_bytes};
use writfold_sources::splicery::{recipe_path, replay};
use writfold_sources::splicery_refusal::describe;

/// One recipe as given.
pub struct Recipe {
    /// The file name of the recipe (its directory is not recorded).
    pub name: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// One splice as the kernel reports it.
pub struct Site {
    pub lo: usize,
    pub hi: usize,
    pub inserted: usize,
    /// The recipe bytes that produced it.
    pub origin: (usize, usize),
    /// The one-based line of `lo` in the original.
    pub line: usize,
}

/// One file of the batch.
pub struct FileEdit {
    /// Relative to the worktree's top.
    pub path: String,
    pub recipe: usize,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub before_sha: String,
    pub after_sha: String,
    pub before_blob: String,
    pub after_blob: String,
    pub sites: Vec<Site>,
    pub diff: String,
}

impl FileEdit {
    pub fn removed(&self) -> usize {
        self.sites.iter().map(|s| s.hi - s.lo).sum()
    }

    pub fn inserted(&self) -> usize {
        self.sites.iter().map(|s| s.inserted).sum()
    }
}

/// The file a recipe names, inside the worktree: no `.git`, no symbolic
/// link on the way, a regular file, and not inside the KB.
fn contained(top: &Path, kb: &Path, path: &str) -> Result<PathBuf> {
    let refuse = |why: &str| KbError::Refused(format!("{path}: {why}"));
    if path.contains(['`', '\n', '\r']) {
        return Err(refuse("unsupported characters in the path"));
    }
    let relative = Path::new(path);
    if relative.components().next() == Some(Component::Normal(".git".as_ref())) {
        return Err(refuse("inside the Git directory"));
    }
    let joined = top.join(relative);
    let real =
        std::fs::canonicalize(&joined).map_err(|_| refuse("no such file in the worktree"))?;
    if real != joined {
        return Err(refuse("reached through a symbolic link"));
    }
    if !std::fs::symlink_metadata(&real).is_ok_and(|m| m.file_type().is_file()) {
        return Err(refuse("not a regular file"));
    }
    if real.starts_with(kb) {
        return Err(refuse("inside the KB; change records with the kb verbs"));
    }
    Ok(real)
}

/// Read every recipe, refusing duplicates of one target path.
pub fn recipes(files: &[PathBuf], cwd: &Path) -> Result<(Vec<Recipe>, Vec<String>)> {
    if files.is_empty() {
        return Err(KbError::Usage(
            "edit apply needs at least one recipe".into(),
        ));
    }
    let mut out = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    for file in files {
        let full = cwd.join(file);
        let bytes = std::fs::read(&full)
            .map_err(|e| KbError::Usage(format!("cannot read recipe {}: {e}", file.display())))?;
        let name = full
            .file_name()
            .map_or_else(|| "recipe".into(), |n| n.to_string_lossy().into_owned());
        let path = recipe_path(&bytes).map_err(|e| {
            KbError::Refused(format!("recipe {name}: {}", describe(&e, "", &[], &bytes)))
        })?;
        if paths.contains(&path) {
            return Err(KbError::Refused(format!(
                "two recipes edit {path}; a batch edits each file once (combine them)"
            )));
        }
        paths.push(path);
        out.push(Recipe {
            sha256: digest(&bytes),
            name,
            bytes,
        });
    }
    Ok((out, paths))
}

/// Replay every recipe against its file's current bytes. Refusals are
/// collected so one answer names every recipe that failed.
pub fn replay_all(
    git: &GitExecutable<'_>,
    top: &Path,
    git_dir: &Path,
    kb: &Path,
    recipes: &[Recipe],
    paths: &[String],
) -> Result<Vec<FileEdit>> {
    let mut out = Vec::new();
    let mut refusals = Vec::new();
    for (index, (recipe, path)) in recipes.iter().zip(paths).enumerate() {
        match one(git, top, git_dir, kb, recipe, path, index) {
            Ok(edit) => out.push(edit),
            Err(KbError::Refused(why)) => refusals.push(why),
            Err(other) => return Err(other),
        }
    }
    if !refusals.is_empty() {
        return Err(KbError::Refused(format!(
            "nothing was written:\n{}",
            refusals.join("\n")
        )));
    }
    Ok(out)
}

fn one(
    git: &GitExecutable<'_>,
    top: &Path,
    git_dir: &Path,
    kb: &Path,
    recipe: &Recipe,
    path: &str,
    index: usize,
) -> Result<FileEdit> {
    let file = contained(top, kb, path)?;
    let before = std::fs::read(&file)?;
    let replayed = replay(before.clone(), path, &recipe.bytes).map_err(|e| {
        KbError::Refused(format!(
            "recipe {}: {}",
            recipe.name,
            describe(&e, path, &before, &recipe.bytes)
        ))
    })?;
    if replayed.no_change() || replayed.candidate() == before.as_slice() {
        return Err(KbError::Refused(format!(
            "recipe {}: the result equals the current bytes of {path}",
            recipe.name
        )));
    }
    let line = |at: usize| {
        before[..at.min(before.len())]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1
    };
    let mut sites: Vec<Site> = replayed
        .edits()
        .iter()
        .map(|e| Site {
            lo: e.target.lo(),
            hi: e.target.hi(),
            inserted: e.replacement.len(),
            origin: (e.origin.start, e.origin.end),
            line: line(e.target.lo()),
        })
        .collect();
    sites.sort_by_key(|s| (s.lo, s.hi));
    let after = replayed.into_candidate();
    let diff = preview(path, &before, &after, &sites);
    let hash = |bytes: &[u8]| -> Result<String> {
        Ok(edit_hash_bytes(
            git,
            git_dir,
            top,
            path,
            bytes,
            budgets()?,
            &CANCELLED,
        )?)
    };
    Ok(FileEdit {
        path: path.to_owned(),
        recipe: index,
        before_sha: digest(&before),
        after_sha: digest(&after),
        before_blob: hash(&before)?,
        after_blob: hash(&after)?,
        before,
        after,
        sites,
        diff,
    })
}

/// The diff from the sites: each site's inserted bytes are read back from
/// the after bytes, where the sites appear in order.
fn preview(path: &str, before: &[u8], after: &[u8], sites: &[Site]) -> String {
    let mut shift = 0isize;
    let mut splices = Vec::with_capacity(sites.len());
    for s in sites {
        let at = (s.lo as isize + shift) as usize;
        let inserted = after.get(at..at + s.inserted).unwrap_or_default();
        splices.push(Splice {
            lo: s.lo,
            hi: s.hi,
            inserted,
        });
        shift += s.inserted as isize - (s.hi - s.lo) as isize;
    }
    unified(path, before, after, &splices)
}
