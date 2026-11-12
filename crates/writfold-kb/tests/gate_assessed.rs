//! The Weft assessed gate over the fixture world. A gate of class
//! `assessed` names a Weft source, its required checks and the binary's
//! SHA-256; `gate run` grounds each assumption's citations, runs Weft and
//! records a receipt; a check is met only when entailed from grounded
//! assumptions. Most tests use the labelled TEST DOUBLE (`assessed`); the
//! test marked `#[ignore]` runs a real Weft binary named by
//! `$WRITFOLD_TEST_WEFT`:
//! `WRITFOLD_TEST_WEFT=/path/to/weft cargo test -p writfold-kb --test gate_assessed -- --ignored`.
mod assessed;
mod support;

use assessed::{
    RUN, SOURCE, assess, define, double, gate, golden, put, receipts, sha, state_of, status, unit,
};
use std::path::PathBuf;
use support::{NOW, TestResult, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

#[test]
fn child() -> TestResult {
    assessed::child()
}

#[test]
fn a_grounded_entailed_check_meets_the_gate() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("gate ship [satisfied] assessed pass met=1/1"),
        "{text}"
    );
    assert!(
        text.contains("  check: ready [entailed grounded] met"),
        "{text}"
    );
    assert!(
        text.contains("  check: rolled_back [refuted grounded] (not required)"),
        "{text}"
    );
    let (text, exit) = status(&w)?;
    golden("gate-status-assessed.txt", &text, exit)?;
    assert_eq!(state_of(&text, "ship"), "satisfied");
    let (lines, exit) = gate(
        &w,
        GateVerb::Lines {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        NOW + 3600,
    )?;
    assert_eq!(exit, 0, "{lines}");
    assert!(
        lines.contains("ship: satisfied assessed pass met=1/1 runs/"),
        "{lines}"
    );
    let mut resume = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    resume.cwd = w.code.clone();
    let (resumed, _) = w.run(&resume)?;
    assert!(
        resumed.contains("  check: ready [entailed grounded] met"),
        "{resumed}"
    );
    // The receipt keeps Weft's own basis beside Writfold's.
    let receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(&receipts(&w)?[0])?)?;
    let ready = &receipt["checks"][0];
    assert_eq!(
        (
            ready["name"].as_str(),
            ready["weft_basis"].as_str(),
            ready["basis"].as_str()
        ),
        (Some("ready"), Some("hypothetical"), Some("grounded"))
    );
    assert_eq!(
        receipt["weft"]["binary"]["digest"],
        receipt["weft"]["binary"]["pin"]
    );
    assert_eq!(
        receipt["assumptions"][1]["citations"][0]["standing"],
        "fresh"
    );
    Ok(())
}

#[test]
fn a_stale_citation_makes_the_same_check_hypothetical() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    unit(&w, NOW)?;
    // The cited gate goes stale: an in-scope file changes after its run.
    std::fs::write(w.code.join("src/ledger.rs"), "// edited\n")?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("gate ship [failed] assessed fail met=0/1"),
        "{text}"
    );
    assert!(text.contains("  check: ready [entailed hypothetical] not met; rests on ungrounded tests_pass !hypothetical"), "{text}");
    assert!(
        text.contains("  cite: tests_pass unit_green [ungrounded] stale"),
        "{text}"
    );
    let (text, exit) = status(&w)?;
    assert_eq!(
        (state_of(&text, "ship"), exit),
        ("failed".to_owned(), 2),
        "{text}"
    );
    // Nothing cited at all is ungrounded too: `unit` never ran here.
    let v = world(&[])?;
    define(
        &v,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    let (text, _) = assess(&v, "ship", Some(&double()), NOW + 60)?;
    assert!(
        text.contains("  cite: tests_pass unit_green [ungrounded] open"),
        "{text}"
    );
    Ok(())
}

