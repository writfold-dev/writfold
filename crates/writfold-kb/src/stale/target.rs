//! The Git provider's resolver (#179, RFC 2026-11-27-citation-target): which
//! commit a repository's citations are judged against, chosen by the proved
//! [`writfold_core::target::choose`] from the local map's `ref=` and from how
//! each of the clone's remotes names the cited repository; and what that
//! commit can support, settled by [`writfold_core::referent::settle`].
//!
//! Tested, not proved: reading remotes from the clone's config (`[include]`
//! is not followed), URL normalisation, a remote's tip and ancestry.
use super::repos::common_dir;
use super::{Checked, Cite, Verdict, rename};
use crate::error::Result;
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_core::referent::{Relation, settle};
use writfold_core::target::{Choice, Names, Pinned, Remote, Tip, Why, choose};
use writfold_core::verdict::Verdict as Kernel;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_commit, kb_is_ancestor};

/// The chosen target, or why there is none.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub commit: Option<GitObjectId>,
    /// The ref that gave it (`upstream/main`, `HEAD`, a map `ref=`), or
    /// `none`.
    pub rule: String,
    /// Why this rule: which remote names the repository, or why none was
    /// chosen (`target-ambiguous: …`).
    pub why: Option<String>,
    /// The chosen remote's URL, for a token's `url=`.
    pub url: Option<String>,
}

/// What a citation asks of a clone: the cited repository's name and URL,
/// the map's `ref=`, and whether the clone was named for it (by the map or
/// a path) rather than found by its directory name.
pub struct Asked<'a> {
    pub repo: &'a str,
    pub url: Option<&'a str>,
    pub pin: Option<&'a str>,
    pub named: bool,
}

/// One remote of the clone: its name, URL and default-branch tip.
struct Seen {
    name: String,
    url: Option<String>,
    names: Names,
    tip: Option<(String, GitObjectId)>,
}

/// `host/path` in lower case without a scheme, user, port or trailing
/// `.git` or `/`: equal for the forms of one repository.
pub fn normalize(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = match url.split_once("://") {
        Some((_, rest)) => rest.to_owned(),
        None => match url.split_once(':') {
            // The scp form `user@host:org/name`.
            Some((host, path)) if !host.contains('/') && !path.starts_with('/') => {
                format!("{host}/{path}")
            }
            _ => return url.starts_with('/').then(|| url.to_owned()),
        },
    };
    let rest = rest.rsplit_once('@').map_or(rest.as_str(), |(_, r)| r);
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    let path = path.trim_end_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    (!path.is_empty()).then(|| format!("{host}/{}", path.to_ascii_lowercase()))
}

/// How a remote's URL names the cited repository.
pub fn names(url: Option<&str>, asked: &Asked<'_>, only: bool) -> Names {
    let Some(remote) = url.and_then(normalize) else {
        return Names::Unread;
    };
    if asked.url.and_then(normalize).as_deref() == Some(remote.as_str()) {
        return Names::Url;
    }
    let repo = asked.repo.trim_matches('/').to_ascii_lowercase();
    if repo.contains('/') {
        if remote.ends_with(&format!("/{repo}")) {
            return Names::Path;
        }
        return Names::Other;
    }
    if remote.rsplit('/').next() == Some(repo.as_str()) {
        return Names::Base;
    }
    match only && asked.named {
        true => Names::Sole,
        false => Names::Other,
    }
}

