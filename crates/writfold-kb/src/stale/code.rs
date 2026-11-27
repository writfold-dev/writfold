//! Code and gate citations, three or four Git spawns per cited repository:
//! the target commit, one `--batch-check` for every cited path, one
//! `--batch` for the files whose blob differs, and one tree listing when a
//! path is missing (to find an exact rename by blob). A missing path whose
//! blob is not in the tree is followed as a rename with edits ([`rename`]).
//! A partial clone's code citations are decided by blob id (`partial`): its
//! blobs may need a fetch. A Git failure in one repository makes that
//! repository's citations unverifiable (`git-error: …`), not the run fail.
use super::failed::per_repo_or;
use super::span::{Located, locate};
use super::target::{self as resolver, Asked as Ask};
use super::{Checked, Cite, Verdict, pin_receipt, pinned, rename, repos};
use crate::error::Result;
use crate::session::Session;
use crate::workspace::{CANCELLED, budgets};
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::verdict::{Verdict as Kernel, other_blob};
use writfold_domain::GitObjectId;
use writfold_runtime::git::{
    GitExecutable, KbRepository, kb_batch_check, pinned_blobs, pinned_tree,
};

pub fn check(
    session: &Session<'_>,
    cites: &[Cite],
    indices: &[usize],
) -> Result<Vec<(usize, Checked)>> {
    let mut out = Vec::new();
    let groups = by_repo(cites, indices, &mut out);
    let Some(git) = session.git.as_ref() else {
        out.extend(
            groups
                .into_values()
                .flatten()
                .map(|i| (i, missing("no git executable"))),
        );
        return Ok(out);
    };
    let stores = crate::store::read(&session.ws.root)?;
    let declared = crate::declared::read(&session.ws.root)?;
    let mut remaining = rename::REV_CAP;
    for (repo, members) in groups {
        let (dir, members, url, named) = match stores.get(&repo) {
            Some(store) => {
                // A pinned citation this machine cannot judge may read a
                // recorded verdict instead (#4).
                let stand_in = |(i, c): (usize, Checked)| {
                    let root = &session.ws.root;
                    (
                        i,
                        pin_receipt::stand_in(root, store, &cites[i], c, session.now),
                    )
                };
                match pinned::split(session, store, cites, members) {
                    pinned::Split::Answered(found) => {
                        out.extend(found.into_iter().map(stand_in));
                        continue;
                    }
                    pinned::Split::Read { dir, head, pinned } => {
                        let found = pinned::check(git, &dir, cites, &pinned);
                        out.extend(per_repo_or(found, &pinned)?.into_iter().map(stand_in));
                        (dir, head, Some(store.url.clone()), true)
                    }
                }
            }
            None => {
                let url = members.iter().find_map(|&i| cites[i].field("url"));
                let Some((dir, via)) = repos::resolve_via(Some(session.ws), &repo, url.as_deref())
                else {
                    out.extend(members.iter().map(|&i| (i, missing("repo-not-local"))));
                    continue;
                };
                let named = matches!(via, repos::Via::Map | repos::Via::Path);
                (dir, members, url, named)
            }
        };
        let pin = repos::mapped_ref(Some(session.ws), &repo);
        let ask = Ask {
            repo: &repo,
            url: url.as_deref(),
            pin: pin.as_deref(),
            named,
            declared: crate::declared::find(&declared, &repo),
        };
        let (gates, members): (Vec<usize>, Vec<usize>) = members
            .into_iter()
            .partition(|&i| cites[i].kind.as_deref() == Some("gate"));
        // A partial clone's code citations are decided by blob id (#182).
        let (code, members) = match repos::partial(&dir) {
            true => (members, gates),
            false => (Vec::new(), [gates, members].concat()),
        };
        let found = check_repo(git, (&dir, &ask), cites, &members, &mut remaining);
        out.extend(per_repo_or(found, &members)?);
        if !code.is_empty() {
            let found = super::partial::resolved(git, (&dir, &ask), cites, &code);
            out.extend(per_repo_or(found, &code)?);
        }
    }
    Ok(out)
}

