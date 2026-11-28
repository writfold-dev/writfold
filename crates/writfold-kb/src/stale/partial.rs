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

const NOT_LOCAL: &str = "blob-not-local: this partial clone lacks the file's bytes; `writfold kb repos --need-blobs` lists them to fetch";

/// A cited path absent at the target whose blob is nowhere in its tree: a
/// full clone follows it through history as a rename with edits.
const NOT_FOLLOWED: &str = "path-not-followed: absent at the target and not an exact rename; a partial clone does not follow it, use a full clone";

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

/// A partial clone's verdicts and the blobs it lacked to decide them.
pub struct Decided {
    pub verdicts: Vec<(usize, Verdict)>,
    /// Per `blob-not-local` verdict, the blob at the target that would
    /// decide it (#215).
    pub need: Vec<(usize, GitObjectId)>,
}

/// The verdicts of a partial clone's code citations at `target`.
pub fn check(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    target: &GitObjectId,
    cites: &[Cite],
    members: &[usize],
) -> Result<Decided> {
    let tree = match pinned_tree(git, git_dir, target, budgets()?, &CANCELLED) {
        Ok(tree) => tree,
        Err(_) => {
            let why = "tree-not-local: this partial clone lacks the target's tree";
            let verdicts = (members.iter())
                .map(|&i| (i, Verdict::Unverifiable(why.into())))
                .collect();
            return Ok(Decided {
                verdicts,
                need: Vec::new(),
            });
        }
    };
    let mut verdicts = Vec::new();
    let other = by_id(&tree, cites, members, &mut verdicts);
    let ids: Vec<GitObjectId> = other.iter().map(|(_, id)| id.clone()).collect();
    let here = local(git, git_dir, &ids)?;
    let readable: Vec<GitObjectId> = (ids.iter().zip(&here))
        .filter(|(_, h)| **h)
        .map(|(id, _)| id.clone())
        .collect();
    let mut bytes = pinned_blobs(git, git_dir, &readable, budgets()?, &CANCELLED)?.into_iter();
    let mut need = Vec::new();
    for ((i, id), here) in other.iter().zip(here) {
        let cite = &cites[*i];
        let known = cite.blob.is_some();
        if !here {
            let k = other_blob(known, Some(Located::Unfetched));
            verdicts.push((*i, word(k, NOT_LOCAL)));
            need.push((*i, id.clone()));
            continue;
        }
        let text = bytes.next().unwrap_or_default();
        verdicts.push((*i, from_bytes(cite, &String::from_utf8_lossy(&text))));
    }
    Ok(Decided { verdicts, need })
}

/// The citations decided by blob id alone (into `out`), and the rest with
/// the blob id at their path, which need the blob's bytes.
fn by_id(
    tree: &[writfold_runtime::git::TreeBlob],
    cites: &[Cite],
    members: &[usize],
    out: &mut Vec<(usize, Verdict)>,
) -> Vec<(usize, GitObjectId)> {
    let mut other = Vec::new();
    for &i in members {
        let cite = &cites[i];
        let path = cite.cpath.as_deref().unwrap_or_default();
        let now = tree.iter().find(|e| e.path == path).map(|e| e.blob.clone());
        match (now, cite.blob.as_deref()) {
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
                        k => word(k, NOT_FOLLOWED),
                    },
                )),
                None => out.push((i, word(missing(Absence::Unfetched), NOT_FOLLOWED))),
            },
            (None, None) => out.push((i, word(missing(Absence::Unfetched), NOT_FOLLOWED))),
        }
    }
    other
}

/// A citation whose blob differs, decided from the bytes at its path.
fn from_bytes(cite: &Cite, text: &str) -> Verdict {
    let located = match (&cite.span, cite.first, cite.last) {
        (Some(span), Some(first), Some(last)) => Some(locate(text, first, last, span)),
        _ => None,
    };
    match (other_blob(cite.blob.is_some(), located), located) {
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
    }
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
        .verdicts
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

/// A partial clone's code citations decided at the target `stale` chooses,
/// for `kb repos` (#215); `None` when no target is chosen.
pub fn decided(
    git: &GitExecutable<'_>,
    (git_dir, ask): (&Path, &super::target::Asked<'_>),
    cites: &[Cite],
) -> Result<Option<Decided>> {
    let target = super::target::resolve(git, git_dir, ask)?;
    let Some(commit) = target.commit else {
        return Ok(None);
    };
    let members: Vec<usize> = (0..cites.len()).collect();
    check(git, git_dir, &commit, cites, &members).map(Some)
}
