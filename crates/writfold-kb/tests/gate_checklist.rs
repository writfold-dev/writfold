//! Checklist gates over the fixture world. A gate of class `checklist`
//! declares items; `gate cover` records one row per judgement about one
//! item, `supported` with references Writfold grounds, or `unsupported`
//! or `not-applicable` with a reason. Each item stands on its newest row
//! by `seq`, re-grounded on every read; a covered checklist stands as
//! `attested`, never `satisfied`.
mod checklist;
mod support;

use checklist::{RUN, STALL, cover, define, gate, golden, item, rows, run_unit, standing, status};
use support::{TestResult, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::Outcome;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

fn refusal(result: writfold_kb::error::Result<writfold_kb::emit::Output>) -> String {
    match result {
        Ok(out) => format!("not refused: {}", out.text),
        Err(e) => e.to_string(),
    }
}

/// Cover both required items and the optional one.
fn cover_all(w: &support::World) -> TestResult {
    run_unit(w)?;
    cover(w, "errors", Outcome::Supported, &["gate:unit"])?;
    cover(w, "record", Outcome::Supported, &[STALL])?;
    cover(w, "perf", Outcome::NotApplicable, &[])?;
    Ok(())
}

#[test]
fn a_covered_checklist_stands_as_attested() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        (standing(&text, "review"), exit),
        ("open".into(), 2),
        "{text}"
    );
    assert!(
        text.contains("  next: writfold kb gate cover review errors supported REF… --by WHO"),
        "{text}"
    );
    run_unit(&w)?;
    let out = cover(&w, "errors", Outcome::Supported, &["gate:unit"])?;
    assert!(
        out.text
            .contains("gate review [open] checklist 1/2 required covered"),
        "{}",
        out.text
    );
    assert!(
        out.text
            .contains("next: writfold kb gate cover review record supported"),
        "{}",
        out.text
    );
    cover(&w, "record", Outcome::Supported, &[STALL])?;
    cover(&w, "perf", Outcome::NotApplicable, &[])?;
    let (text, exit) = status(&w)?;
    golden("gate-status-checklist.txt", &text, exit)?;
    assert_eq!(
        (standing(&text, "review"), exit),
        ("attested".into(), 0),
        "{text}"
    );
    assert!(!text.contains("[satisfied] checklist"), "{text}");
    let mut json = w.request(Verb::Gate(GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    }));
    json.json = true;
    json.now = Some(support::NOW + 3600);
    let (text, exit) = w.run(&json)?;
    golden("gate-status-checklist.json", &text, exit)?;
    let (lines, exit) = gate(
        &w,
        GateVerb::Lines {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    assert_eq!(exit, 0, "{lines}");
    assert!(lines.contains("1 of 2 satisfied, 1 attested"), "{lines}");
    assert!(
        lines.contains("review: attested checklist 2/2 required covered"),
        "{lines}"
    );
    assert!(
        lines.contains("   perf not-applicable by agent runs/"),
        "{lines}"
    );
    let mut resume = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    resume.cwd = w.code.clone();
    let (resumed, _) = w.run(&resume)?;
    assert!(
        resumed.contains("gate review [attested] checklist 2/2"),
        "{resumed}"
    );
    Ok(())
}

#[test]
fn supported_needs_evidence_and_the_others_a_reason() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    run_unit(&w)?;
    let why = refusal(cover(&w, "errors", Outcome::Supported, &[]));
    assert!(why.contains("needs at least one reference"), "{why}");
    let mut verb = checklist::cover_verb(&w.code, "perf", Outcome::NotApplicable, &[]);
    if let GateVerb::Cover(c) = &mut verb {
        c.reason = None;
    }
    let why = refusal(writfold_kb::run(&w.request(Verb::Gate(verb))));
    assert!(why.contains("needs --reason"), "{why}");
    let why = refusal(cover(&w, "perf", Outcome::Unsupported, &["gate:unit"]));
    assert!(why.contains("takes a reason, not references"), "{why}");
    let why = refusal(cover(&w, "speed", Outcome::Supported, &["gate:unit"]));
    assert!(why.contains("has no item speed"), "{why}");
    let why = refusal(cover(&w, "speed", Outcome::Supported, &["gate:unit"]));
    assert!(why.contains("errors, record, perf"), "{why}");
    assert!(rows(&w.kb)?.is_empty());
    Ok(())
}

