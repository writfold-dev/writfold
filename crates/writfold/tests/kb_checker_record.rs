//! `kb stale --record` through the binary with the fake checker: a
//! recording clone writes check receipts and a reading clone, with no
//! checker on `PATH` and no index, reads them, labelled `recorded`.
mod checker;

use checker::receipts::{self, commit, files, max_age, only, reader};
use checker::{RUN, Result, Root, kb, row, world};

const FRESH: &str = "⟦cite kind=src depot=main path=lib/fresh.c L=3-9 want=fresh⟧";
const GONE: &str = "⟦cite kind=src depot=main path=lib/gone.c want=gone⟧";

/// A recording world with `tokens`, `src` declared with `max_age = 7d`.
fn recorder(name: &str, tokens: &[&str]) -> Result<Root> {
    let w = world(name, tokens, true)?;
    max_age(&w, Some("7d"))?;
    Ok(w)
}

fn cover(w: &Root, token: &str) -> Result<(String, i32)> {
    let repo = w.kb().display().to_string();
    let args = [
        "gate",
        "cover",
        "review",
        "src",
        "supported",
        token,
        "--by",
        "agent",
        "--run",
        RUN,
        "--repo",
        &repo,
    ];
    kb(w, &args, &[])
}

#[test]
fn a_reading_clone_grounds_on_a_receipt_without_the_checker() -> Result {
    let w = recorder("rec-share", &[FRESH, GONE])?;
    let (text, exit) = kb(&w, &["stale", "--record"], &[])?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("recorded-now=2 receipts=1"), "{text}");
    assert!(
        text.contains("# note: recorded 2 verdicts of kind src in .writfold/checks/src/"),
        "{text}"
    );
    assert!(
        text.contains("# note: written to the work tree; commit it to share it"),
        "{text}"
    );
    commit(&w, "receipt")?;
    let r = reader(&w, "rec-share-read")?;
    let (text, exit) = kb(&r, &["stale"], &[])?;
    assert_eq!(exit, 2, "{text}");
    assert_eq!(r.spawns(), 0);
    let gone = row(&text, "lib/gone.c");
    assert!(
        gone.contains("[gone] at checker:acme-src@1.4.0:r42 recorded "),
        "{text}"
    );
    assert!(gone.ends_with("!gone !recorded"), "{gone}");
    assert!(text.contains("\n  check: recorded "), "{text}");
    assert!(
        text.contains(" by acme-src 1.4.0 @.writfold/checks/src/"),
        "{text}"
    );
    assert!(
        text.contains("2 verdicts were read from check receipts: checked elsewhere, at "),
        "{text}"
    );
    assert!(
        text.contains("stale=1 unverifiable=0 recorded=2 "),
        "{text}"
    );
    // Only the fresh token: exit 0, counted in the trailer, no row.
    std::fs::write(
        r.kb().join("notes/sources.md"),
        format!("# Sources\n\n{FRESH}\n"),
    )?;
    let (text, exit) = kb(&r, &["stale"], &[])?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("stale=0 unverifiable=0 recorded=1 "),
        "{text}"
    );
    let (text, exit) = cover(&r, FRESH)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("src [supported] 1 reference by agent"),
        "{text}"
    );
    assert!(text.contains("!recorded"), "{text}");
    assert!(text.contains("# note: 1 reference is fresh by a check receipt (checked elsewhere); writfold kb stale --cached dates them"), "{text}");
    let repo = r.kb().display().to_string();
    let (text, _) = kb(&r, &["gate", "status", "--run", RUN, "--repo", &repo], &[])?;
    assert!(
        text.starts_with("gate review [attested]") && text.contains("!recorded"),
        "{text}"
    );
    let (text, _) = kb(&r, &["gate", "lines", "--run", RUN, "--repo", &repo], &[])?;
    assert!(
        text.contains("note: 1 reference is fresh by a check receipt"),
        "{text}"
    );
    let (text, _) = kb(&r, &["check"], &[])?;
    assert!(text.contains("recorded=1"), "{text}");
    assert_eq!(r.spawns(), 0, "no reader started a checker");
    Ok(())
}

#[test]
fn only_a_checkers_statement_about_the_token_is_recorded() -> Result {
    let tokens = [
        FRESH,
        "⟦cite kind=src depot=main path=lib/unsure.c want=unverifiable⟧",
        "⟦cite kind=src depot=main path=lib/odd.c want=odd⟧",
        "⟦cite kind=src depot=main path=lib/next.c v=2 want=fresh⟧",
        "⟦cite kind=src depot=main path=lib/marked.c want=fresh stale=old⟧",
    ];
    let w = recorder("rec-only", &tokens)?;
    // An older receipt holding the unverifiable token's text survives.
    receipts::write(
        &w,
        "20200101T000000Z-aaaaaa.json",
        -3600,
        &[(tokens[1], "gone")],
        "acme-src",
    )?;
    let (text, _) = kb(&w, &["stale", "--record"], &[])?;
    assert!(text.contains("recorded-now=2 receipts=1"), "{text}");
    assert!(row(&text, "lib/marked.c").contains("[marked]"), "{text}");
    assert_eq!(files(&w).len(), 2, "{text}");
    let new = std::fs::read_to_string(&files(&w)[1])?;
    assert_eq!(new.matches("\"status\":\"fresh\"").count(), 2, "{new}");
    for token in &tokens[1..4] {
        assert!(
            !new.contains(&receipts::digest(token)?),
            "{token} recorded:\n{new}"
        );
    }
    assert!(
        new.contains(&receipts::digest(tokens[4])?),
        "the checker's answer, before the mark"
    );
    // With no checker on PATH nothing is recordable.
    let r = reader(&w, "rec-only-absent")?;
    max_age(&r, Some("7d"))?;
    let (text, _) = kb(&r, &["stale", "--record"], &[])?;
    assert!(text.contains("recorded-now=0 receipts=0"), "{text}");
    Ok(())
}

