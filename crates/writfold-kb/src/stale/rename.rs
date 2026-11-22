//! Following a cited path that is absent at the target commit and whose
//! blob is nowhere in its tree: a rename with edits (design section 6.2,
//! step 5). One `diff-tree -M` spawn per distinct cited `rev`, capped per
//! run, then one `cat-file --batch` for every renamed file to search. A
//! renamed citation is `moved` only when the cited span digest is found in
//! the new file; it is never `fresh`.
use super::span::{Located, locate};
use super::{Cite, Verdict};
use crate::error::Result;
use crate::workspace::{CANCELLED, budgets};
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::verdict::{Absence, Verdict as Kernel, missing};
use writfold_domain::GitObjectId;
use writfold_runtime::git::{
    GitExecutable, KbRepository, Renames, TreeBlob, kb_renames, pinned_blobs,
};

/// Distinct cited revisions followed per `kb stale` run; citations of the
/// rest are unverifiable (`rename-unchecked`) and say so.
pub const REV_CAP: usize = 64;

/// The verdict for a path absent at the target, decided by the kernel's
/// [`missing`] (#90): `path` is where a followed rename leads, `cited` the
/// cited lines, `shallow` whether this clone is shallow (named in the
/// reason when the cited commit is absent).
pub fn decide(absence: Absence, path: &str, cited: (u32, u32), shallow: bool) -> Verdict {
    let detail = |text: String| Some(text);
    match (missing(absence), absence) {
        (Kernel::Moved, Absence::Followed(located)) => {
            let (first, last) = match located {
                Some(Located::At(first, last)) => (first, last),
                _ => cited,
            };
            Verdict::Moved {
                path: Some(path.to_owned()),
                first,
                last,
            }
        }
        (Kernel::Gone, Absence::Followed(located)) => Verdict::Gone {
            detail: detail(format!(
                "renamed to {path}; {}",
                match located {
                    Some(Located::Unsearched) => "span unsearched: file too long",
                    Some(_) => "span not found",
                    None => "no span digest to search",
                }
            )),
        },
        (Kernel::Gone, _) => Verdict::Gone {
            detail: detail("path absent; no rename since the cited rev".into()),
        },
        (_, Absence::RevAbsent) => Verdict::Unverifiable(history(shallow)),
        (_, Absence::NoRev) => Verdict::Unverifiable(
            "nothing-to-compare: path absent and no rev= to follow a rename from".into(),
        ),
        (_, Absence::Capped) => Verdict::Unverifiable(format!(
            "rename-unchecked: over {REV_CAP} cited revs this run"
        )),
        _ => Verdict::Unverifiable("path absent".into()),
    }
}

/// The reason for a citation whose cited commit is not in this clone.
pub fn history(shallow: bool) -> String {
    let shallow = if shallow {
        " (shallow clone: fetch --unshallow)"
    } else {
        " (fetch it)"
    };
    format!("history-not-local: the cited rev is not in this clone{shallow}")
}

/// Citations to follow, by cited revision.
pub type ByRev = BTreeMap<String, Vec<usize>>;

/// Group `absent` by cited revision, spending at most `remaining` new
/// revisions. Citations without a `rev=`, or past the cap, are decided.
pub fn plan(
    cites: &[Cite],
    absent: &[usize],
    remaining: &mut usize,
) -> (ByRev, Vec<(usize, Verdict)>) {
    let mut by_rev = ByRev::new();
    let mut decided = Vec::new();
    for &index in absent {
        let Some(rev) = cites[index].rev.as_deref().map(str::to_lowercase) else {
            decided.push((index, decide(Absence::NoRev, "", (1, 1), false)));
            continue;
        };
        if let Some(members) = by_rev.get_mut(&rev) {
            members.push(index);
        } else if *remaining > 0 {
            *remaining -= 1;
            by_rev.insert(rev, vec![index]);
        } else {
            decided.push((index, decide(Absence::Capped, "", (1, 1), false)));
        }
    }
    (by_rev, decided)
}

/// The repository a follow reads, and whether it is a shallow clone.
#[derive(Clone, Copy)]
pub struct Repo<'a> {
    pub dir: &'a Path,
    pub shallow: bool,
}

