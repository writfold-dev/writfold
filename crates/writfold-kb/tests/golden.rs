//! Golden outputs over the invented fixture KB. Outputs are byte-identical
//! for the same snapshot and query. `WRITFOLD_KB_BLESS=1` rewrites them.
mod support;

use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::{Request, Verb};

fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{text}# exit={exit}\n");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::create_dir_all(file.parent().ok_or("golden directory")?)?;
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

fn check(world: &World, name: &str, request: &Request) -> TestResult<String> {
    let (text, exit) = world.run(request)?;
    golden(name, &text, exit)?;
    Ok(text)
}

fn brief(project: Option<&str>, all: bool) -> Verb {
    Verb::Brief {
        project: project.map(str::to_owned),
        all,
    }
}

fn context(target: &str) -> Verb {
    Verb::Context {
        target: target.into(),
        depth: 2,
    }
}

fn why(text: &str) -> Verb {
    Verb::Why {
        text: text.into(),
        sections: Vec::new(),
    }
}

#[test]
fn verbs_over_the_fixture() -> TestResult {
    let w = world(&[])?;
    check(&w, "index-status.txt", &w.request(Verb::IndexStatus))?;
    check(
        &w,
        "brief-harbor.txt",
        &w.request(brief(Some("harbor"), false)),
    )?;
    check(&w, "brief-all.txt", &w.request(brief(None, true)))?;
    check(
        &w,
        "brief-unknown-project.txt",
        &w.request(brief(Some("atlantis"), false)),
    )?;
    let mut inferred = w.request(brief(None, false));
    inferred.cwd = w.kb.join("harbor/log");
    check(&w, "brief-inferred.txt", &inferred)?;
    check(&w, "brief-nohome.txt", &w.request(brief(None, false)))?;
    check(&w, "context-asset.txt", &w.request(context("harbor-api")))?;
    check(&w, "context-ticket.txt", &w.request(context("hbr 101")))?;
    check(
        &w,
        "context-alias-ambiguous.txt",
        &w.request(context("ledger")),
    )?;
    check(
        &w,
        "context-unknown.txt",
        &w.request(context("flush stalls")),
    )?;
    check(&w, "why-text.txt", &w.request(why("batch ledger")))?;
    check(
        &w,
        "why-superseded.txt",
        &w.request(why("decision/2026-01-10-single-writer")),
    )?;
    check(&w, "why-draft-only.txt", &w.request(why("shard")))?;
    check(&w, "why-unknown.txt", &w.request(why("zeppelin mooring")))?;
    let mut json = w.request(why("batch ledger"));
    json.json = true;
    check(&w, "why-text.json", &json)?;
    check(
        &w,
        "stale-offline.txt",
        &w.request(Verb::Stale {
            scope: None,
            offline: true,
            cached: false,
        }),
    )?;
    check(
        &w,
        "stale-all.txt",
        &w.request(Verb::Stale {
            scope: None,
            offline: false,
            cached: false,
        }),
    )?;
    check(
        &w,
        "stale-project.txt",
        &w.request(Verb::Stale {
            scope: Some("harbor".into()),
            offline: false,
            cached: false,
        }),
    )?;
    check(
        &w,
        "brief-harbor-after-stale.txt",
        &w.request(brief(Some("harbor"), false)),
    )?;
    check(
        &w,
        "why-text-after-stale.txt",
        &w.request(why("batch ledger")),
    )?;
    Ok(())
}

fn cursor_of(text: &str) -> Option<String> {
    let trailer = text.lines().last()?;
    let cursor = trailer
        .split(' ')
        .find_map(|part| part.strip_prefix("cursor="))?;
    (cursor != "none").then(|| cursor.to_owned())
}

