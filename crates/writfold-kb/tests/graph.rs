//! Golden outputs of the M2 verbs (search, status, graph and index verify)
//! over the invented fixture KB. `WRITFOLD_KB_BLESS=1` rewrites them.
mod support;

use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;
use writfold_kb::verbs::impact::Direction;

fn golden(world: &World, name: &str, verb: Verb) -> TestResult<String> {
    let (text, exit) = world.run(&world.request(verb))?;
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{text}# exit={exit}\n");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(text);
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(text)
}

pub fn find(text: &str, signatures: bool) -> Verb {
    Verb::Find {
        text: text.into(),
        filter: Filter::default(),
        signatures,
    }
}

fn impact(target: &str, depth: usize, direction: Direction, inferred: bool) -> Verb {
    Verb::Impact {
        target: target.into(),
        depth,
        direction,
        inferred,
    }
}

/// The IDs of the rows (not continuation lines) of an answer.
fn ids(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with(' '))
        .filter_map(|l| l.split(' ').nth(1).map(str::to_owned))
        .collect()
}

#[test]
fn search_finds_exact_text_substring_and_typo() -> TestResult {
    let w = world(&[])?;
    golden(&w, "find-exact.txt", find("hbr 101", false))?;
    golden(&w, "find-text.txt", find("month end flush", false))?;
    let substring = golden(&w, "find-substring.txt", find("dger sto", false))?;
    assert!(
        ids(&substring).contains(&"harbor-db".to_owned()),
        "{substring}"
    );
    let typo = golden(&w, "find-typo.txt", find("harbr-api", false))?;
    assert_eq!(
        ids(&typo).first().map(String::as_str),
        Some("harbor-api"),
        "{typo}"
    );
    let title_typo = w.run(&w.request(find("Lantren", false)))?.0;
    assert!(
        ids(&title_typo).contains(&"lantern-ui".to_owned()),
        "{title_typo}"
    );
    golden(
        &w,
        "match-signature.txt",
        find("fsync took 30s; flush stalled for 12s", true),
    )?;
    golden(&w, "find-unknown.txt", find("zeppelin mooring", true))?;
    let mut kinds = w.request(find("ledger", false));
    if let Verb::Find { filter, .. } = &mut kinds.verb {
        filter.kinds = vec!["decision".into()];
        filter.project = Some("harbor".into());
    }
    let (narrowed, _) = w.run(&kinds)?;
    assert!(
        narrowed
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with(' '))
            .all(|l| l.starts_with("decision ")),
        "{narrowed}"
    );
    Ok(())
}

#[test]
fn status_states_lifecycle_and_trackers() -> TestResult {
    let w = world(&[])?;
    let target = |t: &str| Verb::Status { target: t.into() };
    golden(&w, "status-ticket.txt", target("HBR-101"))?;
    golden(
        &w,
        "status-decision.txt",
        target("decision/2026-01-10-single-writer"),
    )?;
    golden(&w, "status-asset.txt", target("harbor-api"))?;
    golden(&w, "status-project.txt", target("harbor"))?;
    golden(&w, "status-unknown.txt", target("HBR-110"))?;
    Ok(())
}

#[test]
fn impact_states_frontier_and_coverage() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(Verb::Stale {
        scope: None,
        offline: false,
        cached: false,
    }))?;
    let both = golden(
        &w,
        "impact-asset.txt",
        impact("harbor-api", 2, Direction::Both, false),
    )?;
    let trailer = both.lines().last().unwrap_or_default();
    assert!(
        trailer.contains(" frontier=") && trailer.contains("coverage=partial"),
        "{trailer}"
    );
    golden(
        &w,
        "impact-inferred.txt",
        impact("harbor-db", 1, Direction::In, true),
    )?;
    let complete = golden(
        &w,
        "impact-out.txt",
        impact("HBR-99", 3, Direction::Out, false),
    )?;
    let trailer = complete.lines().last().unwrap_or_default();
    assert!(
        trailer.contains("coverage=complete") && trailer.contains(" frontier=0 "),
        "{trailer}"
    );
    golden(
        &w,
        "impact-unknown.txt",
        impact("harbour-api", 2, Direction::Both, false),
    )?;
    Ok(())
}

