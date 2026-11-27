//! #179 (RFC 2026-11-27-citation-target): a fork clone's citations are
//! judged at the remote that names the cited repository, never at `origin`
//! by default. One work clone per layout, the same two citations in each:
//! `old.txt` (in the fork and upstream) and `new.txt` (added upstream after
//! the fork last synced), cited at upstream's tip as `repo=org/service`.
//! The fixture KB has other flagged rows, so exit codes are not compared:
//! each test reads the fork record's rows.
mod support;

use std::path::{Path, PathBuf};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;

/// The repositories: `org/service` (two commits), its fork `me/service` at
/// the first, a `mirror/org/service` at the first, and the cited commit.
struct Layout {
    w: World,
    canon: PathBuf,
    fork: PathBuf,
    mirror: PathBuf,
    cited: String,
}

fn bare(from: &Path, to: &Path) -> TestResult {
    let parent = to.parent().ok_or("no parent")?;
    std::fs::create_dir_all(parent)?;
    let url = format!("file://{}", from.display());
    git(
        parent,
        &["clone", "-q", "--bare", &url, &to.display().to_string()],
    )?;
    Ok(())
}

fn layout() -> TestResult<Layout> {
    let w = world(&[])?;
    let canon = w.base.join("hosts/org/service");
    std::fs::create_dir_all(&canon)?;
    git(&canon, &["init", "-q", "-b", "main"])?;
    std::fs::write(canon.join("old.txt"), "alpha\nbeta\n")?;
    git(&canon, &["add", "-A"])?;
    git(&canon, &["commit", "-q", "-m", "old"])?;
    let fork = w.base.join("hosts/me/service");
    let mirror = w.base.join("hosts/mirror/org/service");
    bare(&canon, &fork)?;
    bare(&canon, &mirror)?;
    std::fs::write(canon.join("new.txt"), "gamma\ndelta\n")?;
    git(&canon, &["add", "-A"])?;
    git(&canon, &["commit", "-q", "-m", "new"])?;
    let cited = git(&canon, &["rev-parse", "HEAD"])?;
    let mut note = String::from("# Fork\n\n");
    for path in ["old.txt", "new.txt"] {
        let blob = git(&canon, &["rev-parse", &format!("HEAD:{path}")])?;
        note.push_str(&format!(
            "⟦cite kind=code repo=org/service rev={cited} path={path} L=1-2 blob={blob}⟧\n"
        ));
    }
    std::fs::write(w.kb.join("lantern/fork.md"), note)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "fork"])?;
    Ok(Layout {
        w,
        canon,
        fork,
        mirror,
        cited,
    })
}

/// A work clone of the fork with `remotes` added and fetched, mapped as
/// `org/service` (with `extra` words on the map line).
fn work(l: &Layout, name: &str, remotes: &[(&str, &Path)], extra: &str) -> TestResult<PathBuf> {
    let dir = l.w.base.join(format!("clones/{name}"));
    std::fs::create_dir_all(l.w.base.join("clones"))?;
    let url = format!("file://{}", l.fork.display());
    git(&l.w.base.join("clones"), &["clone", "-q", &url, name])?;
    for (remote, path) in remotes {
        git(
            &dir,
            &[
                "remote",
                "add",
                remote,
                &format!("file://{}", path.display()),
            ],
        )?;
        git(&dir, &["fetch", "-q", remote])?;
    }
    let map = l.w.kb.join(".git/writfold/repos");
    let line = format!("org/service {}{extra}\n", dir.display());
    let demo = format!("demo-service {}\n", l.w.code.display());
    std::fs::write(map, format!("# local repository map\n{demo}{line}"))?;
    Ok(dir)
}

fn stale(w: &World) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::Stale {
        scope: Some("lantern/fork.md".into()),
        offline: false,
        cached: false,
    }))
}

fn repos(w: &World) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::Repos { missing: false }))
}

/// The `kb stale` row of `path`, or `[fresh]` (no row is printed for a
/// fresh citation).
fn row(text: &str, path: &str) -> String {
    let at = format!("code:org/service:{path}:");
    text.lines()
        .find(|l| l.contains(&at))
        .map_or_else(|| "[fresh]".into(), str::to_owned)
}

#[test]
fn a_stale_fork_with_upstream_is_judged_at_upstream() -> TestResult {
    let l = layout()?;
    work(&l, "both", &[("upstream", &l.canon)], "")?;
    let (text, _) = stale(&l.w)?;
    assert_eq!(row(&text, "new.txt"), "[fresh]", "{text}");
    assert_eq!(row(&text, "old.txt"), "[fresh]", "{text}");
    let (text, _) = repos(&l.w)?;
    assert!(text.contains("target: upstream/main "), "{text}");
    assert!(text.contains("origin names another repository"), "{text}");
    Ok(())
}

