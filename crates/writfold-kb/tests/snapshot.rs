//! `observe … --snapshot FILE` (stores-next spec, section 2): the look's
//! bytes are kept in a pinned store, committed by the user between two runs
//! of one command, and cited from the observation; a snapshot that is lost
//! stops the look grounding.
mod checklist;
mod support;

use checklist::{REVIEW, RUN, UNIT, cover, item, put, status};
use std::path::{Path, PathBuf};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::Outcome;
use writfold_kb::write::Write;
use writfold_kb::write::observe::Observe;

const PAGE: &str =
    "schema = \"writfold-source/1\"\nkind = \"page\"\nfreshness = \"ttl\"\ndefault_ttl = \"30d\"\n";

/// A KB with the review checklist and a store `scratch` (origin and clone),
/// declared with `extra` lines.
fn stored(extra: &str) -> TestResult<(World, PathBuf)> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    put(&w.kb.join(".writfold/gates/review.toml"), REVIEW)?;
    put(&w.kb.join(".writfold/sources/page.toml"), PAGE)?;
    let origin = w.base.join("scratch");
    std::fs::create_dir_all(&origin)?;
    git(&origin, &["init", "-q", "-b", "main"])?;
    git(&origin, &["config", "user.email", "t@example.invalid"])?;
    git(&origin, &["config", "user.name", "t"])?;
    std::fs::write(origin.join("README.md"), "# Scratch\n")?;
    git(&origin, &["add", "-A"])?;
    git(&origin, &["commit", "-q", "-m", "start"])?;
    git(
        &origin,
        &["config", "receive.denyCurrentBranch", "updateInstead"],
    )?;
    let clone = w.base.join("scratch-clone");
    let url = format!("file://{}", origin.display());
    git(
        &w.base,
        &["clone", "-q", &url, &clone.display().to_string()],
    )?;
    git(&clone, &["config", "user.email", "t@example.invalid"])?;
    git(&clone, &["config", "user.name", "t"])?;
    put(
        &w.kb.join(".writfold/stores/scratch.toml"),
        &format!(
            "schema = \"writfold-store/1\"\nname = \"scratch\"\nurl = \"{url}\"\nrole = \"attached\"\n{extra}paths = [\"{}\"]\n",
            clone.display()
        ),
    )?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "store"])?;
    Ok((w, std::fs::canonicalize(clone)?))
}

fn observe(w: &World, status: &str, file: &Path) -> writfold_kb::error::Result<String> {
    let o = Observe {
        source: "page".into(),
        locator: "https://example.org/spec/v2".into(),
        status: status.into(),
        item: Some("review.perf".into()),
        run: Some(RUN.into()),
        snapshot: Some(file.display().to_string()),
        ..Observe::default()
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::Observe(o))))?;
    Ok(out.text)
}

fn refusal(result: writfold_kb::error::Result<String>) -> String {
    match result {
        Ok(text) => format!("not refused: {text}"),
        Err(e) => e.to_string(),
    }
}

fn observed(w: &World) -> usize {
    let dir = w.kb.join(format!("runs/{RUN}/observed"));
    std::fs::read_dir(dir).map_or(0, |d| d.count())
}

/// The `stored:` path the first call staged.
fn staged(why: &str) -> TestResult<String> {
    let line = why
        .lines()
        .find_map(|l| l.trim().strip_prefix("stored: scratch:"));
    let path = line
        .and_then(|l| l.split(" (").next())
        .ok_or("no stored line")?;
    Ok(path.to_owned())
}

