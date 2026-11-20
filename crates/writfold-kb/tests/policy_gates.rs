//! A gate's policy (policy spec 2.2, 2.3): `accept` narrows which
//! standings meet a gate, and a checklist's `not_applicable` and
//! `none_found` say which reasoned outcomes cover an item. Every judging
//! site reads met through `writfold_core::policy::met`; a waiver or a row
//! the policy does not accept is shown as itself, flagged `!not-accepted`,
//! and counts for nothing.
mod checklist;
mod support;

use checklist::{RUN, STALL, cover, gate, item, put, run_unit, standing, status};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::Outcome;
use writfold_kb::gate::statement::{self, Kind, Statement};
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::write::Write;
use writfold_kb::write::observe::Observe;

const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"false\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";
const RESEARCH: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"checklist\"\nscope = [\"src/batch.rs\"]\nitems = [\"errors: errors are handled\", \"record: the stall ticket is closed\", \"perf: no new hot-path allocation\"]\nnot_applicable = [\"perf\"]\nnone_found = [\"errors\"]\naccept = [\"attested\"]\n";

fn refusal(result: writfold_kb::error::Result<writfold_kb::emit::Output>) -> String {
    match result {
        Ok(out) => format!("not refused: {}", out.text),
        Err(e) => format!("{e} (exit {})", e.exit_code()),
    }
}

fn waive(w: &World, id: &str) -> writfold_kb::error::Result<writfold_kb::emit::Output> {
    let say = GateVerb::Say(Say {
        kind: Kind::Waived,
        id: id.into(),
        text: "flaky here".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(say)))
}

#[test]
fn a_gate_that_accepts_only_satisfied_refuses_a_waiver() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    // Without accept, a waiver meets the gate, as before.
    assert_eq!(waive(&w, "unit")?.exit, 0);
    let (text, exit) = status(&w)?;
    assert!(exit == 0 && standing(&text, "unit") == "waived", "{text}");
    let strict = format!("{UNIT}accept = [\"satisfied\"]\n");
    put(&w.kb.join(".writfold/gates/unit.toml"), &strict)?;
    let why = refusal(waive(&w, "unit"));
    assert!(
        why.contains("gate unit does not accept a waiver (accept = [\"satisfied\"] @.writfold/gates/unit.toml:8); meet it: writfold kb gate run unit")
            && why.contains("exit 3"),
        "{why}"
    );
    // A waiver written by hand under the strict definition is shown and
    // counts for nothing.
    let def = writfold_kb::gate::def::find_repository(&w.kb, Some(&w.code), "unit")?;
    let content = content_digest(&w)?;
    let s = Statement {
        kind: Kind::Waived,
        obligation: "unit".into(),
        run: RUN.into(),
        subject: "demo-service".into(),
        commit: None,
        content,
        definition: def.digest.clone(),
        text: "flaky here".into(),
        by: "owner".into(),
        recorded: "2026-11-11T00:30:00Z".into(),
    };
    let file =
        w.kb.join(format!("runs/{RUN}/statements/unit-by-hand.json"));
    put(&file, &String::from_utf8(statement::bytes(&s))?)?;
    let (text, exit) = status(&w)?;
    assert!(exit == 2, "{text}");
    assert!(text.contains("!waived !not-accepted"), "{text}");
    assert!(
        text.contains("why: this gate accepts only: satisfied (@.writfold/gates/unit.toml:8); the waiver is recorded and does not meet it"),
        "{text}"
    );
    Ok(())
}