/// The citations grouped by the repository they name (a gate citation by
/// its `rev=<repo>@…`); one without a repository is unverifiable in `out`.
fn by_repo(
    cites: &[Cite],
    indices: &[usize],
    out: &mut Vec<(usize, Checked)>,
) -> BTreeMap<String, Vec<usize>> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for &index in indices {
        let cite = &cites[index];
        let repo = match cite.kind.as_deref() {
            Some("gate") => cite
                .rev
                .as_deref()
                .and_then(|r| r.split_once('@'))
                .map(|(repo, _)| repo.to_owned()),
            _ => cite.repo.clone(),
        };
        match repo {
            Some(repo) => groups.entry(repo).or_default().push(index),
            None => out.push((index, missing("missing repo="))),
        }
    }
    groups
}

pub(super) fn missing(reason: &str) -> Checked {
    Checked {
        verdict: Verdict::Unverifiable(reason.into()),
        head: "none".into(),
        recorded: None,
    }
}

pub(crate) fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\n')
        && !path.split('/').any(|part| part == "..")
}

fn check_repo(
    git: &GitExecutable<'_>,
    (dir, ask): (&Path, &Ask<'_>),
    cites: &[Cite],
    members: &[usize],
    remaining: &mut usize,
) -> Result<Vec<(usize, Checked)>> {
    let reads = KbRepository {
        git_dir: dir,
        work_tree: None,
    };
    let resolved = resolver::resolve(git, dir, ask)?;
    let Some(target) = resolved.commit.clone() else {
        let why = resolved
            .why
            .as_deref()
            .unwrap_or("repository has no commit");
        return Ok(members.iter().map(|&i| (i, missing(why))).collect());
    };
    let named = &resolved.rule;
    let head = format!("{}:{named}", target.as_str());
    let mut out = Vec::new();
    let mut names = Vec::new();
    let mut asked = Vec::new();
    for &index in members {
        let cite = &cites[index];
        if cite.kind.as_deref() == Some("gate") {
            let sha = cite
                .rev
                .as_deref()
                .and_then(|r| r.split_once('@'))
                .map(|(_, s)| s.to_lowercase());
            let fresh = sha.is_some_and(|s| s.len() >= 7 && target.as_str().starts_with(&s));
            let verdict = if fresh {
                Verdict::Fresh
            } else {
                Verdict::Changed { detail: None }
            };
            out.push((
                index,
                Checked {
                    verdict,
                    head: head.clone(),
                    recorded: None,
                },
            ));
            continue;
        }
        let Some(path) = cite.cpath.as_deref().filter(|p| valid_path(p)) else {
            out.push((index, missing("missing or invalid path=")));
            continue;
        };
        names.push(format!("{}:{path}", target.as_str()));
        let pinned = cite.rev.as_deref().filter(|_| cite.blob.is_none());
        names.push(pinned.map_or_else(|| "0".repeat(40), |rev| format!("{rev}:{path}")));
        // Whether this clone holds the cited commit at all (#90).
        names.push(pinned.map_or_else(|| "0".repeat(40), |rev| format!("{rev}^{{commit}}")));
        asked.push(index);
    }
    let answers = kb_batch_check(git, reads, &names, budgets()?, &CANCELLED)?;
    let asked = Asked {
        indices: &asked,
        answers: &answers,
    };
    let clone = rename::Repo {
        dir,
        shallow: repos::shallow(dir),
    };
    let verdicts = settle(git, clone, &target, cites, asked, remaining)?;
    out.extend(verdicts.into_iter().map(|(i, verdict)| {
        (
            i,
            Checked {
                verdict,
                head: head.clone(),
                recorded: None,
            },
        )
    }));
    super::behind::behind(git, dir, &resolved, cites, out)
}

type Answer = Option<(GitObjectId, String)>;

/// The citations asked about, with three batch-check answers each: the
/// path at the target, the path at the cited rev, the cited commit.
#[derive(Clone, Copy)]
struct Asked<'a> {
    indices: &'a [usize],
    answers: &'a [Answer],
}

