//! External checkers through the real binary: each status a checker can
//! give, each way an exchange can fail (none of them fresh), an absent
//! checker, and the process discipline (one spawn per kind per
//! `kb stale`, a hung checker killed at the deadline with nothing left
//! running).
mod checker;

use checker::{Result, kb, row, world};
use std::time::{Duration, Instant};

const TOKENS: [&str; 10] = [
    "⟦cite kind=src depot=main path=lib/fresh.c L=3-9 want=fresh⟧",
    "⟦cite kind=src depot=main path=lib/moved.c L=3-9 want=moved⟧",
    "⟦cite kind=src depot=main path=lib/changed.c want=changed⟧",
    "⟦cite kind=src depot=main path=lib/gone.c want=gone⟧",
    "⟦cite kind=src depot=main path=lib/unsure.c want=unverifiable⟧",
    "⟦cite kind=src depot=main path=lib/odd.c want=odd⟧",
    "⟦cite kind=src depot=main path=lib/dup.c want=dup⟧",
    "⟦cite kind=src depot=main path=lib/silent.c want=silent⟧",
    "⟦cite kind=src depot=main path=lib/next.c v=2 want=fresh⟧",
    "⟦cite kind=src path=lib/nodepot.c want=fresh⟧",
];

#[test]
fn each_reply_is_one_verdict_and_only_fresh_is_fresh() -> Result {
    let w = world("statuses", &TOKENS, true)?;
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!(exit, 2, "{text}");
    assert_eq!(w.spawns(), 1, "one process for the kind");
    let at = "at checker:acme-src@1.4.0:r42";
    assert_eq!(row(&text, "lib/fresh.c"), "", "{text}");
    let moved = row(&text, "lib/moved.c");
    assert!(
        moved.contains("[moved]") && moved.contains("!moved→lib/new.c:12-18"),
        "{moved}"
    );
    let expect = [
        ("lib/changed.c", format!("[changed] {at} said changed")),
        ("lib/gone.c", format!("[gone] {at} said gone")),
        (
            "lib/unsure.c",
            format!("[unverifiable] {at} checker: said unverifiable"),
        ),
        (
            "lib/odd.c",
            "checker-failed (src): unknown status \"stale\"".into(),
        ),
        (
            "lib/dup.c",
            "checker-failed (src): more than one reply".into(),
        ),
        ("lib/silent.c", "checker-failed (src): no reply".into()),
        (
            "lib/next.c",
            "token-version (v=2; writfold-check-acme-src reads [1])".into(),
        ),
        (
            "lib/nodepot.c",
            "missing depot= (required by .writfold/sources/src.toml)".into(),
        ),
    ];
    for (path, want) in expect {
        assert!(row(&text, path).contains(&want), "{path}: {want}\n{text}");
    }
    assert!(text.contains("checked=10 stale=3 unverifiable=6"), "{text}");
    assert!(!text.contains("checker-absent"), "{text}");
    Ok(())
}

#[test]
fn an_absent_checker_is_unverifiable_and_named() -> Result {
    let w = world("absent", &TOKENS[..2], false)?;
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!(exit, 2, "{text}");
    for path in ["lib/fresh.c", "lib/moved.c"] {
        let found = row(&text, path);
        assert!(
            found.contains("[unverifiable] at checker:acme-src checker-absent (src)"),
            "{text}"
        );
    }
    assert!(text.contains("unverifiable=2 checker-absent=2"), "{text}");
    let (list, exit) = kb(&w, &["checker", "list"], &[])?;
    assert_eq!(exit, 2, "{list}");
    assert!(
        list.starts_with("checker src [absent] writfold-check-acme-src is not on PATH"),
        "{list}"
    );
    // Offline never runs a checker and reports only marks and due TTLs.
    let (offline, exit) = kb(&w, &["stale", "--offline"], &[])?;
    assert_eq!(exit, 0, "{offline}");
    Ok(())
}

#[test]
fn a_failed_exchange_fails_every_token() -> Result {
    let tokens = &TOKENS[..2];
    let cases = [
        (
            "garbage",
            "checker-failed (src): line 2: a line that is not a JSON object",
        ),
        ("exit1", "checker-failed (src): exit 1"),
        (
            "nokind",
            "checker-failed (src): describe does not list kind src",
        ),
    ];
    for (mode, want) in cases {
        let w = world(mode, tokens, true)?;
        let (text, exit) = kb(&w, &["stale"], &[("FAKE_MODE", mode)])?;
        assert_eq!(exit, 2, "{text}");
        for path in ["lib/fresh.c", "lib/moved.c"] {
            assert!(row(&text, path).contains(want), "{mode}: {want}\n{text}");
        }
    }
    let w = world("hang", tokens, true)?;
    let pidfile = w.0.join("pid");
    let pid = pidfile.display().to_string();
    let started = Instant::now();
    let env = [
        ("FAKE_MODE", "hang"),
        ("FAKE_PIDFILE", pid.as_str()),
        ("WRITFOLD_CHECKER_TIMEOUT", "1"),
    ];
    let (text, exit) = kb(&w, &["stale"], &env)?;
    let took = started.elapsed();
    assert!(took < Duration::from_secs(20), "{took:?}");
    assert_eq!(exit, 2, "{text}");
    let want = "checker-failed (src): timed out after 1 s";
    assert!(row(&text, "lib/fresh.c").contains(want), "{text}");
    let child = std::fs::read_to_string(&pidfile)?;
    let stat = std::path::Path::new("/proc")
        .join(child.trim())
        .join("stat");
    assert!(!stat.exists(), "the checker's child {child} outlived it");
    Ok(())
}
