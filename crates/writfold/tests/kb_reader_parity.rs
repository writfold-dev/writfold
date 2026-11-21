//! Cold readers show the verdict the gates use (#35): on a second clone
//! with no checker and no local check, `brief`, `context`, `why` and the
//! session-start count read a committed check receipt the
//! way `stale --cached` does, start no checker, and write nothing.
mod checker;

use checker::receipts::{self, commit, max_age, reader};
use checker::{Result, Root, cached_rows, kb, put, row, run, world};

const GONE: &str = "⟦cite kind=src depot=main path=lib/gone.c L=3-9 want=gone⟧";
const DECISION: &str = "decision/2026-11-01-one";
const DAY: i64 = 86_400;

/// A world whose decision record cites `GONE`, `max_age = 7d`, committed.
fn cited(name: &str, fake: bool) -> Result<Root> {
    let w = world(name, &[GONE], fake)?;
    put(
        &w.kb().join("proposals/decision/2026-11-01-one.md"),
        &format!(
            "---\nid: {DECISION}\nkind: decision\ntitle: Pick one\nstatus: accepted\ncreated: 2026-11-01\nupdated: 2026-11-01\nproject: harbor\n---\n\n## Context\n\nWe chose it. {GONE}\n"
        ),
    )?;
    max_age(&w, Some("7d"))?;
    Ok(w)
}

/// The decision's cite row in `context`, its `cite:` line in `why`, the
/// rows of `brief --all` and the session-start citations line.
struct Seen {
    context: String,
    why: String,
    brief: String,
    start: String,
}

fn seen(w: &Root) -> Result<Seen> {
    let (context, _) = kb(w, &["context", DECISION], &[])?;
    let (why, _) = kb(w, &["why", DECISION], &[])?;
    let (brief, _) = kb(w, &["brief", "--all"], &[])?;
    let hook = "{\"hook_event_name\":\"SessionStart\",\"source\":\"startup\",\"session_id\":\"s\"}";
    let out = run(w, &["kb", "hook", "session-start"], &[], hook)?;
    let start = String::from_utf8_lossy(&out.stdout).into_owned();
    let start = (start.split("\\n"))
        .find(|l| l.starts_with("Citations:"))
        .unwrap_or("")
        .to_owned();
    let line = |text: &str, prefix: &str| -> String {
        let mut lines = text.lines().skip_while(|l| !l.starts_with(prefix));
        let first = lines.next().unwrap_or("").to_owned();
        let more = lines
            .take_while(|l| l.starts_with("  check:"))
            .collect::<Vec<_>>();
        [first]
            .into_iter()
            .chain(more.into_iter().map(str::to_owned))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(Seen {
        context: line(&context, "cite src:lib/gone.c"),
        why: why
            .lines()
            .find(|l| l.starts_with("  cite: "))
            .unwrap_or("")
            .to_owned(),
        brief: line(&brief, "cite src:lib/gone.c"),
        start,
    })
}

#[test]
fn a_second_clone_shows_the_committed_gone_before_any_local_check() -> Result {
    let a = cited("par-two", true)?;
    let (text, exit) = kb(&a, &["stale", "--record"], &[])?;
    assert_eq!(exit, 2, "{text}");
    commit(&a, "receipt")?;
    let started = a.spawns();
    let b = reader(&a, "par-two-b")?;
    kb(&b, &["index", "status"], &[])?;
    let before = (cached_rows(&b)?, status(&b)?);
    let (stale, _) = kb(&b, &["stale", "--cached"], &[])?;
    let cached = row(&stale, "lib/gone.c");
    assert!(
        cached.contains("[gone]") && cached.contains("!recorded"),
        "{stale}"
    );
    let receipt = receipts::files(&b);
    let [receipt] = receipt.as_slice() else {
        return Err(format!("{receipt:?}").into());
    };
    let name = receipt.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    let s = seen(&b)?;
    for shown in [&s.context, &s.brief] {
        assert!(
            shown.contains("[gone]") && shown.contains("!gone !recorded"),
            "{shown}"
        );
        assert!(
            shown.contains("  check: recorded ") && shown.contains(name),
            "the receipt is named: {shown}"
        );
    }
    assert!(
        s.why.contains("[gone] recorded ") && s.why.contains(name) && s.why.contains("!recorded"),
        "{}",
        s.why
    );
    assert!(
        s.start
            .contains("2 gone (2 read from committed check receipts)"),
        "{}",
        s.start
    );
    assert_eq!((b.spawns(), a.spawns()), (0, started), "no checker started");
    assert_eq!(
        (cached_rows(&b)?, status(&b)?),
        before,
        "presentation writes no cache row and no tracked file"
    );
    Ok(())
}

/// `git status --porcelain` of the KB.
fn status(w: &Root) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(w.kb())
        .output()?;
    Ok(String::from_utf8(out.stdout)?)
}