#[test]
fn an_ungrounded_reference_or_a_waiver_is_refused_at_write() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    // `unit` has not run: the gate is open, so citing it grounds nothing.
    let why = refusal(cover(&w, "errors", Outcome::Supported, &["gate:unit"]));
    assert!(why.contains("is not grounded (open)"), "{why}");
    let why = refusal(cover(
        &w,
        "errors",
        Outcome::Supported,
        &["runs/nowhere.json"],
    ));
    assert!(why.contains("not grounded (unresolved)"), "{why}");
    // A waiver is never evidence, whether cited as the gate or as its file.
    let say = GateVerb::Say(Say {
        kind: Kind::Waived,
        id: "unit".into(),
        text: "not run on this host".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(say)))?;
    let why = refusal(cover(&w, "errors", Outcome::Supported, &["gate:unit"]));
    assert!(why.contains("not grounded (waived)"), "{why}");
    let waiver = std::fs::read_dir(w.kb.join(format!("runs/{RUN}/statements")))?
        .flatten()
        .map(|e| format!("runs/{RUN}/statements/{}", e.file_name().to_string_lossy()))
        .next()
        .ok_or("no waiver")?;
    let why = refusal(cover(&w, "errors", Outcome::Supported, &[&waiver]));
    assert!(why.contains("not-a-current-attestation"), "{why}");
    assert!(rows(&w.kb)?.is_empty());
    Ok(())
}

#[test]
fn an_in_scope_edit_turns_every_item_stale() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    cover_all(&w)?;
    std::fs::write(w.code.join("src/batch.rs"), "// edited\n")?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        (standing(&text, "review"), exit),
        ("stale".into(), 2),
        "{text}"
    );
    for id in ["errors", "record", "perf"] {
        assert_eq!(item(&text, id), "stale", "{text}");
    }
    assert!(
        text.contains("the content in scope changed since this row"),
        "{text}"
    );
    Ok(())
}

#[test]
fn moved_evidence_turns_only_its_items_stale_and_reverting_restores_them() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    cover_all(&w)?;
    let ledger = std::fs::read_to_string(w.code.join("src/ledger.rs"))?;
    // `unit`'s scope moves: `errors` cites it; `record` does not.
    std::fs::write(w.code.join("src/ledger.rs"), "// edited\n")?;
    let (text, exit) = status(&w)?;
    golden("gate-status-checklist-moved.txt", &text, exit)?;
    assert_eq!(standing(&text, "review"), "stale", "{text}");
    assert_eq!(
        (
            item(&text, "errors"),
            item(&text, "record"),
            item(&text, "perf")
        ),
        ("stale".into(), "supported".into(), "not-applicable".into()),
        "{text}"
    );
    assert!(
        text.contains("cited evidence moved: gate:unit was satisfied, now stale"),
        "{text}"
    );
    std::fs::write(w.code.join("src/ledger.rs"), ledger)?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        (standing(&text, "review"), exit),
        ("attested".into(), 0),
        "{text}"
    );
    assert_eq!(item(&text, "errors"), "supported", "{text}");
    Ok(())
}

#[test]
fn a_hand_edited_or_moved_row_reads_invalid() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    cover_all(&w)?;
    let files = rows(&w.kb)?;
    let perf = files
        .iter()
        .find(|p| p.to_string_lossy().contains("review.perf-"))
        .ok_or("no perf row")?;
    let text = std::fs::read_to_string(perf)?;
    std::fs::write(perf, text.replace("\"not-applicable\"", "\"supported\""))?;
    let (out, _) = status(&w)?;
    assert!(
        out.contains("[invalid] not written by writfold gate cover"),
        "{out}"
    );
    assert_eq!(item(&out, "perf"), "open", "{out}");
    std::fs::write(perf, &text)?;
    // The same bytes filed under another item's name count for nothing.
    let errors = files
        .iter()
        .find(|p| p.to_string_lossy().contains("review.errors-"))
        .ok_or("no errors row")?;
    std::fs::remove_file(errors)?;
    let copied = perf.with_file_name("review.errors-20261111T000000Z-abcdef.json");
    std::fs::write(&copied, &text)?;
    let (out, exit) = status(&w)?;
    assert!(
        out.contains("its run, gate or item does not match where it is filed"),
        "{out}"
    );
    assert_eq!((item(&out, "errors"), exit), ("open".into(), 2), "{out}");
    assert_eq!(standing(&out, "review"), "open", "{out}");
    Ok(())
}

