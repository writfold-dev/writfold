//! Attached stores (v060-dev6-spec, part B): a declared companion
//! repository of evidence, found through a committed path hint checked
//! against its origin, whose citations are judged at their pinned
//! revision. Removing the declaration brings back the head rule, under
//! which the same reorganised store reads gone.
mod support;

use std::path::{Path, PathBuf};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;

/// The store's origin (`<base>/scratch`) with `work/probe/run.sh`, a clone
/// at `<base>/scratch-clone`, and a declaration whose hint names the clone.
fn stored() -> TestResult<(World, PathBuf, PathBuf)> {
    let w = world(&[])?;
    let origin = w.base.join("scratch");
    std::fs::create_dir_all(origin.join("work/probe"))?;
    git(&origin, &["init", "-q", "-b", "main"])?;
    git(&origin, &["config", "user.email", "t@example.invalid"])?;
    git(&origin, &["config", "user.name", "t"])?;
    std::fs::write(origin.join("work/probe/run.sh"), "echo probe\n")?;
    git(&origin, &["add", "-A"])?;
    git(&origin, &["commit", "-q", "-m", "probe"])?;
    let clone = w.base.join("scratch-clone");
    let url = format!("file://{}", origin.display());
    git(
        &w.base,
        &["clone", "-q", &url, &clone.display().to_string()],
    )?;
    declare(&w, &url, &clone)?;
    let rev = git(&clone, &["rev-parse", "HEAD"])?;
    let blob = git(&clone, &["rev-parse", "HEAD:work/probe/run.sh"])?;
    let note = format!(
        "# Probe\n\nThe probe ⟦cite kind=code repo=scratch rev={rev} path=work/probe/run.sh L=1-1 blob={blob}⟧.\n"
    );
    std::fs::write(w.kb.join("lantern/probe.md"), note)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "probe"])?;
    Ok((w, origin, clone))
}

fn declare(w: &World, url: &str, hint: &Path) -> TestResult {
    std::fs::create_dir_all(w.kb.join(".writfold/stores"))?;
    let text = format!(
        "schema = \"writfold-store/1\"\nname = \"scratch\"\nurl = \"{url}\"\nrole = \"attached\"\npaths = [\"{}\"]\n",
        hint.display()
    );
    std::fs::write(w.kb.join(".writfold/stores/scratch.toml"), text)?;
    Ok(())
}

/// Reorganise the store: the probe moves to `archive/` upstream, and the
/// clone pulls it.
fn reorganise(origin: &Path, clone: &Path) -> TestResult {
    git(origin, &["mv", "work", "archive"])?;
    git(origin, &["commit", "-q", "-m", "tidy"])?;
    git(clone, &["pull", "-q", "--ff-only"])?;
    Ok(())
}

fn stale(w: &World) -> TestResult<String> {
    let (text, _) = w.run(&w.request(Verb::Stale {
        scope: Some("lantern/probe.md".into()),
        offline: false,
        cached: false,
    }))?;
    Ok(text)
}

#[test]
fn a_reorganised_store_keeps_its_evidence_fresh() -> TestResult {
    let (w, origin, clone) = stored()?;
    reorganise(&origin, &clone)?;
    let text = stale(&w)?;
    assert!(
        text.contains(" checked=1 ") && !text.contains("probe.md"),
        "{text}"
    );
    // Without the declaration the store is any repository, found through
    // the map, and judged at head: the same citation is no longer fresh.
    std::fs::remove_file(w.kb.join(".writfold/stores/scratch.toml"))?;
    let map = w.kb.join(".git/writfold/repos");
    let mut lines = std::fs::read_to_string(&map)?;
    lines.push_str(&format!("scratch {}\n", clone.display()));
    std::fs::write(&map, lines)?;
    let text = stale(&w)?;
    assert!(
        text.contains("probe.md") && !text.contains(" checked=1 gone=0"),
        "{text}"
    );
    assert!(
        text.contains("[gone]") || text.contains("[moved]"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_pin_no_ref_reaches_is_gone_and_one_not_fetched_is_unverifiable() -> TestResult {
    let (w, origin, clone) = stored()?;
    // A commit only a deleted branch held: on disk, reached by no ref.
    git(&clone, &["checkout", "-q", "-b", "side"])?;
    std::fs::write(clone.join("work/probe/run.sh"), "echo side\n")?;
    git(&clone, &["commit", "-q", "-am", "side"])?;
    let side = git(&clone, &["rev-parse", "HEAD"])?;
    let side_blob = git(&clone, &["rev-parse", "HEAD:work/probe/run.sh"])?;
    git(&clone, &["checkout", "-q", "main"])?;
    git(&clone, &["branch", "-q", "-D", "side"])?;
    // A commit upstream that the clone has not fetched.
    std::fs::write(origin.join("work/probe/run.sh"), "echo later\n")?;
    git(&origin, &["commit", "-q", "-am", "later"])?;
    let later = git(&origin, &["rev-parse", "HEAD"])?;
    let later_blob = git(&origin, &["rev-parse", "HEAD:work/probe/run.sh"])?;
    let cite = |rev: &str, blob: &str| {
        format!("⟦cite kind=code repo=scratch rev={rev} path=work/probe/run.sh L=1-1 blob={blob}⟧")
    };
    let note = format!(
        "# Probe\n\nSide {}.\n\nLater {}.\n",
        cite(&side, &side_blob),
        cite(&later, &later_blob)
    );
    std::fs::write(w.kb.join("lantern/probe.md"), note)?;
    git(&w.kb, &["commit", "-q", "-am", "pins"])?;
    let text = stale(&w)?;
    assert!(
        text.contains("[gone]") && text.contains("reached by no ref"),
        "{text}"
    );
    assert!(
        text.contains("[unverifiable]") && text.contains("not in the local clone"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_hint_to_another_repository_is_a_mismatch() -> TestResult {
    let (w, _, _) = stored()?;
    // The hint names the code repository, whose origin is not the store's.
    let url = format!("file://{}", w.base.join("scratch").display());
    declare(&w, &url, &w.code)?;
    let text = stale(&w)?;
    assert!(text.contains("store-origin-mismatch"), "{text}");
    let (text, code) = w.run(&w.request(Verb::Repos { missing: true }))?;
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("repo scratch [mismatch]") && text.contains("!origin-differs"),
        "{text}"
    );
    Ok(())
}

#[test]
fn repos_lists_a_declared_store_and_how_it_was_found() -> TestResult {
    let (w, _, _) = stored()?;
    let (text, _) = w.run(&w.request(Verb::Repos { missing: false }))?;
    let at = text
        .find("repo scratch [local]")
        .ok_or_else(|| format!("no row:\n{text}"))?;
    let row = &text[at..];
    assert!(row.contains("!store") && row.contains("!pinned"), "{text}");
    assert!(
        row.contains("resolved: hint @<WORLD>/scratch-clone/.git"),
        "{text}"
    );
    Ok(())
}
