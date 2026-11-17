//! `gate lines` agrees with the plan's word (policy spec 2.6): its header
//! ends with `; plan <word>`, a gate owed only by skipped steps is listed
//! `skipped` and never counts, and it exits 0 only when the plan reads
//! `complete` or `complete-with-exceptions` with every other gate met.
mod checklist;
mod support;

use checklist::{RUN, put};
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;

const NOTE: &str = "schema = \"writfold-gate/1\"\nid = \"note\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\ninstructions = \"note.md\"\n";
const FINDINGS: &str = "schema = \"writfold-gate/1\"\nid = \"findings\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\ninstructions = \"findings.md\"\n";
const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"two\"\nsteps = [\"research: note\", \"review: findings\"]\n";

fn set_up() -> TestResult<World> {
    set_up_with(None)
}

fn set_up_with(required: Option<&str>) -> TestResult<World> {
    let w = world(&[])?;
    if let Some(required) = required {
        put(&w.kb.join(".writfold/plans/required.toml"), required)?;
    }
    let gates = w.kb.join(".writfold/gates");
    put(&gates.join("note.toml"), NOTE)?;
    put(&gates.join("note.md"), "# Read the code first\n")?;
    put(&gates.join("findings.toml"), FINDINGS)?;
    put(&gates.join("findings.md"), "# List what you found\n")?;
    put(&w.kb.join(".writfold/plans/two.toml"), PLAN)?;
    let set = PlanVerb::Set {
        template: Some("two".into()),
        from: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let out = writfold_kb::run(&w.request(Verb::Plan(set)))?;
    assert_eq!(out.exit, 0, "{}", out.text);
    Ok(w)
}

fn attest(w: &World, id: &str) -> TestResult {
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: id.into(),
        text: "done".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let out = writfold_kb::run(&w.request(Verb::Gate(say)))?;
    assert_eq!(out.exit, 0, "{}", out.text);
    Ok(())
}

fn plan(w: &World, skip: bool) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
    let verb = match skip {
        true => PlanVerb::Skip {
            reason: "no time".into(),
            by: "owner".into(),
            run,
            repo,
        },
        false => PlanVerb::Advance { run, repo },
    };
    let out = writfold_kb::run(&w.request(Verb::Plan(verb)))?;
    Ok((out.text, out.exit))
}

fn lines(w: &World) -> TestResult<(String, u8)> {
    let verb = GateVerb::Lines {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let out = writfold_kb::run(&w.request(Verb::Gate(verb)))?;
    Ok((out.text, out.exit))
}

fn header(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

#[test]
fn a_skipped_steps_gate_is_listed_skipped_and_the_plan_word_decides() -> TestResult {
    let w = set_up()?;
    attest(&w, "note")?;
    assert_eq!(plan(&w, false)?.1, 0);
    let (text, exit) = plan(&w, true)?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = lines(&w)?;
    assert!(
        header(&text)
            .ends_with("0 of 2 satisfied, 1 attested, 1 skipped; plan complete-with-exceptions"),
        "{text}"
    );
    assert!(text.lines().any(|l| l == "findings: skipped"), "{text}");
    assert!(!text.contains("findings: open"), "{text}");
    assert_eq!(exit, 0, "{text}");
    // The done step's gate no longer holds: the plan says so, and so does
    // gate lines (exit 2).
    put(&w.kb.join(".writfold/gates/note.md"), "# Read it all\n")?;
    let (text, exit) = lines(&w)?;
    assert!(header(&text).ends_with("; plan complete-stale"), "{text}");
    assert!(text.lines().any(|l| l == "findings: skipped"), "{text}");
    assert_eq!(exit, 2, "{text}");
    Ok(())
}

#[test]
fn every_gate_met_is_not_enough_while_the_plan_is_incomplete() -> TestResult {
    let w = set_up()?;
    attest(&w, "note")?;
    attest(&w, "findings")?;
    let (text, exit) = lines(&w)?;
    assert!(
        header(&text).ends_with("0 of 2 satisfied, 2 attested; plan incomplete"),
        "{text}"
    );
    assert_eq!(exit, 2, "{text}");
    for _ in 0..2 {
        assert_eq!(plan(&w, false)?.1, 0);
    }
    let (text, exit) = lines(&w)?;
    assert!(
        header(&text).ends_with("2 attested; plan complete-with-exceptions"),
        "{text}"
    );
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn a_moved_policy_exits_two_with_the_plan_word() -> TestResult {
    let w = set_up_with(Some(
        "schema = \"writfold-plan-required/1\"\nstart = [\"two\"]\n",
    ))?;
    attest(&w, "note")?;
    assert_eq!(plan(&w, false)?.1, 0);
    assert_eq!(plan(&w, true)?.1, 0);
    assert_eq!(lines(&w)?.1, 0);
    let moved = "schema = \"writfold-plan-required/1\"\nstart = [\"two\"]\nend = [\"two\"]\n";
    put(&w.kb.join(".writfold/plans/required.toml"), moved)?;
    let (text, exit) = lines(&w)?;
    assert!(header(&text).ends_with("; plan policy-moved"), "{text}");
    assert_eq!(exit, 2, "{text}");
    Ok(())
}