#[test]
fn the_newest_row_by_seq_decides_the_item() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    cover_all(&w)?;
    cover(&w, "errors", Outcome::Unsupported, &[])?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        (item(&text, "errors"), exit),
        ("unsupported".into(), 2),
        "{text}"
    );
    assert_eq!(standing(&text, "review"), "open", "{text}");
    let newest = rows(&w.kb)?
        .into_iter()
        .filter(|p| p.to_string_lossy().contains("review.errors-"))
        .map(|p| -> TestResult<u64> {
            let v: serde_json::Value = serde_json::from_slice(&std::fs::read(p)?)?;
            Ok(v["seq"].as_u64().unwrap_or(0))
        })
        .collect::<TestResult<Vec<u64>>>()?;
    let mut seqs = newest;
    seqs.sort_unstable();
    assert_eq!(seqs, vec![1, 2]);
    Ok(())
}

#[test]
fn attest_and_run_are_refused_and_waive_sets_the_gate_aside() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    let run = GateVerb::Run {
        id: "review".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let why = refusal(writfold_kb::run(&w.request(Verb::Gate(run))));
    assert!(
        why.contains("is a checklist") && why.contains("gate cover review"),
        "{why}"
    );
    let say = |kind: Kind, text: &str| {
        GateVerb::Say(Say {
            kind,
            id: "review".into(),
            text: text.into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        })
    };
    let why = refusal(writfold_kb::run(
        &w.request(Verb::Gate(say(Kind::Attested, "read it"))),
    ));
    assert!(
        why.contains("is a checklist") && why.contains("gate cover review"),
        "{why}"
    );
    let unit = checklist::cover_verb(&w.code, "errors", Outcome::Supported, &["gate:review"]);
    let why = refusal(writfold_kb::run(&w.request(Verb::Gate(unit))));
    assert!(why.contains("not grounded (no-such-gate)"), "{why}");
    let mut on_unit = checklist::cover_verb(&w.code, "errors", Outcome::NotApplicable, &[]);
    if let GateVerb::Cover(c) = &mut on_unit {
        c.gate = "unit".into();
    }
    let why = refusal(writfold_kb::run(&w.request(Verb::Gate(on_unit))));
    assert!(why.contains("is a mechanical gate"), "{why}");
    writfold_kb::run(&w.request(Verb::Gate(say(Kind::Waived, "docs-only change"))))?;
    let (text, _) = status(&w)?;
    assert_eq!(standing(&text, "review"), "waived", "{text}");
    assert!(
        text.contains("  waived: docs-only change (by owner,"),
        "{text}"
    );
    Ok(())
}

/// A plan step that owes a checklist cannot advance until it is covered;
/// its done event names a cover row as what met the gate.
#[test]
fn a_plan_step_owing_a_checklist_advances_once_covered() -> TestResult {
    use writfold_kb::plan::PlanVerb;
    let w = world(&[])?;
    define(&w.kb)?;
    checklist::put(
        &w.kb.join(".writfold/plans/change.toml"),
        "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"review: review\"]\n",
    )?;
    let plan = |verb: &str| {
        let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
        let verb = match verb {
            "set" => PlanVerb::Set {
                template: Some("change".into()),
                from: None,
                run,
                repo,
            },
            _ => PlanVerb::Advance { run, repo },
        };
        w.run(&w.request(Verb::Plan(verb)))
    };
    assert_eq!(plan("set")?.1, 0);
    let (text, exit) = plan("advance")?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("owes: review [open]"), "{text}");
    assert!(
        text.contains("next: writfold kb gate cover review errors supported REF… --by WHO"),
        "{text}"
    );
    cover_all(&w)?;
    let (text, exit) = plan("advance")?;
    assert_eq!(exit, 0, "{text}");
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    let done = std::fs::read_dir(&dir)?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("done-"))
        .ok_or("no done event")?;
    let event: serde_json::Value = serde_json::from_slice(&std::fs::read(done.path())?)?;
    let met = event.to_string();
    assert!(met.contains("\"standing\":\"attested\""), "{met}");
    assert!(met.contains(&format!("runs/{RUN}/covers/review.")), "{met}");
    Ok(())
}

#[test]
fn verify_answers_unverifiable_for_a_cover_row() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    cover_all(&w)?;
    let first = rows(&w.kb)?.into_iter().next().ok_or("no row")?;
    let rel = first.strip_prefix(&w.kb)?.to_string_lossy().into_owned();
    let (text, exit) = gate(
        &w,
        GateVerb::Verify {
            receipt: rel,
            repo: Some(w.code.clone()),
        },
    )?;
    assert!(
        text.contains("[unverifiable] a cover row is a judgement"),
        "{text}"
    );
    assert_eq!(exit, 2, "{text}");
    Ok(())
}
