//! Code citations into a declared attached store ([`crate::store`]).
//! The store's clone is found by the proved store resolution; a citation
//! the proved [`writfold_core::store::basis`] puts on the pin is judged
//! at its own revision by [`writfold_core::store::pinned`], never at the
//! store's head: one `cat-file --batch-check` for every cited commit and
//! `rev:path`, and one `for-each-ref --contains` per distinct commit that
//! is present. The rest of the store's citations are judged at head as for
//! any repository.
use super::{Checked, Cite, Verdict};
use crate::error::Result;
use crate::session::Session;
use crate::store::{Freshness, Store};
use crate::workspace::{CANCELLED, budgets};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use writfold_core::store::{self as kernel, AtPin, Basis, Commit};
use writfold_core::verdict::Verdict as Kernel;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_anchored, kb_batch_check};

/// A store's citations: the clone to read and those judged at head, or
/// every citation answered when there is no clone to read.
pub enum Split {
    Read {
        dir: PathBuf,
        head: Vec<usize>,
        pinned: Vec<usize>,
    },
    Answered(Vec<(usize, Checked)>),
}

fn unverifiable(reason: String) -> Checked {
    Checked {
        verdict: Verdict::Unverifiable(reason),
        head: "none".into(),
        recorded: None,
    }
}

/// Resolve `store` and divide its citations by the rule that judges them.
pub fn split(session: &Session<'_>, store: &Store, cites: &[Cite], members: Vec<usize>) -> Split {
    let found = crate::store::resolve(Some(session.ws), store);
    let Some(dir) = found.git_dir else {
        let file = format!("{}/{}.toml", crate::store::DIR, store.name);
        let reason = match found.other_origin {
            Some(other) => format!(
                "store-origin-mismatch: a clone of {other}, not {}",
                store.url
            ),
            None => format!(
                "store-not-local: declared in {file}; clone {} and map it",
                store.url
            ),
        };
        return Split::Answered(
            members
                .iter()
                .map(|&i| (i, unverifiable(reason.clone())))
                .collect(),
        );
    };
    let pinned = store.freshness == Freshness::Pinned;
    let (pinned, head) = members.into_iter().partition(|&i| {
        let cite = &cites[i];
        cite.kind.as_deref() != Some("gate")
            && matches!(kernel::basis(true, pinned, cite.rev.is_some()), Basis::Pin)
    });
    Split::Read { dir, head, pinned }
}

/// A full commit ID: forty or sixty-four lowercase hex digits.
pub(crate) fn full(rev: &str) -> bool {
    matches!(rev.len(), 40 | 64)
        && rev
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The verdicts of citations judged at their pin.
pub fn check(
    git: &GitExecutable<'_>,
    dir: &Path,
    cites: &[Cite],
    members: &[usize],
) -> Result<Vec<(usize, Checked)>> {
    if super::repos::partial(dir) {
        // A partial clone may lack a cited blob that the store still holds.
        return Ok(members
            .iter()
            .map(|&i| (i, unverifiable("partial-clone".into())))
            .collect());
    }
    let reads = KbRepository {
        git_dir: dir,
        work_tree: None,
    };
    let mut out = Vec::new();
    let mut asked = Vec::new();
    let mut names = Vec::new();
    for &index in members {
        let cite = &cites[index];
        let rev = cite.rev.as_deref().unwrap_or_default().to_ascii_lowercase();
        let path = cite.cpath.as_deref().filter(|p| super::code::valid_path(p));
        match path {
            Some(path) if full(&rev) => {
                names.push(rev.clone());
                names.push(format!("{rev}:{path}"));
                asked.push((index, rev));
            }
            Some(_) => out.push((
                index,
                unverifiable("a pinned store's citation needs a full commit rev=".into()),
            )),
            None => out.push((index, unverifiable("missing or invalid path=".into()))),
        }
    }
    if asked.is_empty() {
        return Ok(out);
    }
    let answers = kb_batch_check(git, reads, &names, budgets()?, &CANCELLED)?;
    let mut anchored: BTreeMap<String, bool> = BTreeMap::new();
    for (n, (index, rev)) in asked.iter().enumerate() {
        let commit = answers
            .get(2 * n)
            .cloned()
            .flatten()
            .filter(|(_, kind)| kind == "commit");
        let commit = match commit {
            None => Commit::Absent,
            Some((id, _)) => {
                let reached = match anchored.get(rev) {
                    Some(known) => *known,
                    None => {
                        let found = kb_anchored(git, reads, &id, budgets()?, &CANCELLED)?;
                        anchored.insert(rev.clone(), found);
                        found
                    }
                };
                if reached {
                    Commit::Anchored
                } else {
                    Commit::Unanchored
                }
            }
        };
        let blob = answers
            .get(2 * n + 1)
            .cloned()
            .flatten()
            .filter(|(_, kind)| kind == "blob");
        let at = match (blob, cites[*index].blob.as_deref()) {
            (None, _) => AtPin::Nothing,
            (Some((id, _)), Some(cited)) if !id.as_str().eq_ignore_ascii_case(cited) => {
                AtPin::Other
            }
            (Some(_), _) => AtPin::Cited,
        };
        out.push((*index, verdict(&commit, &at, rev)));
    }
    Ok(out)
}

fn verdict(commit: &Commit, at: &AtPin, rev: &str) -> Checked {
    let detail = match (commit, at) {
        (Commit::Unanchored, _) => "pinned commit is reached by no ref",
        (_, AtPin::Other) => "pinned path holds other bytes",
        _ => "pinned path is absent at its commit",
    };
    let verdict = match kernel::pinned(commit, at) {
        Kernel::Fresh => Verdict::Fresh,
        Kernel::Gone => Verdict::Gone {
            detail: Some(detail.into()),
        },
        _ => Verdict::Unverifiable("pinned commit is not in the local clone (fetch it)".into()),
    };
    Checked {
        verdict,
        head: format!("{}:pin", rev.get(..12).unwrap_or(rev)),
        recorded: None,
    }
}