/// A clone of a KB whose only observation is a hand-written receipt
/// stating `status` for `GONE`, `offset` seconds from now, by `checker`.
fn handwritten(name: &str, offset: i64, status: &str, checker: &str) -> Result<(Root, Root)> {
    let a = cited(name, false)?;
    receipts::write(&a, "a.json", offset, &[(GONE, status)], checker)?;
    commit(&a, "receipt")?;
    let b = reader(&a, &format!("{name}-b"))?;
    Ok((a, b))
}

#[test]
fn readers_age_a_receipt_as_the_gates_do() -> Result {
    // Just within max_age: a fresh receipt is shown as nothing to warn of.
    let (_a, b) = handwritten("par-in", 60 - 7 * DAY, "fresh", "acme-src")?;
    let s = seen(&b)?;
    assert_eq!(
        (s.context.as_str(), s.brief.as_str()),
        ("", ""),
        "fresh adds no row"
    );
    assert!(s.why.contains("[fresh] recorded "), "{}", s.why);
    assert_eq!(s.start, "", "nothing to count");
    // Just beyond it: unverifiable, check-expired, from the receipt.
    let (_a, b) = handwritten("par-out", -60 - 7 * DAY, "fresh", "acme-src")?;
    let s = seen(&b)?;
    for shown in [&s.context, &s.brief] {
        assert!(
            shown.contains("[unverifiable]") && shown.contains("!recorded"),
            "{shown}"
        );
    }
    assert!(s.why.contains("[unverifiable]"), "{}", s.why);
    assert!(s.start.contains("2 unverifiable (2 read"), "{}", s.start);
    Ok(())
}

#[test]
fn an_ignored_receipt_and_no_observation_read_as_unchecked() -> Result {
    let (_a, b) = handwritten("par-wrong", -60, "gone", "other-src")?;
    let s = seen(&b)?;
    assert_eq!(
        (s.context.as_str(), s.brief.as_str()),
        ("", ""),
        "{}",
        s.brief
    );
    assert!(
        s.why.contains("[unchecked]") && !s.why.contains("recorded"),
        "{}",
        s.why
    );
    assert_eq!(s.start, "");
    // No max_age: receipts are not read at all.
    let (a, _) = handwritten("par-none", -60, "gone", "acme-src")?;
    max_age(&a, None)?;
    let b = reader(&a, "par-none-c")?;
    let s = seen(&b)?;
    assert!(s.why.contains("[unchecked]"), "{}", s.why);
    assert_eq!((s.brief.as_str(), s.start.as_str()), ("", ""));
    Ok(())
}

#[test]
fn a_newer_local_answer_beats_an_older_receipt() -> Result {
    let a = cited("par-local", true)?;
    receipts::write(&a, "a.json", -3600, &[(GONE, "fresh")], "acme-src")?;
    commit(&a, "receipt")?;
    // The local checker answers gone now: newer than the recorded fresh.
    let (text, exit) = kb(&a, &["stale"], &[])?;
    assert_eq!(exit, 2, "{text}");
    let s = seen(&a)?;
    assert!(
        s.context.contains("[gone]") && !s.context.contains("recorded"),
        "{}",
        s.context
    );
    assert!(
        s.start.contains("2 gone") && !s.start.contains("read from"),
        "{}",
        s.start
    );
    Ok(())
}

#[test]
fn an_unreadable_kind_declaration_is_said_not_hidden() -> Result {
    let (_a, b) = handwritten("par-broken", -60, "gone", "acme-src")?;
    let s = seen(&b)?;
    assert!(s.why.contains("[gone]"), "{}", s.why);
    // The declaration stops parsing: readers say so instead of showing
    // the committed gone as never checked.
    put(
        &b.kb().join(".writfold/sources/src.toml"),
        "schema = \"writfold-source/1\"\nkind = \n",
    )?;
    let (why, exit) = kb(&b, &["why", DECISION], &[])?;
    assert_eq!(exit, 2, "{why}");
    assert!(
        why.contains("a source kind declaration (.writfold/sources/) could not be read"),
        "{why}"
    );
    assert!(why.contains("coverage=partial"), "{why}");
    let s = seen(&b)?;
    assert!(
        s.start.starts_with("Citations: could not be read"),
        "{}",
        s.start
    );
    Ok(())
}
