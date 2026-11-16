//! Empty is not met: a `gates.met` rule over a run that owes no gate
//! refuses, `gate status` and `gate lines` say that nothing is owed, and a
//! statement whose gate's scope holds the run's own files says it made
//! itself stale.
#[path = "guard_status/helpers.rs"]
#[allow(dead_code)]
mod helpers;
mod support;

use helpers::*;
use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;

fn reason(w: &World) -> TestResult<String> {
    let out = hook(w, false, "s-empty");
    let value: Value = serde_json::from_str(&out.stdout)?;
    let text = value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?;
    Ok(text.to_owned())
}

fn gate_answer(w: &World, verb: GateVerb, json: bool) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.cwd = w.code.clone();
    request.json = json;
    let output = writfold_kb::run(&request)?;
    Ok((output.text, output.exit))
}

fn plan(w: &World, steps: &str) -> TestResult {
    put(
        &w.kb.join(".writfold/plans/change.toml"),
        &format!("schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [{steps}]\n"),
    )?;
    let set = PlanVerb::Set {
        template: Some("change".into()),
        from: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    assert_eq!(writfold_kb::run(&w.request(Verb::Plan(set)))?.exit, 0);
    Ok(())
}

#[test]
fn no_gate_defined_is_not_met() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    parity(&w, "deny")?;
    let text = reason(&w)?;
    assert!(
        text.contains("no gate is owed here: none is defined for demo-service; gates.met is not met by an empty set; next: define .writfold/gates/<id>.toml, or leave this repository out of the rule with repos"),
        "{text}"
    );
    // A repos exclusion is the way out.
    rule(
        &w.kb,
        "commit",
        &COMMIT.replace("require =", "repos = [\"*\", \"!demo-service\"]\nrequire ="),
    )?;
    assert!(hook(&w, false, "s-empty").stdout.is_empty());
    Ok(())
}

#[test]
fn a_plan_naming_no_gate_is_not_met() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    definition(&w, "unit", UNIT)?;
    plan(&w, "\"implement:\"")?;
    parity(&w, "deny")?;
    assert!(reason(&w)?.contains("no gate is owed here: the run's plan names none;"));
    let status = GateVerb::Status {
        run: None,
        repo: None,
    };
    let (text, exit) = gate_answer(&w, status, false)?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("# note: no gate is owed: the run's plan names none (1 defined)"),
        "{text}"
    );
    assert!(text.contains("answer=unknown reason=no-gates"), "{text}");
    Ok(())
}

#[test]
fn one_gate_met_still_passes() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    definition(&w, "unit", UNIT)?;
    parity(&w, "deny")?;
    assert_eq!(check(&w, "unit")?, 0);
    parity(&w, "pass")?;
    Ok(())
}

#[test]
fn gate_lines_with_nothing_owed_say_so() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    let lines = GateVerb::Lines {
        run: None,
        repo: None,
    };
    let (text, exit) = gate_answer(&w, lines, false)?;
    assert_eq!(exit, 2);
    assert!(
        text.lines()
            .next()
            .is_some_and(|l| l.ends_with(": no gate is owed; nothing was checked")),
        "{text}"
    );
    assert!(!text.contains("0 of 0"), "{text}");
    Ok(())
}

fn waive_kb(w: &World) -> TestResult<String> {
    let verb = GateVerb::Say(Say {
        kind: Kind::Waived,
        id: "kb-docs".into(),
        text: "docs only".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.kb.clone()),
    });
    let output = writfold_kb::run(&w.request(Verb::Gate(verb)))?;
    assert_eq!(output.exit, 0, "{}", output.text);
    Ok(output.text)
}

#[test]
fn a_waiver_inside_its_own_scope_says_so() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    let def = "schema = \"writfold-gate/1\"\nid = \"kb-docs\"\nversion = 1\nclass = \"attested\"\nscope = [\"**\"]\n";
    definition(&w, "kb-docs", def)?;
    let text = waive_kb(&w)?;
    assert!(text.contains("# note: this gate's scope includes the run's own files, so the statement changed the content it is about; exclude runs/** from the scope"), "{text}");
    assert!(!text.contains("[waived]"), "{text}");
    definition(
        &w,
        "kb-docs",
        &def.replace("[\"**\"]", "[\"**\", \"!runs/**\"]"),
    )?;
    let text = waive_kb(&w)?;
    assert!(text.contains("[waived]"), "{text}");
    assert!(!text.contains("exclude runs/**"), "{text}");
    Ok(())
}