#[test]
fn a_snapshot_is_staged_then_recorded_once_committed() -> TestResult {
    let (w, clone) = stored("")?;
    let page = w.base.join("spec-v2.html");
    std::fs::write(&page, "<h1>v2</h1>\n")?;
    let why = refusal(observe(&w, "ok", &page));
    assert!(
        why.contains("the snapshot is not committed; nothing was recorded"),
        "{why}"
    );
    assert!(
        why.contains("next: git -C") && why.contains(" push"),
        "{why}"
    );
    assert!(
        why.contains("--snapshot ") && why.contains("--for review.perf"),
        "{why}"
    );
    assert_eq!(observed(&w), 0, "nothing is recorded before the commit");
    let rel = staged(&why)?;
    assert!(
        rel.starts_with("snapshots/2026-") && rel.ends_with("/spec-v2.html"),
        "{rel}"
    );
    let provenance = std::fs::read_to_string(clone.join(format!("{rel}.snapshot.json")))?;
    assert!(
        provenance.contains("\"schema\": \"writfold-snapshot/1\""),
        "{provenance}"
    );
    // Running it again before the commit stages nothing new and still refuses.
    let again = refusal(observe(&w, "ok", &clone.join(&rel)));
    assert!(again.contains("not committed"), "{again}");
    git(&clone, &["add", "-A"])?;
    git(&clone, &["commit", "-q", "-m", "snapshot"])?;
    let text = observe(&w, "ok", &clone.join(&rel))?;
    assert!(text.contains("recorded runs/"), "{text}");
    let path = text
        .lines()
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|r| r.split(' ').next())
        .ok_or("no path")?;
    let saved = std::fs::read_to_string(w.kb.join(path))?;
    assert!(
        saved.contains("\"snapshot\": \"⟦cite kind=code repo=scratch rev="),
        "{saved}"
    );
    assert!(saved.contains(&format!("path={rel} ")), "{saved}");
    // It grounds a cover row while its snapshot is held, flagged on looked:.
    cover(&w, "perf", Outcome::Supported, &[path])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "perf"), "supported", "{text}");
    assert!(text.contains("!snapshot"), "{text}");
    // No ref reaches the snapshot any more: the look stops grounding.
    let head = git(&clone, &["rev-parse", "HEAD"])?;
    git(&clone, &["checkout", "-q", "--detach", &head])?;
    git(&clone, &["branch", "-q", "-D", "main"])?;
    git(&clone, &["update-ref", "-d", "refs/remotes/origin/main"])?;
    git(&clone, &["update-ref", "-d", "refs/remotes/origin/HEAD"])?;
    let (text, _) = status(&w)?;
    assert_ne!(item(&text, "perf"), "supported", "{text}");
    Ok(())
}

#[test]
fn snapshots_are_refused_where_the_kernel_withholds_them() -> TestResult {
    let (w, clone) = stored("")?;
    let page = w.base.join("page.html");
    std::fs::write(&page, "<p>x</p>\n")?;
    // A look that found nothing embeds no citation, so it keeps no snapshot.
    let why = refusal(observe(&w, "no-data", &page));
    assert!(
        why.contains("--snapshot is for ok and partial looks"),
        "{why}"
    );
    assert!(!clone.join("snapshots").exists(), "nothing is written");
    // Bytes that no longer match their provenance file.
    let why = refusal(observe(&w, "ok", &page));
    let rel = staged(&why)?;
    std::fs::write(clone.join(&rel), "<p>edited</p>\n")?;
    let why = refusal(observe(&w, "ok", &clone.join(&rel)));
    assert!(
        why.contains("no longer matches its provenance file"),
        "{why}"
    );
    // Committed bytes that differ from the provenance (edited, then committed).
    git(&clone, &["add", "-A"])?;
    git(&clone, &["commit", "-q", "-m", "edited"])?;
    let why = refusal(observe(&w, "ok", &clone.join(&rel)));
    assert!(why.contains("no longer matches"), "{why}");
    assert_eq!(observed(&w), 0);
    // A store judged at head keeps no snapshot.
    let (w, clone) = stored("freshness = \"head\"\n")?;
    let why = refusal(observe(&w, "ok", &page));
    assert!(
        why.contains("a snapshot is kept only in a pinned store"),
        "{why}"
    );
    assert!(!clone.join("snapshots").exists());
    Ok(())
}

#[test]
fn a_kb_without_stores_refuses_and_a_plain_look_is_unchanged() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/review.toml"), REVIEW)?;
    let page = w.base.join("page.html");
    std::fs::write(&page, "x\n")?;
    let why = refusal(observe(&w, "ok", &page));
    assert!(why.contains("no store is declared"), "{why}");
    assert_eq!(observed(&w), 0);
    Ok(())
}