fn rows_of(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn budget_cut_off_and_cursor_resume() -> TestResult {
    let w = world(&[])?;
    let mut tiny = w.request(context("harbor-api"));
    tiny.budget = 20;
    let (nothing, _) = w.run(&tiny)?;
    assert!(
        rows_of(&nothing).is_empty()
            && nothing.contains(" needs=")
            && nothing.contains(" cursor=none ")
    );
    let mut first = w.request(context("harbor-api"));
    first.budget = 120;
    let page = check(&w, "context-budget-page1.txt", &first)?;
    let mut second = first.clone();
    second.cursor = cursor_of(&page);
    check(&w, "context-budget-page2.txt", &second)?;
    let mut foreign = w.request(context("harbor-db"));
    foreign.budget = 120;
    foreign.cursor = cursor_of(&page);
    match writfold_kb::run(&foreign) {
        Err(error) => assert_eq!(
            (
                error.exit_code(),
                error.to_string().contains("different program")
            ),
            (3, true)
        ),
        Ok(_) => return Err("a cursor from another query must be refused".into()),
    }
    for verb in [
        context("harbor-api"),
        brief(Some("harbor"), false),
        Verb::Stale {
            scope: None,
            offline: true,
            cached: false,
        },
    ] {
        let mut whole = w.request(verb.clone());
        whole.budget = writfold_kb::MAX_BUDGET;
        let expected = rows_of(&w.run(&whole)?.0);
        for budget in [100, 150, 400] {
            let mut request = w.request(verb.clone());
            request.budget = budget;
            let mut joined = Vec::new();
            for _ in 0..200 {
                let text = w.run(&request)?.0;
                if rows_of(&text).is_empty() {
                    return Err(format!("{verb:?}: no row fits budget {budget}: {text}").into());
                }
                joined.extend(rows_of(&text));
                request.cursor = cursor_of(&text);
                if request.cursor.is_none() {
                    break;
                }
            }
            // A brief orders rows by section shares of its budget, so its
            // pages hold the same rows in a budget-dependent order.
            if matches!(verb, Verb::Brief { .. }) {
                let (mut a, mut b) = (joined.clone(), expected.clone());
                a.sort();
                b.sort();
                assert_eq!(a, b, "{verb:?} at budget {budget}");
            } else {
                assert_eq!(joined, expected, "{verb:?} at budget {budget}");
            }
        }
    }
    Ok(())
}

#[test]
fn unreadable_files_are_counted_and_listed() -> TestResult {
    let w = world(&["malformed"])?;
    let status = check(
        &w,
        "index-status-unreadable.txt",
        &w.request(Verb::IndexStatus),
    )?;
    assert_eq!(
        status
            .lines()
            .filter(|l| l.starts_with("unreadable "))
            .count(),
        5
    );
    let (brief, exit) = w.run(&w.request(brief(Some("harbor"), false)))?;
    assert!(
        brief
            .lines()
            .last()
            .is_some_and(|t| t.contains(" unreadable=5 ") && t.contains("coverage=partial"))
    );
    assert_eq!(exit, 2);
    Ok(())
}

#[test]
fn snapshot_changes_refresh_and_equal_snapshots_reuse() -> TestResult {
    let w = world(&[])?;
    let trailer = |text: &str| text.lines().last().unwrap_or_default().to_owned();
    let (first, _) = w.run(&w.request(Verb::IndexStatus))?;
    assert!(trailer(&first).contains("index=rebuilt"));
    let (again, _) = w.run(&w.request(Verb::IndexStatus))?;
    assert!(trailer(&again).contains("index=fresh"));
    let ticket = w.kb.join("ktlo/2026/hbr-101-slow-ledger-flush.md");
    let text = std::fs::read_to_string(&ticket)?.replace("status: in-progress", "status: done");
    std::fs::write(&ticket, text)?;
    let (dirty, _) = w.run(&w.request(context("HBR-101")))?;
    assert!(
        trailer(&dirty).contains("+dirty:1 index=refreshed:1"),
        "{dirty}"
    );
    assert!(dirty.starts_with("ticket HBR-101 [done]"), "{dirty}");
    std::fs::write(w.kb.join("notes.txt"), "not a KB input")?;
    let (unchanged, _) = w.run(&w.request(context("HBR-101")))?;
    assert!(
        trailer(&unchanged).contains("+dirty:1 index=fresh"),
        "{unchanged}"
    );
    support::git(&w.kb, &["commit", "-qam", "close HBR-101"])?;
    let (committed, _) = w.run(&w.request(context("HBR-101")))?;
    // The commit moves HEAD; the file it holds is the one already indexed
    // dirty, so nothing is re-derived.
    assert!(
        trailer(&committed).contains("index=refreshed:0") && !trailer(&committed).contains("dirty"),
        "{committed}"
    );
    support::git(&w.kb, &["reset", "-q", "--hard", "HEAD~1"])?;
    let (reset, _) = w.run(&w.request(context("HBR-101")))?;
    assert!(trailer(&reset).contains("index=refreshed:1"), "{reset}");
    assert!(reset.starts_with("ticket HBR-101 [in-progress]"), "{reset}");
    Ok(())
}

#[test]
fn a_kb_outside_git_reads_with_no_pin() -> TestResult {
    let w = world(&[])?;
    let plain = w.base.join("plain");
    support::git(&w.base, &["clone", "-q", "--no-hardlinks", "kb", "plain"])?;
    std::fs::remove_dir_all(plain.join(".git"))?;
    let mut request = w.request(context("harbor-api"));
    request.kb = Some(plain);
    let (text, _) = w.run(&request)?;
    let trailer = text.lines().last().unwrap_or_default();
    assert!(trailer.contains(" kb=none index=rebuilt "), "{trailer}");
    assert!(text.starts_with("asset harbor-api [active]"));
    Ok(())
}
