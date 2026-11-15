//! How a clone reads check receipts it did not write: bound to the token's
//! exact text and the declared checker, aged against the kind's `max_age`,
//! newest statement first, strict about the file, never cached locally and
//! never starting a checker.
mod checker;

use checker::receipts::{self, commit, files, max_age, reader, stamp};
use checker::{RUN, Result, Root, cached_rows, kb, row, world};

const FRESH: &str = "⟦cite kind=src depot=main path=lib/fresh.c L=3-9 want=fresh⟧";
const DAY: i64 = 86_400;

/// A KB declaring `src` with `max_age = 7d`, one token, no checker, and an
/// index built.
fn bare(name: &str, token: &str) -> Result<Root> {
    let w = world(name, &[token], false)?;
    max_age(&w, Some("7d"))?;
    kb(&w, &["index", "status"], &[])?;
    Ok(w)
}

/// The `stale --cached` row for `lib/fresh.c` ("" when fresh).
fn cached(w: &Root) -> Result<String> {
    let (text, _) = kb(w, &["stale", "--cached"], &[])?;
    Ok(row(&text, "lib/fresh.c").to_owned())
}

#[test]
fn a_receipt_is_bound_to_the_token_text_and_the_checker() -> Result {
    let w = bare("rcp-bound", FRESH)?;
    receipts::write(&w, "a.json", -60, &[(FRESH, "gone")], "acme-src")?;
    assert!(cached(&w)?.contains("[gone]") && cached(&w)?.contains("!recorded"));
    // One character of the token changed: no receipt holds it.
    let other = FRESH.replace("L=3-9", "L=3-8");
    std::fs::write(
        w.kb().join("notes/sources.md"),
        format!("# Sources\n\n{other}\n"),
    )?;
    let (text, _) = kb(&w, &["stale", "--cached"], &[])?;
    assert!(text.contains("not-checked"), "{text}");
    let (text, _) = kb(&w, &["stale"], &[])?;
    assert!(
        text.contains("checker-absent (src)") && !text.contains("recorded"),
        "{text}"
    );
    // Another checker's receipt is ignored.
    let w = bare("rcp-other", FRESH)?;
    receipts::write(&w, "a.json", -60, &[(FRESH, "gone")], "other-src")?;
    assert!(cached(&w)?.contains("not-checked"), "{}", cached(&w)?);
    Ok(())
}

#[test]
fn a_receipt_ages_against_max_age_and_needs_one() -> Result {
    let w = bare("rcp-age", FRESH)?;
    receipts::write(&w, "a.json", 60 - 7 * DAY, &[(FRESH, "fresh")], "acme-src")?;
    assert_eq!(cached(&w)?, "", "just within max_age");
    let before = cached_rows(&w)?;
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(
        cached_rows(&w)?,
        before,
        "a recorded verdict is never cached"
    );
    receipts::write(&w, "a.json", -60 - 7 * DAY, &[(FRESH, "fresh")], "acme-src")?;
    let expired = cached(&w)?;
    assert!(
        expired.contains("check-expired (7d)") && expired.contains("!recorded"),
        "{expired}"
    );
    let (text, exit) = kb(&w, &["stale", "--offline"], &[])?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        row(&text, "lib/fresh.c").contains("check-expired"),
        "{text}"
    );
    // Dated two hours ahead: unused.
    receipts::write(&w, "a.json", 2 * 3600, &[(FRESH, "fresh")], "acme-src")?;
    assert!(cached(&w)?.contains("not-checked"), "{}", cached(&w)?);
    receipts::write(&w, "a.json", 3000, &[(FRESH, "fresh")], "acme-src")?;
    assert_eq!(cached(&w)?, "", "less than an hour ahead counts as age 0");
    // Without max_age nothing is read.
    max_age(&w, None)?;
    assert!(cached(&w)?.contains("not-checked"), "{}", cached(&w)?);
    Ok(())
}

#[test]
fn the_newest_statement_wins_and_a_tie_is_the_worst() -> Result {
    let w = bare("rcp-newest", FRESH)?;
    let set = |fresh_at: i64, gone_at: i64| -> Result {
        receipts::write(&w, "a.json", fresh_at, &[(FRESH, "fresh")], "acme-src")?;
        receipts::write(&w, "b.json", gone_at, &[(FRESH, "gone")], "acme-src")
    };
    set(-7200, -3600)?;
    assert!(cached(&w)?.contains("[gone]"), "newer gone");
    set(-3600, -7200)?;
    assert_eq!(cached(&w)?, "", "newer fresh");
    set(-3600, -3600)?;
    assert!(cached(&w)?.contains("[gone]"), "a tie goes to the worst");
    Ok(())
}