#[test]
fn a_fork_only_clone_is_ambiguous_never_gone() -> TestResult {
    let l = layout()?;
    work(&l, "fork-only", &[], "")?;
    let (text, code) = stale(&l.w)?;
    assert_eq!(code, 2, "{text}");
    let new = row(&text, "new.txt");
    assert!(
        new.contains("[unverifiable]")
            && new.contains("target-ambiguous: no remote of this clone names org/service"),
        "{new}"
    );
    assert!(!text.contains("[gone]"), "{text}");
    let (text, code) = repos(&l.w)?;
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("repo org/service [ambiguous]"), "{text}");
    assert!(text.contains("ref=REF"), "{text}");
    Ok(())
}

#[test]
fn candidates_that_disagree_are_ambiguous_and_agreeing_ones_are_not() -> TestResult {
    let l = layout()?;
    work(
        &l,
        "three",
        &[("upstream", &l.canon), ("mirror", &l.mirror)],
        "",
    )?;
    let (text, _) = stale(&l.w)?;
    assert!(
        row(&text, "new.txt").contains("remotes that name org/service disagree"),
        "{text}"
    );
    // The mirror catches up: both candidates at one commit.
    git(
        &l.mirror,
        &[
            "fetch",
            "-q",
            &format!("file://{}", l.canon.display()),
            "main:main",
        ],
    )?;
    git(&l.w.base.join("clones/three"), &["fetch", "-q", "mirror"])?;
    let (text, _) = stale(&l.w)?;
    assert_eq!(row(&text, "new.txt"), "[fresh]", "{text}");
    Ok(())
}

#[test]
fn the_maps_ref_decides_and_a_missing_one_is_ambiguous() -> TestResult {
    let l = layout()?;
    work(
        &l,
        "pinned",
        &[("upstream", &l.canon)],
        " ref=refs/remotes/upstream/main",
    )?;
    let (text, _) = stale(&l.w)?;
    assert_eq!(row(&text, "new.txt"), "[fresh]", "{text}");
    work(
        &l,
        "pin-missing",
        &[("upstream", &l.canon)],
        " ref=refs/remotes/nowhere/main",
    )?;
    let (text, _) = stale(&l.w)?;
    assert!(row(&text, "new.txt").contains("does not resolve"), "{text}");
    Ok(())
}

#[test]
fn a_target_behind_the_cited_commit_never_reads_gone() -> TestResult {
    let l = layout()?;
    // The map pins the fork: it is behind the cited commit.
    work(
        &l,
        "behind",
        &[("upstream", &l.canon)],
        " ref=refs/remotes/origin/main",
    )?;
    let (text, code) = stale(&l.w)?;
    assert_eq!(code, 2, "{text}");
    let new = row(&text, "new.txt");
    assert!(
        new.contains("[unverifiable]") && new.contains("target-behind"),
        "{new}"
    );
    // What the bytes at the target still show stays: old.txt is fresh.
    assert_eq!(row(&text, "old.txt"), "[fresh]", "{text}");
    assert!(!l.cited.is_empty());
    Ok(())
}

fn cite_make(w: &World) -> TestResult<(String, u8)> {
    let verb = writfold_kb::write::Write::CiteMake(writfold_kb::write::cite::CiteMake {
        repo: "org/service".into(),
        path: "new.txt".into(),
        first: 1,
        last: 2,
        // The work tree is the fork's checkout, without `new.txt`: cite the
        // committed bytes at the chosen target.
        allow_dirty: true,
        ..Default::default()
    });
    match w.run(&w.request(Verb::Write(verb))) {
        Ok(found) => Ok(found),
        Err(e) => Ok((e.to_string(), 3)),
    }
}

/// #180: `cite make` cites the target `stale` will judge at, and writes
/// that remote's URL; on an ambiguous clone it refuses with the reason.
#[test]
fn cite_make_cites_the_chosen_target() -> TestResult {
    let l = layout()?;
    work(&l, "both", &[("upstream", &l.canon)], "")?;
    let (text, code) = cite_make(&l.w)?;
    assert_eq!(code, 0, "{text}");
    assert!(text.contains(&format!("rev={}", l.cited)), "{text}");
    assert!(
        text.contains("url=file://<WORLD>/hosts/org/service "),
        "{text}"
    );
    work(&l, "fork-only", &[], "")?;
    let (text, code) = cite_make(&l.w)?;
    assert_eq!(code, 3, "{text}");
    assert!(text.contains("target-ambiguous"), "{text}");
    Ok(())
}

