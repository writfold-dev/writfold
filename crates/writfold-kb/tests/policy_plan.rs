//! A run's policy over its plan (policy spec 2.4 to 2.6): `unskippable`
//! required steps, the pin written with every splice and the drift read
//! against it, and one word for how a plan ended.
mod checklist;
mod support;

use checklist::{RUN, put};
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;
use writfold_kb::write::Write;
use writfold_kb::write::status::SetStatus;

const REQUIRED: &str =
    "schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\nunskippable = [\"research\"]\n";
const NOTE: &str = "schema = \"writfold-gate/1\"\nid = \"note\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\ninstructions = \"note.md\"\n";
const RESEARCH: &str =
    "schema = \"writfold-plan/1\"\nname = \"research\"\nsteps = [\"research: note\"]\n";
const MINE: &str =
    "schema = \"writfold-plan/1\"\nname = \"mine\"\nsteps = [\"implement:\", \"review:\"]\n";

fn set_up(required: &str) -> TestResult<World> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/note.toml"), NOTE)?;
    put(
        &w.kb.join(".writfold/gates/note.md"),
        "# Read the code first\n",
    )?;
    put(&w.kb.join(".writfold/plans/research.toml"), RESEARCH)?;
    put(&w.kb.join(".writfold/plans/mine.toml"), MINE)?;
    put(&w.kb.join(".writfold/plans/required.toml"), required)?;
    Ok(w)
}

fn plan(w: &World, verb: &str) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
    let verb = match verb {
        "set" => PlanVerb::Set {
            template: Some("mine".into()),
            from: None,
            run,
            repo,
        },
        "status" => PlanVerb::Status {
            gates: false,
            run,
            repo,
        },
        "skip" => PlanVerb::Skip {
            reason: "no time".into(),
            by: "owner".into(),
            run,
            repo,
        },
        _ => PlanVerb::Advance { run, repo },
    };
    match writfold_kb::run(&w.request(Verb::Plan(verb))) {
        Ok(out) => Ok((out.text, out.exit)),
        Err(e) => Ok((e.to_string(), e.exit_code())),
    }
}

fn attest(w: &World) -> TestResult<u8> {
    attest_text(w).map(|(_, exit)| exit)
}

fn attest_text(w: &World) -> TestResult<(String, u8)> {
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "note".into(),
        text: "read".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let out = writfold_kb::run(&w.request(Verb::Gate(say)))?;
    Ok((out.text, out.exit))
}

#[test]
fn an_unskippable_step_cannot_be_skipped_and_the_next_one_can() -> TestResult {
    let w = set_up(REQUIRED)?;
    let (text, exit) = plan(&w, "set")?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("policy/pin-1-"), "the pin is written: {text}");
    let pins = w.kb.join(format!("runs/{RUN}/policy"));
    let pin = (std::fs::read_dir(&pins)?.flatten().map(|e| e.path()).next()).ok_or("no pin")?;
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&pin)?)?;
    assert_eq!(v["required"]["unskippable"][0], "research");
    assert_eq!(v["gates"][0]["gate"], "note");
    let (text, exit) = plan(&w, "skip")?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("!locked"), "{text}");
    assert!(
        text.contains("not skipped: research#1/research is a required step this run cannot skip (unskippable = [\"research\"] @.writfold/plans/required.toml:3, pinned @runs/"),
        "{text}"
    );
    assert!(
        !w.kb
            .join(format!("runs/{RUN}/plan/skip-research%231%2Fresearch.json"))
            .exists()
    );
    // Weakening the live file does not unlock a run already pinned.
    put(
        &w.kb.join(".writfold/plans/required.toml"),
        "schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\n",
    )?;
    assert_eq!(plan(&w, "skip")?.1, 3);
    assert_eq!(attest(&w)?, 0);
    // The weakened file is itself a policy move: exit 2, nothing refused.
    assert_eq!(plan(&w, "advance")?.1, 2);
    let (text, exit) = plan(&w, "skip")?;
    assert!(exit == 2 && text.contains("(skipped, not done)"), "{text}");
    Ok(())
}