/// The remotes and their URLs in a Git config file, in order.
pub fn remotes(config: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let mut current: Option<usize> = None;
    for line in config.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if let Some(header) = line.strip_prefix('[') {
            let header = header.trim_end_matches(']').trim();
            current = header
                .strip_prefix("remote")
                .map(|r| r.trim().trim_matches('"').to_owned())
                .filter(|name| !name.is_empty())
                .map(|name| match out.iter().position(|(n, _)| *n == name) {
                    Some(i) => i,
                    None => {
                        out.push((name, None));
                        out.len() - 1
                    }
                });
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        if let (Some(i), true) = (current, key.trim().eq_ignore_ascii_case("url"))
            && let Some(entry) = out.get_mut(i)
            && entry.1.is_none()
        {
            entry.1 = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    out
}

/// The branch a remote-tracking `HEAD` symref points at, from its loose
/// file (`git clone` and `git remote set-head` write one).
fn head_branch(git_dir: &Path, remote: &str) -> Option<String> {
    let file = common_dir(git_dir).join(format!("refs/remotes/{remote}/HEAD"));
    let text = std::fs::read_to_string(file).ok()?;
    let prefix = format!("ref: refs/remotes/{remote}/");
    text.trim().strip_prefix(&prefix).map(str::to_owned)
}

fn commit(git: &GitExecutable<'_>, git_dir: &Path, spec: &str) -> Result<Option<GitObjectId>> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    Ok(kb_commit(git, reads, spec, budgets()?, &CANCELLED)?)
}

/// A remote's default branch: its `HEAD`, else the one branch name the
/// other remotes' `HEAD`s agree on.
fn tip(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    remote: &str,
    agreed: Option<&str>,
) -> Result<Option<(String, GitObjectId)>> {
    if let Some(id) = commit(git, git_dir, &format!("refs/remotes/{remote}/HEAD"))? {
        let branch = head_branch(git_dir, remote).unwrap_or_else(|| "HEAD".into());
        return Ok(Some((format!("{remote}/{branch}"), id)));
    }
    let Some(branch) = agreed else {
        return Ok(None);
    };
    let found = commit(git, git_dir, &format!("refs/remotes/{remote}/{branch}"))?;
    Ok(found.map(|id| (format!("{remote}/{branch}"), id)))
}

/// Whether the clone has any remote-tracking ref of `remote`, loose or
/// packed.
fn tracked(git_dir: &Path, remote: &str) -> bool {
    let common = common_dir(git_dir);
    let loose = common.join(format!("refs/remotes/{remote}"));
    let packed = std::fs::read_to_string(common.join("packed-refs")).unwrap_or_default();
    loose.is_dir() || packed.contains(&format!(" refs/remotes/{remote}/"))
}

/// The remotes that were ever fetched into this clone. A bare or mirror
/// clone, or one whose remotes were never fetched, has none: its own
/// `HEAD` is the only state it holds, as before #179.
fn seen(git: &GitExecutable<'_>, git_dir: &Path, asked: &Asked<'_>) -> Result<Vec<Seen>> {
    let config = std::fs::read_to_string(common_dir(git_dir).join("config")).unwrap_or_default();
    let mut all = remotes(&config);
    if !all.iter().any(|(name, _)| tracked(git_dir, name)) {
        all.clear();
    }
    let mut heads: Vec<String> = all
        .iter()
        .filter_map(|(n, _)| head_branch(git_dir, n))
        .collect();
    heads.sort();
    heads.dedup();
    let agreed = heads.first().filter(|_| heads.len() == 1).cloned();
    let only = all.len() == 1;
    let mut out = Vec::new();
    for (name, url) in &all {
        out.push(Seen {
            name: name.clone(),
            url: url.clone(),
            names: names(url.as_deref(), asked, only),
            tip: tip(git, git_dir, name, agreed.as_deref())?,
        });
    }
    Ok(out)
}

/// The kernel's input: each remote with its tip as an index into the
/// distinct commits seen.
fn kernel(remotes: &[Seen]) -> Vec<Remote> {
    let mut commits: Vec<&GitObjectId> = Vec::new();
    let mut out = Vec::new();
    for r in remotes {
        let tip = match &r.tip {
            Some((_, id)) => {
                let k = match commits.iter().position(|c| *c == id) {
                    Some(k) => k,
                    None => {
                        commits.push(id);
                        commits.len() - 1
                    }
                };
                Tip::At(u32::try_from(k).unwrap_or(u32::MAX))
            }
            None => Tip::NoHead,
        };
        out.push(Remote {
            names: r.names,
            tip,
        });
    }
    out
}

/// What each remote names, for the reason line: `origin names me/service`.
fn described(remotes: &[Seen]) -> String {
    let each: Vec<String> = remotes
        .iter()
        .map(|r| {
            let says = match r.names {
                Names::Url => "is the cited url",
                Names::Path | Names::Base | Names::Sole => "names it",
                Names::Other => "names another repository",
                Names::Unread => "has no readable url",
            };
            format!("{} {says}", r.name)
        })
        .collect();
    each.join("; ")
}

/// The URL of the remote a map `ref=` names (`refs/remotes/R/…` or
/// `R/…`), if it names one.
fn pinned_url(git_dir: &Path, spec: &str) -> Option<String> {
    let config = std::fs::read_to_string(common_dir(git_dir).join("config")).ok()?;
    let rest = spec.strip_prefix("refs/remotes/").unwrap_or(spec);
    let remote = rest.split('/').next()?;
    remotes(&config)
        .into_iter()
        .find(|(name, _)| name == remote)
        .and_then(|(_, url)| url)
}

/// The target of a clone for one cited repository.
pub fn resolve(git: &GitExecutable<'_>, git_dir: &Path, asked: &Asked<'_>) -> Result<Resolved> {
    let pinned = match asked.pin {
        None => (Pinned::No, None),
        Some(spec) => match commit(git, git_dir, spec)? {
            Some(id) => (Pinned::Resolves, Some(id)),
            None => (Pinned::Missing, None),
        },
    };
    let remotes = match pinned.0 {
        Pinned::No => seen(git, git_dir, asked)?,
        _ => Vec::new(),
    };
    let none = |why: String| Resolved {
        commit: None,
        rule: "none".into(),
        why: Some(format!("target-ambiguous: {why}")),
        url: None,
    };
    Ok(match choose(pinned.0, &kernel(&remotes)) {
        Choice::Pinned => Resolved {
            commit: pinned.1,
            rule: format!("ref={}", asked.pin.unwrap_or_default()),
            why: Some("the local map's ref=".into()),
            url: pinned_url(git_dir, asked.pin.unwrap_or_default()),
        },
        Choice::Head => Resolved {
            commit: commit(git, git_dir, "HEAD")?,
            rule: "HEAD".into(),
            why: None,
            url: None,
        },
        Choice::Remote(i) => {
            let r = remotes.get(i);
            let (rule, id) = r.and_then(|r| r.tip.clone()).unzip();
            Resolved {
                commit: id,
                rule: rule.unwrap_or_else(|| "none".into()),
                why: (remotes.len() > 1).then(|| described(&remotes)),
                url: r.and_then(|r| r.url.clone()),
            }
        }
        Choice::Ambiguous(why) => none(match why {
            Why::PinMissing => format!(
                "the map's ref {} does not resolve",
                asked.pin.unwrap_or_default()
            ),
            Why::NoRemoteNames => format!(
                "no remote of this clone names {} ({})",
                asked.repo,
                described(&remotes)
            ),
            Why::NoDefaultBranch => "a remote that names the repository has no default branch here (git remote set-head REMOTE -a, or ref= in the map)".into(),
            Why::Disagree => format!(
                "remotes that name {} disagree ({})",
                asked.repo,
                described(&remotes)
            ),
        }),
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_forms_of_one_repository_normalise_equal() {
        let forms = [
            "https://github.com/org/service.git",
            "https://user@GitHub.com/org/service/",
            "ssh://git@github.com:22/org/service.git",
            "git@github.com:org/service.git",
            "git@github.com:org/service",
        ];
        for f in forms {
            assert_eq!(
                normalize(f).as_deref(),
                Some("github.com/org/service"),
                "{f}"
            );
        }
        assert_ne!(normalize("git@github.com:me/service"), normalize(forms[0]));
        assert_ne!(
            normalize("git@github.com:org/service-two"),
            normalize(forms[0])
        );
        assert_eq!(
            normalize("/srv/git/service.git").as_deref(),
            Some("/srv/git/service.git")
        );
    }

    #[test]
    fn a_remote_names_the_cited_repository_or_another() {
        let asked = |repo| Asked {
            repo,
            url: None,
            pin: None,
            named: true,
        };
        let fork = Some("git@github.com:me/service.git");
        let canon = Some("https://github.com/org/service");
        assert_eq!(names(fork, &asked("org/service"), false), Names::Other);
        assert_eq!(names(canon, &asked("org/service"), false), Names::Path);
        assert_eq!(names(canon, &asked("service"), false), Names::Base);
        assert_eq!(
            names(Some("file:///tmp/x/origin"), &asked("demo"), true),
            Names::Sole
        );
        assert_eq!(
            names(Some("file:///tmp/x/origin"), &asked("demo"), false),
            Names::Other
        );
        assert_eq!(names(None, &asked("demo"), true), Names::Unread);
        let by_url = Asked {
            url: Some("git@github.com:me/service.git"),
            ..asked("org/service")
        };
        assert_eq!(names(fork, &by_url, false), Names::Url);
    }

    #[test]
    fn remotes_are_read_in_order() {
        let config = "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@github.com:me/service.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n[remote \"upstream\"]\n\turl = https://github.com/org/service\n";
        let got = remotes(config);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "origin");
        assert_eq!(got[1].1.as_deref(), Some("https://github.com/org/service"));
    }
}