#[test]
fn refuted_and_undetermined_checks_do_not_meet_the_gate() -> TestResult {
    let w = world(&[])?;
    let gates = [
        ("ship-back", "{gate}/step.weft", "rolled_back"),
        ("ship-prod", "{gate}/step.weft", "deployed"),
    ];
    define(&w, &sha(&double())?, &gates)?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship-back", Some(&double()), NOW + 60)?;
    assert!(
        exit == 2 && text.contains("  check: rolled_back [refuted grounded] not met"),
        "{text}"
    );
    let (text, exit) = assess(&w, "ship-prod", Some(&double()), NOW + 120)?;
    assert!(
        exit == 2 && text.contains("  check: deployed [undetermined grounded] not met"),
        "{text}"
    );
    let (text, _) = status(&w)?;
    assert_eq!(state_of(&text, "ship-back"), "failed", "{text}");
    assert_eq!(state_of(&text, "ship-prod"), "failed", "{text}");
    Ok(())
}

#[test]
fn edits_to_the_source_or_cited_evidence_make_the_receipt_stale() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    // A cited KB record changes state.
    let record = w.kb.join("ktlo/2026/hbr-99-flush-stall.md");
    let original = std::fs::read_to_string(&record)?;
    std::fs::write(
        &record,
        original.replace("status: done", "status: in-progress"),
    )?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        (state_of(&text, "ship"), exit),
        ("stale".to_owned(), 2),
        "{text}"
    );
    assert!(
        text.contains("  why: cited evidence moved: stall_done (⟦cite kind=record"),
        "{text}"
    );
    assert!(text.contains("was fresh, now changed"), "{text}");
    std::fs::write(&record, &original)?;
    assert_eq!(state_of(&status(&w)?.0, "ship"), "satisfied");
    // A cited gate goes stale.
    let ledger = w.code.join("src/ledger.rs");
    let code = std::fs::read_to_string(&ledger)?;
    std::fs::write(&ledger, "// edited\n")?;
    let (text, _) = status(&w)?;
    assert!(
        text.contains(
            "  why: cited evidence moved: unit_green (gate:unit) was satisfied, now stale"
        ),
        "{text}"
    );
    std::fs::write(&ledger, &code)?;
    assert_eq!(state_of(&status(&w)?.0, "ship"), "satisfied");
    // The Weft source changes.
    std::fs::write(
        w.kb.join(".writfold/gates/step.weft"),
        SOURCE.replace("Plan step", "Step"),
    )?;
    let (text, _) = status(&w)?;
    assert!(
        text.contains(
            "  why: the Weft source changed ({gate}/step.weft is not the assessed bytes)"
        ),
        "{text}"
    );
    assert_eq!(state_of(&text, "ship"), "stale");
    Ok(())
}

#[test]
fn an_attested_citation_grounds_and_is_named() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    // `unit` is a statement here, not a command.
    put(
        &w.kb.join(".writfold/gates/unit.toml"),
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n",
    )?;
    let say = GateVerb::Say(writfold_kb::gate::Say {
        kind: writfold_kb::gate::statement::Kind::Attested,
        id: "unit".into(),
        text: "ran the suite by hand".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    gate(&w, say, NOW)?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("  check: ready [entailed grounded] met; rests on attested tests_pass"),
        "{text}"
    );
    assert!(
        text.contains("  cite: tests_pass unit_green [attested] attested @runs/"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_source_submitted_in_the_run_is_read_from_the_run() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{run}/plan/step.weft", "ready")],
    )?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert!(
        exit == 3 && text.contains("submit the Weft file at {run}/plan/step.weft"),
        "{text}"
    );
    put(&w.kb.join(format!("runs/{RUN}/plan/step.weft")), SOURCE)?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 120)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

/// Against the real Weft binary: the grounded pass, then the same check
/// hypothetical once its cited gate is stale.
#[test]
#[ignore = "needs WRITFOLD_TEST_WEFT, a built weft binary"]
fn the_real_weft_binary_meets_and_refuses() -> TestResult {
    let weft = PathBuf::from(std::env::var("WRITFOLD_TEST_WEFT")?);
    let w = world(&[])?;
    define(&w, &sha(&weft)?, &[("ship", "{gate}/step.weft", "ready")])?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship", Some(&weft), NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("  check: ready [entailed grounded] met"),
        "{text}"
    );
    git(&w.code, &["status", "--short"])?;
    std::fs::write(w.code.join("src/ledger.rs"), "// edited\n")?;
    let (text, exit) = assess(&w, "ship", Some(&weft), NOW + 120)?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("  check: ready [entailed hypothetical] not met; rests on ungrounded tests_pass !hypothetical"), "{text}");
    Ok(())
}