#[test]
fn local_statements_against_recorded_ones() -> Result {
    // A newer local fresh beats an older recorded gone.
    let w = world("rcp-local", &[FRESH], true)?;
    max_age(&w, Some("7d"))?;
    receipts::write(&w, "a.json", -3600, &[(FRESH, "gone")], "acme-src")?;
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(cached(&w)?, "", "the local fresh row is newer");
    // A present, failing checker is seen, not replaced by a receipt.
    receipts::write(&w, "a.json", -60, &[(FRESH, "fresh")], "acme-src")?;
    let (text, exit) = kb(&w, &["stale"], &[("FAKE_MODE", "exit1")])?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        row(&text, "lib/fresh.c").contains("checker-failed (src): exit 1"),
        "{text}"
    );
    // Its newer local unverifiable row does not mask the recorded fresh.
    assert_eq!(cached(&w)?, "", "{}", cached(&w)?);
    Ok(())
}

#[test]
fn an_unreadable_receipt_is_skipped_whole_and_never_removed() -> Result {
    let breaks = [
        ("\"kind\": \"src\",", "\"kind\": \"src\", \"extra\": 1,"),
        ("\"status\":\"fresh\"", "\"status\":\"stale\""),
        ("\"status\":\"fresh\"", "\"status\":\"moved\""),
    ];
    for (n, (from, to)) in breaks.iter().enumerate() {
        let w = world(&format!("rcp-bad{n}"), &[FRESH], true)?;
        max_age(&w, Some("7d"))?;
        receipts::write(&w, "bad.json", -60, &[(FRESH, "fresh")], "acme-src")?;
        let bad = w.kb().join(receipts::DIR).join("bad.json");
        let text = std::fs::read_to_string(&bad)?.replacen(from, to, 1);
        std::fs::write(&bad, &text)?;
        let (out, _) = kb(&w, &["stale", "--cached"], &[])?;
        assert!(
            out.contains("# note: check receipt .writfold/checks/src/bad.json skipped: "),
            "{out}"
        );
        assert!(row(&out, "lib/fresh.c").contains("not-checked"), "{out}");
        kb(&w, &["stale", "--record"], &[])?;
        assert!(bad.exists(), "a skipped file is never removed");
        assert_eq!(files(&w).len(), 2);
    }
    let w = world("rcp-dup", &[FRESH], false)?;
    max_age(&w, Some("7d"))?;
    receipts::write(
        &w,
        "dup.json",
        -60,
        &[(FRESH, "fresh"), (FRESH, "fresh")],
        "acme-src",
    )?;
    let (out, _) = kb(&w, &["stale", "--cached"], &[])?;
    assert!(out.contains("skipped: duplicate token"), "{out}");
    Ok(())
}

#[test]
fn readers_start_no_checker_with_receipts_present() -> Result {
    let w = world("rcp-count", &[FRESH], true)?;
    max_age(&w, Some("7d"))?;
    kb(&w, &["stale", "--record"], &[])?;
    commit(&w, "receipt")?;
    let started = w.spawns();
    let repo = w.kb().display().to_string();
    let cover = [
        "gate",
        "cover",
        "review",
        "src",
        "supported",
        FRESH,
        "--by",
        "agent",
        "--run",
        RUN,
        "--repo",
        &repo,
    ];
    let (text, exit) = kb(&w, &cover, &[])?;
    assert_eq!(exit, 0, "{text}");
    kb(&w, &["gate", "status", "--run", RUN, "--repo", &repo], &[])?;
    kb(&w, &["stale", "--cached"], &[])?;
    kb(&w, &["stale", "--offline"], &[])?;
    kb(&w, &["why", "sources"], &[])?;
    kb(&w, &["checker", "list"], &[])?;
    assert_eq!(w.spawns(), started + 1, "only checker list's describe ran");
    let r = reader(&w, "rcp-count-read")?;
    let (text, exit) = kb(&r, &["stale"], &[])?;
    assert_eq!((exit, r.spawns()), (0, 0), "{text}");
    assert!(stamp(0)?.ends_with('Z'));
    Ok(())
}
