//! Generated review units (#156): a checklist whose items are the pairs of
//! a scope gate's kept manifest, each covered by a worker's report prepared
//! `--for` it. Five members are ten units; eight covered read 8/10 and the
//! two owed are named; a report prepared for another unit answers nothing;
//! without a current pass of the scope there are no units and the checklist
//! is not covered; a sixth member makes fifteen units and every earlier row
//! stale; 65 members are refused with the count. Coverage is not
//! correctness: a report that says the wrong thing still covers its unit.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

mod units_support;

use agent_support::outcome;
use helpers::RUN;
use support::{TestResult, World};
use units_support::{SCOPE, UNITS, cover, members, report, scope, status, world};
use writfold_kb::Verb;

const PAIRS: [&str; 10] = [
    "a~b", "a~c", "a~d", "a~e", "b~c", "b~d", "b~e", "c~d", "c~e", "d~e",
];

#[test]
fn ten_units_eight_covered_then_all_then_a_sixth_member() -> TestResult {
    let w = world()?;
    let text = status(&w);
    assert!(
        text.contains("the scope gate related-scope is open"),
        "{text}"
    );
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 0/10 units covered"), "{text}");
    let mut cands = Vec::new();
    for (n, unit) in PAIRS.iter().enumerate() {
        // Coverage is not correctness: these reports say anything at all.
        let c = report(&w, unit, "The two records agree.\n", 10 + n as i64)?;
        cands.push(c);
    }
    // A report prepared for another unit answers nothing.
    let (text, exit) = cover(&w, "a~b", &cands[1]);
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("none of the cited references does"), "{text}");
    for (unit, c) in PAIRS.iter().zip(&cands).take(8) {
        let (text, exit) = cover(&w, unit, c);
        assert_eq!(exit, 0, "{unit}: {text}");
    }
    let text = status(&w);
    assert!(text.contains("checklist 8/10 units covered"), "{text}");
    assert!(
        text.contains("next: writfold kb gate cover pair-review c~e"),
        "{text}"
    );
    let (text, exit) = helpers::advance(&w)?;
    assert_ne!(exit, 0, "the step advanced with two units owed: {text}");
    for (unit, c) in PAIRS.iter().zip(&cands).skip(8) {
        assert_eq!(cover(&w, unit, c).1, 0);
    }
    let text = status(&w);
    assert!(text.contains("checklist 10/10 units covered"), "{text}");
    assert!(text.contains("gate pair-review [attested]"), "{text}");
    // A sixth member: fifteen units, every earlier row stale.
    members(&w, &["a", "b", "c", "d", "e", "f"])?;
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 0/15 units covered"), "{text}");
    assert!(text.contains("a~f"), "{text}");
    assert!(text.contains("a~b [stale]"), "{text}");
    Ok(())
}

#[test]
fn over_sixty_four_members_is_refused_with_the_count() -> TestResult {
    let w = world()?;
    let ids: Vec<String> = (0..65).map(|i| format!("r{i:02}")).collect();
    members(&w, &ids.iter().map(String::as_str).collect::<Vec<_>>())?;
    scope(&w)?;
    let text = status(&w);
    assert!(
        text.contains("the scope has 65 members; pair units take at most 64"),
        "{text}"
    );
    assert!(text.contains("gate pair-review [open]"), "{text}");
    Ok(())
}

/// `delivery:` references (#168): no such candidate and a bound report
/// that is not ready ground nothing; a covered row is judged again on
/// read, so a moved closure stales it.
#[test]
fn a_delivery_reference_is_judged_when_cited_and_on_read() -> TestResult {
    let w = world()?;
    scope(&w)?;
    let (text, exit) = cover(&w, "a~b", "20261111T000000Z-000000");
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("is not grounded (no such candidate)"),
        "{text}"
    );
    let c = report(&w, "a~b", "The two records agree.\n", 10)?;
    assert_eq!(cover(&w, "a~b", &c).1, 0);
    // The profile edited: the closure moves, the report is unbound, and
    // the row that cites it reads stale.
    helpers::put(
        &w,
        ".writfold/agents/reviewer.toml",
        &agent_support::AGENT.replace("Review only", "Review just"),
    )?;
    let text = status(&w);
    assert!(text.contains("a~b [stale]"), "{text}");
    assert!(text.contains("cited evidence moved: delivery:"), "{text}");
    // A mode that also asks for a statement: bound, but not ready.
    let attest = format!("{}attest = [\"report.read\"]\n", units_support::MODE);
    helpers::put(&w, ".writfold/deliveries/review.toml", &attest)?;
    helpers::put(&w, ".writfold/agents/reviewer.toml", agent_support::AGENT)?;
    let c = report(&w, "a~c", "The two records agree.\n", 11)?;
    let (text, exit) = cover(&w, "a~c", &c);
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("is not grounded (not ready)"), "{text}");
    Ok(())
}

/// Commit `files` to the subject repository; its new HEAD.
fn commit(w: &World, files: &[(&str, &str)], message: &str) -> TestResult<String> {
    for (path, text) in files {
        let file = w.code.join(path);
        std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
        std::fs::write(file, text)?;
    }
    support::git(&w.code, &["add", "-A"])?;
    support::git(&w.code, &["commit", "-q", "--allow-empty", "-m", message])?;
    Ok(support::git(&w.code, &["rev-parse", "HEAD"])?
        .trim()
        .to_owned())
}

/// Pins: `handoff check` refuses a required gate that stands on workers'
/// reports. Every unit covered by a report bound to its invocation reads
/// `attested` (a statement), and a receiver accepts only a mechanical
/// pass. The reason shown is `no-evidence` (a checklist gate has no
/// receipt to find), not `statement`; that is reported on #168, so the
/// reason word is not pinned here.
#[test]
fn handoff_refuses_a_gate_that_stands_on_reports() -> TestResult {
    let w = world()?;
    scope(&w)?;
    for (n, unit) in PAIRS.iter().enumerate() {
        let c = report(&w, unit, "The two records agree.\n", 10 + n as i64)?;
        assert_eq!(cover(&w, unit, &c).1, 0);
    }
    let text = status(&w);
    assert!(text.contains("gate pair-review [attested]"), "{text}");
    let required = "schema = \"writfold-handoff/1\"\nrequired = [\"pair-review\"]\n";
    let rev = commit(
        &w,
        &[
            (".writfold/handoff.toml", required),
            (".writfold/gates/pair-review.toml", UNITS),
            (".writfold/gates/related-scope.toml", SCOPE),
        ],
        "policy",
    )?;
    let mut request = w.request(Verb::Handoff {
        policy: rev,
        candidate: "HEAD".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    request.cwd = w.code.clone();
    let (text, exit) = outcome(&request);
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("handoff pair-review [refused] "), "{text}");
    assert!(text.contains("handoff=refused"), "{text}");
    assert!(!text.contains("[accepted]"), "{text}");
    Ok(())
}