/// Decide each asked citation from the blob answers, reading contents and
/// the tree only where needed.
fn settle(
    git: &GitExecutable<'_>,
    clone: rename::Repo<'_>,
    target: &GitObjectId,
    cites: &[Cite],
    asked: Asked<'_>,
    remaining: &mut usize,
) -> Result<Vec<(usize, Verdict)>> {
    let answers = asked.answers;
    let dir = clone.dir;
    let mut decided = Vec::new();
    let mut read: Vec<(usize, GitObjectId, Option<String>)> = Vec::new();
    let mut absent = Vec::new();
    for (n, &index) in asked.indices.iter().enumerate() {
        let cite = &cites[index];
        let now = answers
            .get(3 * n)
            .cloned()
            .flatten()
            .filter(|(_, kind)| kind == "blob");
        let then = answers
            .get(3 * n + 1)
            .cloned()
            .flatten()
            .map(|(id, _)| id.as_str().to_owned());
        let cited = cite.blob.clone().or(then);
        let rev_here = answers.get(3 * n + 2).is_some_and(Option::is_some);
        match now {
            Some((id, _)) if cited.as_deref() == Some(id.as_str()) => {
                decided.push((index, Verdict::Fresh))
            }
            Some((id, _)) if cited.is_some() => read.push((index, id, None)),
            Some((id, _)) => read.push((index, id, Some(unknown(cite, rev_here, clone.shallow)))),
            None => absent.push(index),
        }
    }
    let ids: Vec<GitObjectId> = read.iter().map(|(_, id, _)| id.clone()).collect();
    let contents = if ids.is_empty() {
        Vec::new()
    } else {
        pinned_blobs(git, dir, &ids, budgets()?, &CANCELLED)?
    };
    for ((index, _, unknown), bytes) in read.iter().zip(contents) {
        let text = String::from_utf8_lossy(&bytes);
        decided.push((*index, by_span(&cites[*index], &text, unknown.as_deref())));
    }
    if !absent.is_empty() {
        let tree = pinned_tree(git, dir, target, budgets()?, &CANCELLED)?;
        let mut edited = Vec::new();
        for index in absent {
            let cite = &cites[index];
            let renamed = cite
                .blob
                .as_deref()
                .and_then(|blob| tree.iter().find(|entry| entry.blob.as_str() == blob));
            match renamed {
                Some(entry) => decided.push((
                    index,
                    Verdict::Moved {
                        path: Some(entry.path.clone()),
                        first: cite.first.unwrap_or(1),
                        last: cite.last.unwrap_or(1),
                    },
                )),
                None => edited.push(index),
            }
        }
        if !edited.is_empty() {
            let followed = rename::follow(git, clone, target, &tree, cites, &edited, remaining)?;
            decided.extend(followed);
        }
    }
    Ok(decided)
}

/// Why no cited blob is known for a citation whose path holds another blob.
fn unknown(cite: &Cite, rev_here: bool, shallow: bool) -> String {
    match &cite.rev {
        Some(_) if !rev_here => rename::history(shallow),
        Some(_) => {
            "nothing-to-compare: the path is not at the cited rev, and there is no blob=".into()
        }
        None => "nothing-to-compare: no blob= or rev= to compare with".into(),
    }
}

/// Another blob at the cited path, decided by the kernel's [`other_blob`]
/// (#90): `unknown` is why no cited blob is known, `None` when one was
/// compared and differs.
fn by_span(cite: &Cite, text: &str, unknown: Option<&str>) -> Verdict {
    let located = match (&cite.span, cite.first, cite.last) {
        (Some(span), Some(first), Some(last)) => Some(locate(text, first, last, span)),
        _ => None,
    };
    let (first, last) = (cite.first.unwrap_or(1), cite.last.unwrap_or(1));
    match other_blob(unknown.is_none(), located) {
        Kernel::Fresh => Verdict::Fresh,
        Kernel::Moved => {
            let (first, last) = match located {
                Some(Located::At(a, b)) => (a, b),
                _ => (first, last),
            };
            Verdict::Moved {
                path: None,
                first,
                last,
            }
        }
        Kernel::Unverifiable => Verdict::Unverifiable(unknown.unwrap_or_default().to_owned()),
        _ if located.is_none() => Verdict::Changed {
            detail: Some("blob differs; no span digest to search".into()),
        },
        _ if located == Some(Located::Unsearched) => Verdict::Changed {
            detail: Some("unsearched: file too long".into()),
        },
        _ => {
            let symbol = cite.field("sym").and_then(|sym| {
                let short = sym
                    .rsplit("::")
                    .next()
                    .unwrap_or(&sym)
                    .rsplit('.')
                    .next()
                    .unwrap_or(&sym)
                    .to_owned();
                text.lines()
                    .position(|line| line.contains(&short))
                    .map(|n| format!("sym {sym} at line {}", n + 1))
            });
            Verdict::Changed { detail: symbol }
        }
    }
}
