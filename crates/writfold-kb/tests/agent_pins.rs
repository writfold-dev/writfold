//! Pinned agent closures (#161, RFC 2026-11-26-pinned-agent-closures):
//! a run whose plan owes a mode naming an agent pins `agent:<id>` with its
//! closure digest. While the step owing the mode is open, an edited mode
//! (pinned already) or an edit to its agent's closure (the profile, a
//! skill, the `model` hint; a tool's `description` is not behaviour) makes the run read `policy moved`; `plan repin` adopts the
//! closure as it is now; a plan that owes no such mode pins no agent.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use agent_support::{AGENT, SKILL, setup};
use helpers::{RUN, put, set};
use support::{TestResult, World};
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;

const MODE: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"review\"\nchecks = [\"invocation.bound\"]\nagent = \"reviewer\"\n";
const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"claims-review\"\nsteps = [\n  \"review: deliver:review\",\n]\n";
const MOVED: &str = "policy moved since this run pinned it";

fn world() -> TestResult<World> {
    let w = setup()?;
    put(&w, ".writfold/deliveries/review.toml", MODE)?;
    put(&w, ".writfold/plans/claims-review.toml", PLAN)?;
    assert_eq!(set(&w, "claims-review")?.1, 0);
    Ok(w)
}

/// Every pin file's text, joined.
fn pins(w: &World) -> TestResult<String> {
    let dir = w.kb.join(format!("runs/{RUN}/policy"));
    let mut text = String::new();
    for entry in std::fs::read_dir(dir)? {
        text.push_str(&std::fs::read_to_string(entry?.path())?);
    }
    Ok(text)
}

fn status(w: &World) -> TestResult<String> {
    Ok(helpers::status(w, true)?.0)
}

fn repin(w: &World) -> TestResult<(String, u8)> {
    helpers::call(
        w,
        Verb::Plan(PlanVerb::Repin {
            reason: "the reviewer's instructions were tightened on purpose".into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )
}

#[test]
fn an_edited_mode_reads_policy_moved_while_its_step_is_open() -> TestResult {
    let w = world()?;
    assert!(!status(&w)?.contains(MOVED));
    put(
        &w,
        ".writfold/deliveries/review.toml",
        &MODE.replace(
            "checks = [\"invocation.bound\"]",
            "checks = [\"claims.cited-fresh\"]",
        ),
    )?;
    let text = status(&w)?;
    assert!(text.contains(MOVED), "{text}");
    assert!(text.contains("deliver:review"), "{text}");
    Ok(())
}

#[test]
fn an_edited_closure_of_the_modes_agent_reads_policy_moved() -> TestResult {
    let w = world()?;
    let pinned = pins(&w)?;
    assert!(pinned.contains("\"agent:reviewer\""), "{pinned}");
    let edits: [(&str, String); 3] = [
        (
            ".writfold/agents/reviewer.toml",
            AGENT.replace("Review only", "Review just"),
        ),
        (
            ".writfold/skills/verify-claims.toml",
            SKILL.replace("against the diff", "line by line"),
        ),
        (
            ".writfold/agents/reviewer.toml",
            AGENT.replace("model = \"reviewer\"", "model = \"other\""),
        ),
    ];
    for (path, edited) in edits {
        let before = std::fs::read_to_string(w.kb.join(path))?;
        put(&w, path, &edited)?;
        let text = status(&w)?;
        assert!(text.contains(MOVED), "{path}: {text}");
        assert!(text.contains("agent:reviewer"), "{path}: {text}");
        put(&w, path, &before)?;
        assert!(!status(&w)?.contains(MOVED), "{path} restored");
    }
    Ok(())
}

#[test]
fn repin_adopts_the_closure_as_it_is_now() -> TestResult {
    let w = world()?;
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("Review only", "Review just"),
    )?;
    assert!(status(&w)?.contains(MOVED));
    let (text, exit) = repin(&w)?;
    assert_eq!(exit, 0, "{text}");
    let text = status(&w)?;
    assert!(!text.contains(MOVED), "{text}");
    Ok(())
}

#[test]
fn a_plan_that_owes_no_agent_mode_pins_no_agent() -> TestResult {
    let w = setup()?;
    put(&w, ".writfold/deliveries/review.toml", MODE)?;
    let plain = "schema = \"writfold-plan/1\"\nname = \"plain\"\nsteps = [\n  \"work:\",\n]\n";
    put(&w, ".writfold/plans/plain.toml", plain)?;
    assert_eq!(set(&w, "plain")?.1, 0);
    let pinned = pins(&w)?;
    assert!(!pinned.contains("agent:"), "{pinned}");
    Ok(())
}
