//! What a chosen target can support (#179): each verdict that says a
//! citation changed, moved or went is settled by the proved
//! [`writfold_core::referent::settle`] against how the cited commit relates
//! to the target, one ancestry check per distinct cited commit.
//!
//! Tested, not proved: the ancestry walk and the verdict-word mapping.
use super::target::{Resolved, commit};
use super::{Checked, Cite, Verdict, rename};
use crate::error::Result;
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_core::referent::{Relation, settle};
use writfold_core::verdict::Verdict as Kernel;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_is_ancestor};

/// The kernel's word for a verdict.
pub fn word(v: &Verdict) -> Kernel {
    match v {
        Verdict::Fresh => Kernel::Fresh,
        Verdict::Moved { path: Some(_), .. } => Kernel::Renamed,
        Verdict::Moved { path: None, .. } => Kernel::Moved,
        Verdict::Changed { .. } => Kernel::Changed,
        Verdict::Gone { .. } => Kernel::Gone,
        Verdict::Expired => Kernel::Expired,
        Verdict::Unverifiable(_) => Kernel::Unverifiable,
        Verdict::Marked(_) => Kernel::Marked,
        Verdict::Due => Kernel::Due,
    }
}

/// How the cited commit relates to the target.
pub fn relation(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    target: &GitObjectId,
    rev: &str,
) -> Result<Relation> {
    let Some(cited) = commit(git, git_dir, rev)? else {
        return Ok(Relation::Unknown);
    };
    if &cited == target {
        return Ok(Relation::Contains);
    }
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    let (b, c) = (budgets()?, &CANCELLED);
    if kb_is_ancestor(git, reads, &cited, target, b, c)? {
        return Ok(Relation::Contains);
    }
    if kb_is_ancestor(git, reads, target, &cited, b, c)? {
        return Ok(Relation::Behind);
    }
    Ok(Relation::Diverged)
}

/// The verdict a target that is `rel` to the cited commit can support,
/// through the proved [`settle`].
pub fn settled(v: Verdict, rel: Relation, rule: &str) -> Verdict {
    let kernel = word(&v);
    match settle(true, rel, kernel) {
        k if k == kernel => v,
        _ => Verdict::Unverifiable(format!(
            "target-behind: the target {rule} is an ancestor of the cited rev; fetch"
        )),
    }
}

/// The cited commit of a code citation (`rev=`) or a gate citation
/// (`rev=<repo>@<sha>`).
fn cited_rev(cite: &Cite) -> Option<&str> {
    let rev = cite.rev.as_deref()?;
    Some(match cite.kind.as_deref() {
        Some("gate") => rev.split_once('@')?.1,
        _ => rev,
    })
}

/// Settle each verdict that says something changed against how its cited
/// commit relates to the target: one ancestry check per distinct cited
/// commit, at most [`rename::REV_CAP`] per repository.
pub fn behind(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    resolved: &Resolved,
    cites: &[Cite],
    found: Vec<(usize, Checked)>,
) -> Result<Vec<(usize, Checked)>> {
    let Some(target) = &resolved.commit else {
        return Ok(found);
    };
    let mut known: Vec<(String, Relation)> = Vec::new();
    let mut out = Vec::new();
    for (i, mut checked) in found {
        let rev = cites.get(i).and_then(cited_rev);
        if let (Some(rev), true) = (
            rev,
            writfold_core::referent::claims_change(word(&checked.verdict)),
        ) {
            let rel = match known.iter().find(|(r, _)| r == rev) {
                Some((_, rel)) => Some(*rel),
                None if known.len() < rename::REV_CAP => {
                    let rel = relation(git, git_dir, target, rev)?;
                    known.push((rev.to_owned(), rel));
                    Some(rel)
                }
                None => None,
            };
            checked.verdict = match rel {
                Some(rel) => settled(checked.verdict, rel, &resolved.rule),
                None => Verdict::Unverifiable(format!(
                    "relation-unchecked: over {} cited revs in this repository",
                    rename::REV_CAP
                )),
            };
        }
        out.push((i, checked));
    }
    Ok(out)
}
