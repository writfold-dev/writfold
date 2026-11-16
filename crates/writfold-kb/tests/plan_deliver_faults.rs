//! Refusals around a delivery step, each exit 3 with nothing written: a
//! step naming no delivery mode (directly, through `use=`, through a
//! required end template), a mode file that does not load, a mode that
//! declares nothing, and a gate verb given `deliver:<mode>`. Then what the
//! gate surfaces say: `gate status` neither lists nor counts a delivery,
//! `gates.met` ignores it (a commit is not held by its PR comment), and
//! `step.met` judges it.
#[path = "guard_status/helpers.rs"]
#[allow(dead_code)]
mod guard;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{CLEAN, RUN, events, fixture, prepare, put, set};
use serde_json::Value;
use support::{NOW, TestResult, World};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

fn refused(w: &World, template: &str) -> TestResult<String> {
    let out = set(w, template);
    let why = out.err().map(|e| e.to_string()).unwrap_or_default();
    let why = why.strip_prefix("refused: ").unwrap_or(&why).to_owned();
    assert!(events(w).is_empty(), "{template}: wrote {:?}", events(w));
    Ok(why)
}

fn plan(w: &World, name: &str, steps: &str) -> TestResult {
    let text = format!("schema = \"writfold-plan/1\"\nname = \"{name}\"\nsteps = [{steps}]\n");
    put(w, &format!(".writfold/plans/{name}.toml"), &text)
}

#[test]
fn a_delivery_no_mode_can_meet_is_refused_when_spliced() -> TestResult {
    let w = fixture()?;
    plan(&w, "nope", "\"x: deliver:nope\"")?;
    let why = refused(&w, "nope")?;
    assert_eq!(
        why,
        "plan nope names deliver:nope, which is not a delivery mode: add .writfold/deliveries/nope.toml to the KB root, or use a built-in mode: pr-comment, slack-summary"
    );
    plan(&w, "wrap", "\"a: use=nope\"")?;
    assert!(refused(&w, "wrap")?.starts_with("plan nope names deliver:nope,"));
    plan(&w, "empty", "\"x: deliver:empty\"")?;
    assert_eq!(
        refused(&w, "empty")?,
        "plan empty names deliver:empty, whose mode declares nothing to check; a delivery is not met by an empty set: add checks, gates or attest to .writfold/deliveries/empty.toml"
    );
    put(
        &w,
        ".writfold/deliveries/broken.toml",
        "schema = \"writfold-delivery-mode/2\"\nmode = \"broken\"\n",
    )?;
    plan(&w, "broken", "\"x: deliver:broken\"")?;
    assert_eq!(
        refused(&w, "broken")?,
        "deliver:broken: .writfold/deliveries/broken.toml: schema must be writfold-delivery-mode/1"
    );
    // A required end template is checked like the plan's own.
    plan(&w, "closing", "\"close: deliver:nope\"")?;
    let required = "schema = \"writfold-plan-required/1\"\nend = [\"closing\"]\n";
    put(&w, ".writfold/plans/required.toml", required)?;
    assert!(refused(&w, "change")?.starts_with("plan closing names deliver:nope,"));
    Ok(())
}

#[test]
fn a_gate_verb_refuses_a_delivery() -> TestResult {
    let w = fixture()?;
    let word = "deliver:pr-comment".to_owned();
    let say = |kind| Say {
        kind,
        id: word.clone(),
        text: "posted".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let run = GateVerb::Run {
        id: word.clone(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    for verb in [
        GateVerb::Say(say(Kind::Waived)),
        GateVerb::Say(say(Kind::Attested)),
        run,
    ] {
        let out = writfold_kb::run(&w.request(Verb::Gate(verb)));
        assert!(
            out.is_err_and(|e| e.to_string()
                == "refused: deliver:pr-comment is a delivery, not a gate: use writfold kb deliver check|attest|waive"),
            "a gate verb took a delivery"
        );
    }
    Ok(())
}

fn gate_status(w: &World) -> TestResult<(String, u8)> {
    let verb = GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let out = writfold_kb::run(&w.request(Verb::Gate(verb)))?;
    Ok((out.text, out.exit))
}

/// The guard's text: a deny's reason, or a Stop nudge's context (once per
/// session, so each call names its own).
fn reason(w: &World, stop: bool, session: &str) -> TestResult<String> {
    let out = guard::hook(w, stop, session);
    if out.stdout.is_empty() {
        return Ok(String::new());
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let text = match stop {
        true => value["hookSpecificOutput"]["additionalContext"].as_str(),
        false => value["hookSpecificOutput"]["permissionDecisionReason"].as_str(),
    };
    Ok(text.unwrap_or_default().to_owned())
}

#[test]
fn gates_met_ignores_a_delivery_and_step_met_judges_it() -> TestResult {
    let w = fixture()?;
    guard::rule(&w.kb, "commit", guard::COMMIT)?;
    guard::rule(&w.kb, "finish", guard::STOP)?;
    guard::current(&w)?;
    set(&w, "note")?;
    let (text, exit) = gate_status(&w)?;
    assert!(
        exit == 2
            && !text.lines().any(|l| l.starts_with("gate deliver:"))
            && text.contains("# note: no gate is owed here: the run's plan names only deliveries (deliver:note), judged at their steps: writfold kb plan status")
            && text.contains("reason=no-gates"),
        "{text}"
    );
    let commit = reason(&w, false, "s-commit")?;
    assert!(
        commit.contains("owes: no gate is owed here: the run's plan names only deliveries; gates.met is not met by an empty set; next: require step.met for the delivery step, name a gate in a plan step, or leave this repository out of the rule with repos ("),
        "{commit}"
    );
    let stop = reason(&w, true, "s-open")?;
    assert!(
        stop.contains("step say owes deliver:note [open]; next: writfold kb deliver prepare --mode note --from FILE"),
        "{stop}"
    );
    let id = prepare(&w, "note", CLEAN, NOW + 60)?;
    helpers::say(&w, Kind::Attested, &id, "tone.reviewed", NOW + 120)?;
    let stop = reason(&w, true, "s-ready")?;
    assert_eq!(stop, "", "step.met passes once the delivery is ready");
    Ok(())
}

#[test]
fn a_met_gate_beside_an_open_delivery_lets_the_commit_through() -> TestResult {
    let w = fixture()?;
    guard::rule(&w.kb, "commit", guard::COMMIT)?;
    guard::current(&w)?;
    guard::definition(&w, "unit", guard::UNIT)?;
    plan(&w, "both", "\"implement: unit deliver:pr-comment\"")?;
    set(&w, "both")?;
    assert_eq!(guard::check(&w, "unit")?, 0);
    let (text, exit) = gate_status(&w)?;
    assert!(
        exit == 0
            && text.contains("gate unit [satisfied]")
            && text.contains("# note: the plan also owes deliver:pr-comment at both#1/implement; a delivery is judged at its step: writfold kb plan status"),
        "{text}"
    );
    let commit = reason(&w, false, "s-commit")?;
    assert_eq!(commit, "", "gates.met counted the delivery");
    Ok(())
}