/// Write `.writfold/repos/service.toml` naming `url` (and `extra` lines).
fn declare(l: &Layout, url: &Path, extra: &str) -> TestResult {
    let dir = l.w.kb.join(".writfold/repos");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("service.toml"),
        format!(
            "schema = \"writfold-repo/1\"\nname = \"org/service\"\nurl = \"file://{}\"\n{extra}",
            url.display()
        ),
    )?;
    Ok(())
}

/// #197: a declaration decides which remote is the repository, whatever a
/// token's url= says; a clone with no remote at the declared URL is
/// ambiguous; a declared ref is that branch; the map's ref= still wins.
#[test]
fn a_declared_repository_is_its_declared_url() -> TestResult {
    let l = layout()?;
    // Tokens minted in a fork clone name the fork's URL: without a
    // declaration that fork (url= equal) would be the target.
    let fork_url = format!("file://{}", l.fork.display());
    let note = std::fs::read_to_string(l.w.kb.join("lantern/fork.md"))?
        .replace("⟧", &format!(" url={fork_url}⟧"));
    std::fs::write(l.w.kb.join("lantern/fork.md"), note)?;
    git(&l.w.kb, &["commit", "-qam", "fork url"])?;
    work(&l, "both", &[("upstream", &l.canon)], "")?;
    let (text, _) = stale(&l.w)?;
    assert!(
        row(&text, "new.txt").contains("target-behind"),
        "the fork wins undeclared: {text}"
    );
    declare(&l, &l.canon, "")?;
    let (text, _) = stale(&l.w)?;
    assert_eq!(row(&text, "new.txt"), "[fresh]", "{text}");
    let (text, _) = repos(&l.w)?;
    assert!(
        text.contains("declared: .writfold/repos/service.toml; upstream is its url"),
        "{text}"
    );

    work(&l, "fork-only", &[], "")?;
    let (text, _) = stale(&l.w)?;
    assert!(
        row(&text, "new.txt").contains("no remote of this clone is at the declared url"),
        "{text}"
    );

    declare(&l, &l.canon, "ref = \"release\"\n")?;
    work(&l, "no-release", &[("upstream", &l.canon)], "")?;
    let (text, _) = stale(&l.w)?;
    assert!(
        row(&text, "new.txt").contains("the declared ref release"),
        "{text}"
    );
    work(
        &l,
        "pinned",
        &[("upstream", &l.canon)],
        " ref=refs/remotes/upstream/main",
    )?;
    let (text, _) = stale(&l.w)?;
    assert_eq!(
        row(&text, "new.txt"),
        "[fresh]",
        "the map's ref= wins: {text}"
    );
    Ok(())
}

/// #198: no product module asks a clone for `origin` by name; the
/// resolver (`stale/target.rs`) and the store module's mismatch note are
/// the only places a remote's name is read.
#[test]
fn no_module_reads_origin_by_convention() -> TestResult {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !path.ends_with("stale/target.rs")
            {
                let text = std::fs::read_to_string(&path)?;
                for needle in ["remote.origin.url", "refs/remotes/origin"] {
                    if text.contains(needle) {
                        found.push(format!("{}: {needle}", path.display()));
                    }
                }
            }
        }
    }
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

/// #198: a store cloned as a fork, with `upstream` at the declared URL, is
/// the store: no `!origin-differs`.
#[test]
fn a_store_cloned_as_a_fork_is_found_by_its_declared_url() -> TestResult {
    let l = layout()?;
    let dir = work(&l, "store-fork", &[("upstream", &l.canon)], "")?;
    std::fs::write(l.w.kb.join(".git/writfold/repos"), "# map\n")?;
    let stores = l.w.kb.join(".writfold/stores");
    std::fs::create_dir_all(&stores)?;
    std::fs::write(
        stores.join("scratch.toml"),
        format!(
            "schema = \"writfold-store/1\"\nname = \"scratch\"\nurl = \"file://{}\"\nrole = \"attached\"\npaths = [\"{}\"]\n",
            l.canon.display(),
            dir.display()
        ),
    )?;
    let (text, _) = repos(&l.w)?;
    let row = text
        .lines()
        .find(|r| r.starts_with("repo scratch"))
        .ok_or("no store row")?;
    assert!(!row.contains("origin-differs"), "{text}");
    assert!(row.contains("[local]"), "{text}");
    Ok(())
}
