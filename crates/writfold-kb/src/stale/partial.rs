//! Code citations in a partial clone, decided by blob id first (#182, RFC
//! 2026-11-27-partial-clone-identity-first). A blobless clone holds every
//! tree, so the blob id at a cited path is known without the blob: the
//! same id is fresh, and a path absent at the target whose cited blob sits
//! at another path is an exact rename. Another id is decided from its bytes
//! when they are in the clone, and otherwise reads what the proved kernel
//! says of bytes the clone lacks: `unverifiable`, never changed or gone.
//!
//! Tested, not proved: listing the tree, asking which blobs are local
//! (`cat-file --batch-check` on blob ids with lazy fetch disabled) and the
//! strings.
use super::span::locate;
use super::{Checked, Cite, Verdict};
use crate::error::Result;
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_core::verdict::{Absence, Located, Verdict as Kernel, missing, other_blob};
use writfold_domain::GitObjectId;
use writfold_runtime::git::{
    GitError, GitExecutable, KbRepository, kb_batch_check, pinned_blobs, pinned_tree,
};
use writfold_runtime::process::ProcessError;

const NOT_LOCAL: &str =
    "blob-not-local: this partial clone lacks the file's bytes; fetch them or use a full clone";

fn word(k: Kernel, why: &str) -> Verdict {
    match k {
        Kernel::Fresh => Verdict::Fresh,
        Kernel::Unverifiable => Verdict::Unverifiable(why.to_owned()),
        Kernel::Changed => Verdict::Changed { detail: None },
        Kernel::Gone => Verdict::Gone { detail: None },
        _ => Verdict::Unverifiable(why.to_owned()),
    }
}

/// Which of `ids` are in the clone (not merely promised). With lazy
/// fetch disabled Git refuses a promised object outright, so each id is
/// asked on its own and a refusal reads "not here"; any other failure is
/// an error.
fn local(git: &GitExecutable<'_>, git_dir: &Path, ids: &[GitObjectId]) -> Result<Vec<bool>> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    let mut out = Vec::new();
    for id in ids {
        let names = [id.as_str().to_owned()];
        out.push(
            match kb_batch_check(git, reads, &names, budgets()?, &CANCELLED) {
                Ok(answers) => answers.first().is_some_and(Option::is_some),
                Err(GitError::Process(ProcessError::Command { .. })) => false,
                Err(error) => return Err(error.into()),
            },
        );
    }
    Ok(out)
}

/// The verdicts of a partial clone's code citations at `target`.
pub fn check(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    target: &GitObjectId,
    cites: &[Cite],
    members: &[usize],
) -> Result<Vec<(usize, Verdict)>> {
    let tree = match pinned_tree(git, git_dir, target, budgets()?, &CANCELLED) {
        Ok(tree) => tree,
        Err(_) => {
            let why = "tree-not-local: this partial clone lacks the target's tree";
            return Ok(members
                .iter()
                .map(|&i| (i, Verdict::Unverifiable(why.into())))
                .collect());
        }
    };
    let mut out = Vec::new();
    let mut other: Vec<(usize, GitObjectId)> = Vec::new();
    for &i in members {
        let cite = &cites[i];
        let path = cite.cpath.as_deref().unwrap_or_default();
        let now = tree.iter().find(|e| e.path == path).map(|e| e.blob.clone());
        let cited = cite.blob.as_deref();
        match (now, cited) {
            (Some(id), Some(b)) if id.as_str() == b => out.push((i, Verdict::Fresh)),
            (Some(id), _) => other.push((i, id)),
            (None, Some(b)) => match tree.iter().find(|e| e.blob.as_str() == b) {
                Some(entry) => out.push((
                    i,
                    match missing(Absence::Renamed) {
                        Kernel::Renamed => Verdict::Moved {
                            path: Some(entry.path.clone()),
                            first: cite.first.unwrap_or(1),
                            last: cite.last.unwrap_or(1),
                        },
                        k => word(k, NOT_LOCAL),
                    },
                )),
                None => out.push((i, word(missing(Absence::Unfetched), NOT_LOCAL))),
            },
            (None, None) => out.push((i, word(missing(Absence::Unfetched), NOT_LOCAL))),
        }
    }
    let ids: Vec<GitObjectId> = other.iter().map(|(_, id)| id.clone()).collect();
    let here = local(git, git_dir, &ids)?;
    let readable: Vec<GitObjectId> = (ids.iter().zip(&here))
        .filter(|(_, h)| **h)
        .map(|(id, _)| id.clone())
        .collect();
    let mut bytes = pinned_blobs(git, git_dir, &readable, budgets()?, &CANCELLED)?.into_iter();
    for ((i, _), here) in other.iter().zip(here) {
        let cite = &cites[*i];
        let known = cite.blob.is_some();
        if !here {
            out.push((
                *i,
                word(other_blob(known, Some(Located::Unfetched)), NOT_LOCAL),
            ));
            continue;
        }
        let text = bytes.next().unwrap_or_default();
        let text = String::from_utf8_lossy(&text);
        let located = match (&cite.span, cite.first, cite.last) {
            (Some(span), Some(first), Some(last)) => Some(locate(&text, first, last, span)),
            _ => None,
        };
        let verdict = match (other_blob(known, located), located) {
            (Kernel::Fresh, _) => Verdict::Fresh,
            (Kernel::Moved, Some(Located::At(a, b))) => Verdict::Moved {
                path: None,
                first: a,
                last: b,
            },
            (k, _) => word(
                k,
                "nothing-to-compare: no blob= to compare with in this partial clone",
            ),
        };
        out.push((*i, verdict));
    }
    Ok(out)
}

/// A partial clone's code citations: the target as `stale` chooses it,
/// then [`check`], then what a target behind the cited commit supports.
pub fn resolved(
    git: &GitExecutable<'_>,
    (git_dir, ask): (&Path, &super::target::Asked<'_>),
    cites: &[Cite],
    members: &[usize],
) -> Result<Vec<(usize, Checked)>> {
    let target = super::target::resolve(git, git_dir, ask)?;
    let Some(commit) = target.commit.clone() else {
        let why = target
            .why
            .clone()
            .unwrap_or_else(|| "repository has no commit".into());
        return Ok(members
            .iter()
            .map(|&i| {
                (
                    i,
                    Checked {
                        verdict: Verdict::Unverifiable(why.clone()),
                        head: "none".into(),
                        recorded: None,
                    },
                )
            })
            .collect());
    };
    let head = format!("{}:{}", commit.as_str(), target.rule);
    let found: Vec<(usize, Checked)> = check(git, git_dir, &commit, cites, members)?
        .into_iter()
        .map(|(i, verdict)| {
            (
                i,
                Checked {
                    verdict,
                    head: head.clone(),
                    recorded: None,
                },
            )
        })
        .collect();
    super::behind::behind(git, git_dir, &target, cites, found)
}
