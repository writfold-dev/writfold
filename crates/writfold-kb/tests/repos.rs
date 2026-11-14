//! `kb repos` and partial clones (design section 6): one row per cited
//! repository (`local`, `partial` or `missing`) with how its clone was
//! found; a partial clone's code citations read as unverifiable
//! (`partial-clone`) instead of failing `stale`, and any other Git failure
//! in one repository degrades only that repository's citations.
mod support;

use support::{TestResult, World, git, world};
use writfold_kb::Verb;

/// A token citing `src/ledger.rs:1-2` of the code repository's HEAD as
/// `repo=<name>`, with `extra` fields.
fn token(w: &World, name: &str, extra: &str) -> TestResult<String> {
    let rev = git(&w.code, &["rev-parse", "HEAD"])?;
    let blob = git(&w.code, &["rev-parse", "HEAD:src/ledger.rs"])?;
    Ok(format!(
        "⟦cite kind=code repo={name} rev={rev} path=src/ledger.rs L=1-2 blob={blob}{extra}⟧"
    ))
}

/// A world whose KB also cites `lazy` (a `--filter=blob:none` bare clone of
/// the code repository, mapped) and `ghost` (no clone anywhere).
fn cited() -> TestResult<World> {
    let w = world(&[])?;
    git(&w.code, &["config", "uploadpack.allowFilter", "true"])?;
    let lazy = w.base.join("lazy.git");
    let url = format!("file://{}", w.code.display());
    let to = lazy.display().to_string();
    git(
        &w.base,
        &["clone", "-q", "--bare", "--filter=blob:none", &url, &to],
    )?;
    let map = w.kb.join(".git/writfold/repos");
    let mut text = std::fs::read_to_string(&map)?;
    text.push_str(&format!("lazy {to}\n"));
    std::fs::write(&map, text)?;
    let note = format!(
        "# Repositories\n\n{}\n{}\n{}\n",
        token(&w, "lazy", "")?,
        token(&w, "ghost", " url=https://example.invalid/org/ghost.git")?,
        token(&w, "ghost", "")?,
    );
    std::fs::write(w.kb.join("lantern/repos.md"), note)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "repos"])?;
    Ok(w)
}

fn row<'a>(text: &'a str, id: &str) -> TestResult<&'a str> {
    let at = format!("repo {id} ");
    let start = text
        .find(&at)
        .ok_or_else(|| format!("no row for {id} in\n{text}"))?;
    let rest = &text[start..];
    let end = rest[1..].find("\nrepo ").map_or(rest.len(), |n| n + 1);
    Ok(&rest[..end])
}

#[test]
fn every_cited_repository_has_a_row() -> TestResult {
    let w = cited()?;
    let (text, code) = w.run(&w.request(Verb::Repos { missing: false }))?;
    assert_eq!(code, 2, "{text}");
    let demo = row(&text, "demo-service")?;
    assert!(
        demo.starts_with("repo demo-service [local] tokens="),
        "{demo}"
    );
    assert!(
        demo.contains("\n  resolved: map @<WORLD>/demo-service/.git\n"),
        "{demo}"
    );
    assert!(demo.contains("\n  target: HEAD "), "{demo}");
    assert!(demo.contains("\n  fetched: none"), "{demo}");
    let lazy = row(&text, "lazy")?;
    assert!(lazy.starts_with("repo lazy [partial] tokens=1"), "{lazy}");
    assert!(lazy.contains("resolved: map @<WORLD>/lazy.git"), "{lazy}");
    let ghost = row(&text, "ghost")?;
    assert!(
        ghost.starts_with("repo ghost [missing] tokens=2 @lantern/repos.md:4"),
        "{ghost}"
    );
    assert!(
        ghost.contains("\n  url: https://example.invalid/org/ghost.git"),
        "{ghost}"
    );
    assert!(!ghost.contains("resolved:"), "{ghost}");
    assert!(text.contains(" repos=3 missing=1 partial=1 "), "{text}");
    Ok(())
}