/// Follow every absent citation's path from its cited revision to `target`.
pub fn follow(
    git: &GitExecutable<'_>,
    clone: Repo<'_>,
    target: &GitObjectId,
    tree: &[TreeBlob],
    cites: &[Cite],
    absent: &[usize],
    remaining: &mut usize,
) -> Result<Vec<(usize, Verdict)>> {
    let (by_rev, mut decided) = plan(cites, absent, remaining);
    let reads = KbRepository {
        git_dir: clone.dir,
        work_tree: None,
    };
    let mut renamed: Vec<(usize, String, GitObjectId)> = Vec::new();
    for (rev, members) in by_rev {
        let map = kb_renames(git, reads, &rev, target.as_str(), budgets()?, &CANCELLED)?;
        for index in members {
            let old = cites[index].cpath.as_deref().unwrap_or_default();
            let cited = (
                cites[index].first.unwrap_or(1),
                cites[index].last.unwrap_or(1),
            );
            let Renames::Found(map) = &map else {
                decided.push((index, decide(Absence::RevAbsent, "", cited, clone.shallow)));
                continue;
            };
            let Some(new) = map.get(old) else {
                decided.push((index, decide(Absence::NoRename, "", cited, false)));
                continue;
            };
            match tree.iter().find(|entry| &entry.path == new) {
                Some(entry) => renamed.push((index, new.clone(), entry.blob.clone())),
                None => {
                    let absence = Absence::Followed(Some(Located::Absent));
                    decided.push((index, decide(absence, new, cited, false)));
                }
            }
        }
    }
    let ids: Vec<GitObjectId> = renamed.iter().map(|(_, _, id)| id.clone()).collect();
    let contents = if ids.is_empty() {
        Vec::new()
    } else {
        pinned_blobs(git, clone.dir, &ids, budgets()?, &CANCELLED)?
    };
    for ((index, path, _), bytes) in renamed.iter().zip(contents) {
        let cite = &cites[*index];
        let cited = (cite.first.unwrap_or(1), cite.last.unwrap_or(1));
        let located = match (&cite.span, cite.first, cite.last) {
            (Some(span), Some(first), Some(last)) => {
                Some(locate(&String::from_utf8_lossy(&bytes), first, last, span))
            }
            _ => None,
        };
        decided.push((
            *index,
            decide(Absence::Followed(located), path, cited, false),
        ));
    }
    Ok(decided)
}

#[cfg(test)]
mod tests {
    use super::{Absence, Cite, Located, REV_CAP, Verdict, decide, plan};

    #[test]
    fn a_rename_is_never_fresh() {
        let cases = [
            None,
            Some(Located::Same),
            Some(Located::At(4, 6)),
            Some(Located::Absent),
            Some(Located::Unsearched),
        ];
        for located in cases {
            let verdict = decide(Absence::Followed(located), "src/new.rs", (1, 3), false);
            assert_ne!(verdict, Verdict::Fresh);
            let moved = matches!(verdict, Verdict::Moved { .. });
            let found = matches!(located, Some(Located::Same | Located::At(_, _)));
            assert_eq!(moved, found);
        }
        assert_eq!(
            decide(
                Absence::Followed(Some(Located::At(4, 6))),
                "src/new.rs",
                (1, 3),
                false
            ),
            Verdict::Moved {
                path: Some("src/new.rs".into()),
                first: 4,
                last: 6
            }
        );
        assert_eq!(
            decide(Absence::Followed(Some(Located::Absent)), "p", (1, 3), false),
            Verdict::Gone {
                detail: Some("renamed to p; span not found".into())
            }
        );
    }

    /// #90: an unavailable follow is unverifiable with its reason, never gone.
    #[test]
    fn missing_history_is_not_a_removal() {
        let reason = |a, shallow| match decide(a, "", (1, 1), shallow) {
            Verdict::Unverifiable(why) => why,
            other => format!("not unverifiable: {other:?}"),
        };
        assert!(reason(Absence::RevAbsent, true).starts_with("history-not-local:"));
        assert!(reason(Absence::RevAbsent, true).contains("shallow clone"));
        assert!(!reason(Absence::RevAbsent, false).contains("shallow"));
        assert!(reason(Absence::NoRev, false).starts_with("nothing-to-compare:"));
        assert!(reason(Absence::Capped, false).starts_with("rename-unchecked:"));
        assert_eq!(decide(Absence::NoRename, "", (1, 1), false).name(), "gone");
    }

    #[test]
    fn one_spawn_per_distinct_rev_up_to_the_cap() {
        let cite = |rev: Option<String>| Cite {
            rev,
            ..Cite::default()
        };
        let mut cites: Vec<Cite> = (0..REV_CAP + 3)
            .map(|n| cite(Some(format!("{n:07x}"))))
            .collect();
        cites.push(cite(Some("0000000".into())));
        cites.push(cite(Some("000000A".into())));
        cites.push(cite(None));
        let all: Vec<usize> = (0..cites.len()).collect();
        let mut remaining = REV_CAP;
        let (by_rev, decided) = plan(&cites, &all, &mut remaining);
        assert_eq!(remaining, 0);
        assert_eq!(by_rev.len(), REV_CAP);
        assert_eq!(by_rev.get("0000000").map(Vec::len), Some(2));
        assert_eq!(by_rev.get("000000a").map(Vec::len), Some(2));
        assert_eq!(decided.len(), 4);
        assert!(decided.iter().all(|(_, v)| v.name() == "unverifiable"));
        let mut none = 0;
        let (by_rev, decided) = plan(&cites, &all[..2], &mut none);
        assert!(by_rev.is_empty() && decided.len() == 2);
    }
}