#[test]
fn a_hand_written_skip_of_a_locked_step_counts_for_nothing() -> TestResult {
    let w = set_up(REQUIRED)?;
    plan(&w, "set")?;
    // A skip written by hand, as an earlier binary would have written it.
    let other = set_up("schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\n")?;
    plan(&other, "set")?;
    assert_eq!(plan(&other, "skip")?.1, 0);
    let dir = other.kb.join(format!("runs/{RUN}/plan"));
    let skip = std::fs::read_dir(&dir)?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("skip-"))
        .ok_or("no skip")?;
    std::fs::copy(
        skip.path(),
        w.kb.join(format!("runs/{RUN}/plan")).join(skip.file_name()),
    )?;
    let (text, _) = plan(&w, "status")?;
    assert!(
        text.contains("step research#1/research [current]"),
        "{text}"
    );
    assert!(
        text.contains("a skip of a step this run cannot skip"),
        "{text}"
    );
    // The other run had no lock: it skipped, and its plan says so.
    let (text, _) = plan(&other, "status")?;
    assert!(
        text.contains("step research#1/research [skipped]"),
        "{text}"
    );
    Ok(())
}

#[test]
fn an_instruction_edit_moves_the_policy_and_restoring_it_clears_open_steps() -> TestResult {
    let w = set_up(REQUIRED)?;
    plan(&w, "set")?;
    let (text, exit) = plan(&w, "status")?;
    assert!(exit == 0 && text.contains("policy=same"), "{text}");
    put(
        &w.kb.join(".writfold/gates/note.md"),
        "# Read the code and the tests first\n",
    )?;
    let (text, exit) = plan(&w, "status")?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("!policy-moved") && text.contains("policy=moved"),
        "{text}"
    );
    assert!(text.contains("why: note is now @"), "{text}");
    assert!(
        text.contains("policy moved since this run pinned it: 1 gate (note)"),
        "{text}"
    );
    put(
        &w.kb.join(".writfold/gates/note.md"),
        "# Read the code first\n",
    )?;
    let (text, _) = plan(&w, "status")?;
    assert!(
        text.contains("policy=same") && !text.contains("!policy-moved"),
        "{text}"
    );
    // Done under an edited definition and put back: still moved.
    put(&w.kb.join(".writfold/gates/note.md"), "# Anything goes\n")?;
    let (text, exit) = attest_text(&w)?;
    assert_eq!(exit, 0, "{text}");
    // Done under the edited definition, while the policy is moved: exit 2.
    assert_eq!(plan(&w, "advance")?.1, 2);
    put(
        &w.kb.join(".writfold/gates/note.md"),
        "# Read the code first\n",
    )?;
    let (text, _) = plan(&w, "status")?;
    assert!(
        text.contains("policy=moved") && text.contains("why: met under note@"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_plan_says_how_it_ended() -> TestResult {
    let w = set_up("schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\n")?;
    plan(&w, "set")?;
    let (text, _) = plan(&w, "status")?;
    assert!(
        text.contains("plan incomplete") && text.contains("summary=incomplete"),
        "{text}"
    );
    assert_eq!(attest(&w)?, 0);
    for _ in 0..2 {
        assert_eq!(plan(&w, "advance")?.1, 0);
    }
    let (text, _) = plan(&w, "skip")?;
    assert!(text.contains("summary=complete-with-exceptions"), "{text}");
    assert!(
        text.contains("plan complete-with-exceptions 3 steps: 1 passed, 1 attested, 0 waived, 1 skipped; policy=same"),
        "{text}"
    );
    assert!(!text.contains("summary=complete "), "{text}");
    Ok(())
}

#[test]
fn closing_a_run_writes_its_note_and_says_the_plan_is_incomplete() -> TestResult {
    let w = set_up("schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\n")?;
    plan(&w, "set")?;
    let close = SetStatus {
        target: RUN.into(),
        state: "closed".into(),
        note: Some("abandoned, superseded by HBR-140".into()),
        ..SetStatus::default()
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::SetStatus(close))))?;
    assert!(
        out.text
            .contains("closed with the plan incomplete: step 1/3 open"),
        "{}",
        out.text
    );
    let sidecar = std::fs::read_to_string(w.kb.join(format!("runs/{RUN}/writfold-run.json")))?;
    assert!(
        sidecar.contains("\"closed_note\": \"abandoned, superseded by HBR-140\""),
        "{sidecar}"
    );
    let (text, _) = plan(&w, "status")?;
    assert!(text.contains("summary=closed-incomplete"), "{text}");
    Ok(())
}