#[test]
fn missing_filters_and_json_has_a_stable_shape() -> TestResult {
    let w = cited()?;
    let mut request = w.request(Verb::Repos { missing: true });
    request.json = true;
    let (text, code) = w.run(&request)?;
    assert_eq!(code, 2);
    let body: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(body["verb"], "repos");
    let rows = body["rows"].as_array().ok_or("rows")?;
    assert_eq!(rows.len(), 1, "{text}");
    assert_eq!(rows[0]["kind"], "repo");
    assert_eq!(rows[0]["id"], "ghost");
    assert_eq!(rows[0]["status"], "missing");
    assert_eq!(rows[0]["text"], "tokens=2");
    assert_eq!(rows[0]["more"][0]["label"], "url");
    assert_eq!(
        rows[0]["more"][0]["text"],
        "https://example.invalid/org/ghost.git"
    );
    assert_eq!(body["trailer"]["counts"]["missing"], 1);
    assert_eq!(body["trailer"]["counts"]["repos"], 3);
    Ok(())
}

#[test]
fn no_missing_repository_exits_zero() -> TestResult {
    let w = world(&[])?;
    let (text, code) = w.run(&w.request(Verb::Repos { missing: true }))?;
    assert_eq!(code, 0, "{text}");
    assert!(!text.contains("repo "), "{text}");
    assert!(text.contains(" repos=1 missing=0 partial=0 "), "{text}");
    Ok(())
}

#[test]
fn fetch_time_is_reported() -> TestResult {
    let w = world(&[])?;
    std::fs::write(w.code.join(".git/FETCH_HEAD"), "")?;
    let (text, _) = w.run(&w.request(Verb::Repos { missing: false }))?;
    let demo = row(&text, "demo-service")?;
    assert!(!demo.contains("fetched: none"), "{demo}");
    assert!(demo.contains("\n  fetched: 20"), "{demo}");
    Ok(())
}

fn stale(w: &World) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::Stale {
        scope: None,
        offline: false,
        cached: false,
    }))
}

#[test]
fn a_partial_clone_is_unverifiable_not_an_error() -> TestResult {
    let w = cited()?;
    let (text, code) = stale(&w)?;
    assert_eq!(code, 2, "{text}");
    let lazy = text
        .lines()
        .find(|l| l.contains("@lantern/repos.md:3"))
        .ok_or("lazy row")?;
    assert!(lazy.contains("[unverifiable]"), "{lazy}");
    assert!(lazy.contains("partial-clone"), "{lazy}");
    let ghost = text
        .lines()
        .find(|l| l.contains("@lantern/repos.md:4"))
        .ok_or("ghost row")?;
    assert!(ghost.contains("repo-not-local"), "{ghost}");
    // The other repository's citations are still checked.
    assert!(text.contains("[gone]"), "{text}");
    Ok(())
}

#[test]
fn a_git_failure_degrades_only_its_repository() -> TestResult {
    let w = cited()?;
    // The promisor marking moved to an included file, which the config
    // reading does not follow: Git refuses the lazy fetch and fails, and
    // only this repository's citations say so.
    let config = w.base.join("lazy.git/config");
    let text = std::fs::read_to_string(&config)?;
    let plain: String = text
        .lines()
        .filter(|l| !l.contains("promisor") && !l.contains("partialclone"))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&config, format!("{plain}[include]\n\tpath = marked\n"))?;
    std::fs::write(
        w.base.join("lazy.git/marked"),
        "[remote \"origin\"]\n\tpromisor = true\n",
    )?;
    assert!(!writfold_kb::stale::repos::partial(
        &w.base.join("lazy.git")
    ));
    let (text, code) = stale(&w)?;
    assert_eq!(code, 2, "{text}");
    let lazy = text
        .lines()
        .find(|l| l.contains("@lantern/repos.md:3"))
        .ok_or("lazy row")?;
    assert!(lazy.contains("[unverifiable]"), "{lazy}");
    assert!(lazy.contains("git-error: "), "{lazy}");
    assert!(text.contains("[gone]"), "{text}");
    Ok(())
}