#[test]
fn record_refuses_offline_and_cached_and_needs_max_age() -> Result {
    let w = world("rec-refuse", &[FRESH], true)?;
    for flag in ["--offline", "--cached"] {
        let (text, exit) = kb(&w, &["stale", "--record", flag], &[])?;
        assert_eq!(exit, 1, "{text}");
        assert!(text.contains("--record needs the checkers to be asked; it cannot be combined with --offline or --cached"), "{text}");
    }
    assert_eq!(w.spawns(), 0);
    let (text, exit) = kb(&w, &["stale", "--record"], &[])?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("# note: not recorded: kind src declares no max_age (a recorded verdict needs a lifetime)"), "{text}");
    assert!(files(&w).is_empty());
    Ok(())
}

#[test]
fn a_scoped_recording_keeps_receipts_holding_other_tokens() -> Result {
    let w = recorder("rec-scope", &[FRESH])?;
    checker::put(
        &w.kb().join("other/more.md"),
        &format!("# More\n\n{GONE}\n"),
    )?;
    commit(&w, "more")?;
    kb(&w, &["stale", "notes/sources.md", "--record"], &[])?;
    assert_eq!(files(&w).len(), 1);
    let (text, _) = kb(&w, &["stale", "--record"], &[])?;
    assert!(text.contains("(1 superseded receipt removed)"), "{text}");
    assert_eq!(files(&w).len(), 1, "the scoped receipt was fully covered");
    kb(&w, &["stale", "notes/sources.md", "--record"], &[])?;
    assert_eq!(
        files(&w).len(),
        2,
        "the full receipt holds an out-of-scope token"
    );
    Ok(())
}

#[test]
fn two_clones_record_and_merge_cleanly() -> Result {
    let w = recorder("rec-merge", &[FRESH])?;
    let a = reader(&w, "rec-merge-a")?;
    let b = reader(&w, "rec-merge-b")?;
    for clone in [&a, &b] {
        std::fs::copy(
            w.bin().join("writfold-check-acme-src"),
            clone.bin().join("writfold-check-acme-src"),
        )?;
        let (text, _) = kb(clone, &["stale", "--record"], &[])?;
        assert!(text.contains("receipts=1"), "{text}");
        commit(clone, "record")?;
    }
    let other = b.kb().display().to_string();
    checker::git(
        &a.kb(),
        &["pull", "-q", "--no-rebase", "--no-edit", &other, "HEAD"],
    )?;
    assert_eq!(files(&a).len(), 2);
    let (text, exit) = kb(&a, &["stale", "--cached"], &[])?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn json_carries_the_flag_the_check_line_and_the_counts() -> Result {
    let w = recorder("rec-json", &[GONE])?;
    let (text, _) = kb(&w, &["stale", "--record", "--json"], &[])?;
    let body: serde_json::Value = serde_json::from_str(text.lines().next().unwrap_or_default())?;
    assert_eq!(body["trailer"]["counts"]["recorded-now"], 1, "{text}");
    assert_eq!(body["trailer"]["counts"]["receipts"], 1, "{text}");
    commit(&w, "receipt")?;
    let r = reader(&w, "rec-json-read")?;
    let (text, _) = kb(&r, &["stale", "--json"], &[])?;
    let body: serde_json::Value = serde_json::from_str(text.lines().next().unwrap_or_default())?;
    let first = &body["rows"][0];
    assert!(
        first["flags"]
            .as_array()
            .is_some_and(|f| f.iter().any(|x| x == "recorded")),
        "{text}"
    );
    assert_eq!(first["more"][0]["label"], "check", "{text}");
    assert_eq!(first["more"][0]["line"], 1, "{text}");
    assert_eq!(body["trailer"]["counts"]["recorded"], 1, "{text}");
    let (text, _) = kb(&r, &["checker", "list", "--json"], &[])?;
    let body: serde_json::Value = serde_json::from_str(text.lines().next().unwrap_or_default())?;
    assert_eq!(body["rows"][0]["more"][0]["label"], "receipts", "{text}");
    let (text, _) = kb(&r, &["checker", "list"], &[])?;
    assert!(
        text.contains("\n  receipts: 1 verdicts in 1 files, newest "),
        "{text}"
    );
    assert!(text.contains(", max_age 7d"), "{text}");
    max_age(&r, None)?;
    let (text, _) = kb(&r, &["checker", "list"], &[])?;
    assert!(
        text.contains("\n  receipts: 1 files, not read: no max_age"),
        "{text}"
    );
    let (text, _) = kb(&r, &["stale"], &[])?;
    assert!(
        row(&text, "lib/gone.c").contains("checker-absent (src)"),
        "{text}"
    );
    let _ = only(&r)?;
    Ok(())
}
