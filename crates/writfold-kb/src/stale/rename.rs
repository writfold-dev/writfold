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
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, TreeBlob, kb_renames, pinned_blobs};

/// Distinct cited revisions followed per `kb stale` run; citations of the
/// rest stay `gone` and say so.
pub const REV_CAP: usize = 64;

/// The verdict for a renamed citation from where its span sits in the new
/// file (`None` when the citation carries no span digest or range), and
/// the cited lines.
pub fn decide(path: &str, located: Option<Located>, cited: (u32, u32)) -> Verdict {
    let gone = |why: &str| Verdict::Gone {
        detail: Some(format!("renamed to {path}; {why}")),
    };
    let moved = |(first, last)| Verdict::Moved {
        path: Some(path.to_owned()),
        first,
        last,
    };
    match located {
        Some(Located::Same) => moved(cited),
        Some(Located::At(first, last)) => moved((first, last)),
        Some(Located::Absent) => gone("span not found"),
        Some(Located::Unsearched) => gone("span unsearched: file too long"),
        None => gone("no span digest to search"),
    }
}

fn gone(detail: &str) -> Verdict {
    Verdict::Gone {
        detail: Some(format!("path absent; {detail}")),
    }
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
            decided.push((index, gone("no rev= to follow a rename")));
            continue;
        };
        if let Some(members) = by_rev.get_mut(&rev) {
            members.push(index);
        } else if *remaining > 0 {
            *remaining -= 1;
            by_rev.insert(rev, vec![index]);
        } else {
            let detail = format!("rename-unchecked (over {REV_CAP} cited revs this run)");
            decided.push((index, gone(&detail)));
        }
    }
    (by_rev, decided)
}

/// Follow every absent citation's path from its cited revision to `target`.
pub fn follow(
    git: &GitExecutable<'_>,
    dir: &Path,
    target: &GitObjectId,
    tree: &[TreeBlob],
    cites: &[Cite],
    absent: &[usize],
    remaining: &mut usize,
) -> Result<Vec<(usize, Verdict)>> {
    let (by_rev, mut decided) = plan(cites, absent, remaining);
    let reads = KbRepository {
        git_dir: dir,
        work_tree: None,
    };
    let mut renamed: Vec<(usize, String, GitObjectId)> = Vec::new();
    for (rev, members) in by_rev {
        let map = kb_renames(git, reads, &rev, target.as_str(), budgets()?, &CANCELLED)?;
        for index in members {
            let old = cites[index].cpath.as_deref().unwrap_or_default();
            let Some(map) = &map else {
                decided.push((index, gone("cited rev not in repository")));
                continue;
            };
            let Some(new) = map.get(old) else {
                decided.push((index, gone("no rename since the cited rev")));
                continue;
            };
            match tree.iter().find(|entry| &entry.path == new) {
                Some(entry) => renamed.push((index, new.clone(), entry.blob.clone())),
                None => decided.push((index, decide(new, Some(Located::Absent), (0, 0)))),
            }
        }
    }
    let ids: Vec<GitObjectId> = renamed.iter().map(|(_, _, id)| id.clone()).collect();
    let contents = if ids.is_empty() {
        Vec::new()
    } else {
        pinned_blobs(git, dir, &ids, budgets()?, &CANCELLED)?
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
        decided.push((*index, decide(path, located, cited)));
    }
    Ok(decided)
}

#[cfg(test)]
mod tests {
    use super::{Cite, Located, REV_CAP, Verdict, decide, plan};

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
            let verdict = decide("src/new.rs", located, (1, 3));
            assert_ne!(verdict, Verdict::Fresh);
            let moved = matches!(verdict, Verdict::Moved { .. });
            let found = matches!(located, Some(Located::Same | Located::At(_, _)));
            assert_eq!(moved, found);
        }
        assert_eq!(
            decide("src/new.rs", Some(Located::Same), (1, 3)),
            Verdict::Moved {
                path: Some("src/new.rs".into()),
                first: 1,
                last: 3
            }
        );
        assert_eq!(
            decide("src/new.rs", Some(Located::At(4, 6)), (1, 3)),
            Verdict::Moved {
                path: Some("src/new.rs".into()),
                first: 4,
                last: 6
            }
        );
        assert_eq!(
            decide("p", Some(Located::Absent), (1, 3)),
            Verdict::Gone {
                detail: Some("renamed to p; span not found".into())
            }
        );
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
        assert!(decided.iter().all(|(_, v)| v.name() == "gone"));
        let mut none = 0;
        let (by_rev, decided) = plan(&cites, &all[..2], &mut none);
        assert!(by_rev.is_empty() && decided.len() == 2);
    }
}