/// The scope digest of `unit` now, from a receipt the gate records.
fn content_digest(w: &World) -> TestResult<String> {
    let (_, exit) = gate(
        w,
        GateVerb::Run {
            id: "unit".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    assert_eq!(exit, 2);
    let dir = w.kb.join(format!("runs/{RUN}/gates"));
    let newest = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .max()
        .ok_or("no receipt")?;
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(newest)?)?;
    Ok(v["subject"]["scope"].as_str().ok_or("scope")?.to_owned())
}

#[test]
fn a_definition_without_policy_keys_keeps_its_digest() -> TestResult {
    let dir = std::env::temp_dir().join(format!("writfold-digest-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let def = writfold_kb::gate::def_check::check(
        UNIT,
        "unit",
        writfold_kb::gate::def::Origin::Kb,
        &dir,
        ".writfold/gates/unit.toml",
    )
    .map_err(|r| writfold_kb::defs::why(&r))?;
    // The digest this definition had before `accept` existed.
    assert_eq!(
        def.digest,
        "sha256:298321c7c8d1dc1668951954b78604d2ec5cfcf7816804696d15b84d0943bea2"
    );
    let with = format!("{UNIT}accept = [\"satisfied\"]\n");
    let strict = writfold_kb::gate::def_check::check(
        &with,
        "unit",
        writfold_kb::gate::def::Origin::Kb,
        &dir,
        ".writfold/gates/unit.toml",
    )
    .map_err(|r| writfold_kb::defs::why(&r))?;
    assert_ne!(strict.digest, def.digest);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn observe(w: &World, status: &str) -> TestResult<String> {
    let o = Observe {
        source: "probe".into(),
        locator: "find 'ledger batch'".into(),
        status: status.into(),
        revision: None,
        item: Some("review.errors".into()),
        note: None,
        cite: None,
        searched: None,
        run: Some(RUN.into()),
        ..Observe::default()
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::Observe(o))))?;
    let path = (out.text.lines())
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .ok_or("no path")?;
    Ok(path.to_owned())
}

fn cover_with_reason(
    w: &World,
    item: &str,
    outcome: Outcome,
    refs: &[&str],
) -> writfold_kb::error::Result<writfold_kb::emit::Output> {
    let verb = GateVerb::Cover(writfold_kb::gate::checklist::Cover {
        gate: "review".into(),
        item: item.into(),
        outcome,
        references: refs.iter().map(|r| (*r).to_owned()).collect(),
        reason: Some("searched src/ and tests/ for ledger batching; KB find also empty".into()),
        note: None,
        by: "agent".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(verb)))
}

#[test]
fn a_checklist_takes_only_the_outcomes_its_policy_names() -> TestResult {
    let w = world(&[])?;
    checklist::define(&w.kb)?;
    put(&w.kb.join(".writfold/gates/review.toml"), RESEARCH)?;
    let probe = "schema = \"writfold-source/1\"\nkind = \"probe\"\nfreshness = \"ttl\"\ndefault_ttl = \"1d\"\n";
    put(&w.kb.join(".writfold/sources/probe.toml"), probe)?;
    run_unit(&w)?;
    // not-applicable only for perf; none-found only for errors.
    let why = refusal(cover_with_reason(&w, "record", Outcome::NotApplicable, &[]));
    assert!(
        why.contains("review.record cannot be covered not-applicable (not_applicable = [\"perf\"] @.writfold/gates/review.toml:7); cover it supported with evidence"),
        "{why}"
    );
    let why = refusal(cover_with_reason(&w, "record", Outcome::NoneFound, &[]));
    assert!(
        why.contains("gate cover … none-found cites at least one look"),
        "{why}"
    );
    let ok_look = observe(&w, "ok")?;
    let why = refusal(cover_with_reason(
        &w,
        "errors",
        Outcome::NoneFound,
        &[&ok_look],
    ));
    assert!(
        why.contains("none-found needs a look that found nothing")
            && why.contains("a look with status"),
        "{why}"
    );
    let empty = observe(&w, "no-data")?;
    let why = refusal(cover_with_reason(
        &w,
        "record",
        Outcome::NoneFound,
        &[&empty],
    ));
    assert!(
        why.contains("review.record cannot be covered none-found"),
        "{why}"
    );
    let out = cover_with_reason(&w, "errors", Outcome::NoneFound, &[&empty])?;
    assert!(
        out.text
            .contains("item: errors [none-found] 1 look found nothing"),
        "{}",
        out.text
    );
    cover(&w, "record", Outcome::Supported, &[STALL])?;
    cover_with_reason(&w, "perf", Outcome::NotApplicable, &[])?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        (standing(&text, "review"), exit),
        ("attested".into(), 0),
        "{text}"
    );
    assert_eq!(item(&text, "errors"), "none-found", "{text}");
    assert!(!text.contains("[supported] 1 look"), "{text}");
    // The checklist takes no waiver.
    let why = refusal(waive(&w, "review"));
    assert!(
        why.contains("gate review does not accept a waiver"),
        "{why}"
    );
    let _ = NOW;
    Ok(())
}