#[test]
fn uses_and_timeline() -> TestResult {
    let w = world(&[])?;
    let uses = |t: &str| Verb::Uses { target: t.into() };
    golden(&w, "uses-asset.txt", uses("harbor-api"))?;
    golden(&w, "uses-signature.txt", uses("sig:fsync-stall"))?;
    let code = format!("demo-service@{}:src/ledger.rs:4-4", "0".repeat(40));
    let located = w.run(&w.request(uses(&code)))?.0;
    let named = golden(
        &w,
        "uses-code.txt",
        uses("code:demo-service:src/ledger.rs:4-4"),
    )?;
    assert_eq!(ids(&located), ids(&named));
    golden(
        &w,
        "timeline-asset.txt",
        Verb::Timeline {
            target: "harbor-db".into(),
        },
    )?;
    golden(
        &w,
        "timeline-ticket.txt",
        Verb::Timeline {
            target: "HBR-101".into(),
        },
    )?;
    Ok(())
}

#[test]
fn index_verify_equals_a_rebuild() -> TestResult {
    let w = world(&[])?;
    golden(&w, "index-verify.txt", Verb::IndexVerify)?;
    Ok(())
}

fn cursor_of(text: &str) -> Option<String> {
    let trailer = text.lines().last()?;
    let cursor = trailer.split(' ').find_map(|p| p.strip_prefix("cursor="))?;
    (cursor != "none").then(|| cursor.to_owned())
}

fn rows_of(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn pages_of_the_new_verbs_join_to_the_whole() -> TestResult {
    let w = world(&[])?;
    for verb in [
        find("ledger flush", true),
        impact("harbor-api", 3, Direction::Both, true),
        Verb::Uses {
            target: "harbor-api".into(),
        },
        Verb::Status {
            target: "harbor".into(),
        },
        Verb::Timeline {
            target: "harbor-db".into(),
        },
    ] {
        let mut whole = w.request(verb.clone());
        whole.budget = writfold_kb::MAX_BUDGET;
        let expected = rows_of(&w.run(&whole)?.0);
        let mut request = w.request(verb.clone());
        request.budget = 90;
        let mut joined = Vec::new();
        for _ in 0..100 {
            let text = w.run(&request)?.0;
            joined.extend(rows_of(&text));
            request.cursor = cursor_of(&text);
            if request.cursor.is_none() {
                break;
            }
        }
        assert_eq!(joined, expected, "{verb:?}");
    }
    Ok(())
}

#[test]
fn a_second_claim_of_an_id_is_unreadable_until_the_first_goes() -> TestResult {
    let w = world(&[])?;
    let copy = std::fs::read_to_string(w.kb.join("ktlo/2026/hbr-99-flush-stall.md"))?;
    std::fs::write(w.kb.join("ktlo/2026/hbr-99-zz-copy.md"), &copy)?;
    let (status, _) = w.run(&w.request(Verb::IndexStatus))?;
    let listed = "unreadable ticket duplicate id HBR-99 @ktlo/2026/hbr-99-zz-copy.md";
    assert!(status.contains(listed), "{status}");
    std::fs::remove_file(w.kb.join("ktlo/2026/hbr-99-flush-stall.md"))?;
    let (moved, exit) = w.run(&w.request(Verb::Status {
        target: "HBR-99".into(),
    }))?;
    assert!(moved.starts_with("ticket HBR-99 [done]"), "{moved}");
    assert!(moved.contains("@ktlo/2026/hbr-99-zz-copy.md"), "{moved}");
    assert_eq!(exit, 0, "{moved}");
    let (verified, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert_eq!(exit, 0, "{verified}");
    Ok(())
}
