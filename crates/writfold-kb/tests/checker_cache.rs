//! Readers other than `kb stale` judge a checked kind from the verdict
//! cache, and only from a row that checker wrote: a verdict cached while
//! the kind was still time-based is not read as the checker's, and a token
//! no checker answered is `unverifiable: not-checked`.
mod support;

use support::{NOW, TestResult, git, world};
use writfold_kb::Verb;
use writfold_kb::session::Session;
use writfold_kb::stale::{self, Cite, Verdict};
use writfold_kb::workspace::{GitTool, discover};

const TOKEN: &str = "⟦cite kind=web url=https://example.invalid/a seen=2026-11-10T00:00Z⟧";
const TTL: &str =
    "schema = \"writfold-source/1\"\nkind = \"web\"\nfreshness = \"ttl\"\ndefault_ttl = \"30d\"\n";
const CHECKED: &str = "schema = \"writfold-source/1\"\nkind = \"web\"\nfreshness = \"checked\"\nchecker = \"absent-for-this-test\"\nfields = [\"url\"]\n";

fn put(w: &support::World, path: &str, text: &str) -> TestResult {
    let file = w.kb.join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, text)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", path])?;
    Ok(())
}

/// The verdict a non-spawning reader gives the token as filed in the KB.
fn read(w: &support::World) -> TestResult<Verdict> {
    let ws = discover(Some(&w.kb), &w.base)?;
    let tool = GitTool::locate()?;
    let session = Session::open(&ws, Some(&tool), false, NOW)?;
    let found = writfold_kb::parse::cite::tokens(TOKEN);
    let cites: Vec<Cite> = found
        .into_iter()
        .map(|c| Cite::from_citation("lantern/sources.md", 0, c))
        .collect();
    let mut checked = stale::check(&session, &cites, false)?;
    Ok(checked.pop().ok_or("no verdict")?.verdict)
}

#[test]
fn only_the_checkers_own_cached_row_is_read() -> TestResult {
    let w = world(&[])?;
    put(&w, "lantern/sources.md", &format!("# Sources\n\n{TOKEN}\n"))?;
    put(&w, ".writfold/sources/web.toml", TTL)?;
    let (text, _) = w.run(&w.request(Verb::Stale {
        scope: Some("lantern".into()),
        offline: false,
        cached: false,
    }))?;
    assert!(
        text.contains("checked=1 ") && !text.contains("cite web:"),
        "{text}"
    );
    assert_eq!(read(&w)?, Verdict::Fresh);
    // Declared checked: the cached TTL verdict for the same token text is
    // not the checker's, so it is not read.
    put(&w, ".writfold/sources/web.toml", CHECKED)?;
    assert_eq!(read(&w)?, Verdict::Unverifiable("not-checked".into()));
    // A row the checker wrote is read back with its detail.
    let ws = discover(Some(&w.kb), &w.base)?;
    let tool = GitTool::locate()?;
    let session = Session::open(&ws, Some(&tool), false, NOW)?;
    session.conn.execute(
        "UPDATE cite_checks SET repo_head = 'checker:absent-for-this-test@1.0:r9', \
         verdict = 'changed', detail = 'said changed' WHERE path = 'lantern/sources.md'",
        [],
    )?;
    drop(session);
    let detail = Some("said changed".to_owned());
    assert_eq!(read(&w)?, Verdict::Changed { detail });
    Ok(())
}

const DAY: i64 = 86_400;

/// The checker's own fresh row for the token, checked `age` seconds ago.
fn cache_fresh(w: &support::World, age: i64) -> TestResult {
    let ws = discover(Some(&w.kb), &w.base)?;
    let tool = GitTool::locate()?;
    let session = Session::open(&ws, Some(&tool), false, NOW)?;
    session.conn.execute(
        "UPDATE cite_checks SET repo_head = 'checker:absent-for-this-test@1.0:r9', \
         verdict = 'fresh', detail = NULL, checked_at = ?1 WHERE path = 'lantern/sources.md'",
        [writfold_kb::time::stamp(NOW - age)],
    )?;
    Ok(())
}

fn stale_text(w: &support::World, offline: bool) -> TestResult<String> {
    let (text, _) = w.run(&w.request(Verb::Stale {
        scope: Some("lantern".into()),
        offline,
        cached: false,
    }))?;
    Ok(text)
}

#[test]
fn a_cached_verdict_older_than_max_age_is_check_expired() -> TestResult {
    let w = world(&[])?;
    put(&w, "lantern/sources.md", &format!("# Sources\n\n{TOKEN}\n"))?;
    put(&w, ".writfold/sources/web.toml", TTL)?;
    stale_text(&w, false)?;
    // Without max_age a cached fresh verdict is read at any age.
    put(&w, ".writfold/sources/web.toml", CHECKED)?;
    cache_fresh(&w, 8 * DAY)?;
    assert_eq!(read(&w)?, Verdict::Fresh);
    // With max_age = "7d": 6 days old still reads fresh; 8 days old is
    // unverifiable for every cache reader (gate grounding reads this way).
    put(
        &w,
        ".writfold/sources/web.toml",
        &format!("{CHECKED}max_age = \"7d\"\n"),
    )?;
    cache_fresh(&w, 6 * DAY)?;
    assert_eq!(read(&w)?, Verdict::Fresh);
    let offline = stale_text(&w, true)?;
    assert!(!offline.contains("cite web:"), "{offline}");
    cache_fresh(&w, 8 * DAY)?;
    let expired = Verdict::Unverifiable("check-expired (8d)".into());
    assert_eq!(read(&w)?, expired);
    // Exactly at max_age is still within it.
    cache_fresh(&w, 7 * DAY)?;
    assert_eq!(read(&w)?, Verdict::Fresh);
    // `kb stale --offline` reports the expired row from the cache.
    cache_fresh(&w, 8 * DAY)?;
    let offline = stale_text(&w, true)?;
    let row = offline
        .lines()
        .find(|l| l.starts_with("cite web:"))
        .ok_or(offline.clone())?;
    assert!(
        row.contains("[unverifiable]") && row.contains("check-expired (8d)"),
        "{row}"
    );
    // Online `kb stale` asks the checker again (absent here) instead.
    let online = stale_text(&w, false)?;
    assert!(
        online.contains("checker-absent (web)") && !online.contains("check-expired"),
        "{online}"
    );
    Ok(())
}
