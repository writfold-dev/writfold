//! Shared steps for the agent tests: a world with a tool, a skill naming
//! the plan `claims-review`, the profile `reviewer`, and `agent prepare`.
#![allow(
    dead_code,
    reason = "shared by several test binaries; each one uses only part of it"
)]
use crate::helpers::{self, RUN, fixture, put};
use crate::support::{TestResult, World};
use writfold_kb::Verb;
use writfold_kb::agents::AgentVerb;
use writfold_kb::agents::prepare::Prepare;
use writfold_kb::plan::PlanVerb;

pub const TOOL: &str = "schema = \"writfold-tool/1\"\nid = \"repo.read\"\n";
pub const SKILL: &str = "schema = \"writfold-skill/1\"\nid = \"verify-claims\"\ntools = [\"repo.read\"]\nplan = \"claims-review\"\ninstructions = \"\"\"\nCheck each claim against the diff.\n\"\"\"\n";
pub const AGENT: &str = "schema = \"writfold-agent-profile/1\"\nid = \"reviewer\"\nskills = [\"verify-claims\"]\ntools = [\"repo.read\"]\nmodel = \"reviewer\"\ninstructions = \"Review only the supplied change.\"\n";
pub const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"claims-review\"\nsteps = [\n  \"review: deliver:pr-comment\",\n]\n";

pub fn setup() -> TestResult<World> {
    let w = fixture()?;
    put(&w, ".writfold/tools/repo.read.toml", TOOL)?;
    put(&w, ".writfold/skills/verify-claims.toml", SKILL)?;
    put(&w, ".writfold/agents/reviewer.toml", AGENT)?;
    put(&w, ".writfold/plans/claims-review.toml", PLAN)?;
    std::fs::write(w.base.join("task.md"), "Review PR 42.\n")?;
    Ok(w)
}

/// The answer, or the refusal or error with its exit code.
pub fn outcome(request: &writfold_kb::Request) -> (String, u8) {
    match writfold_kb::run(request) {
        Ok(out) => (out.text, out.exit),
        Err(e) => (e.to_string(), e.exit_code()),
    }
}

pub fn prepare(w: &World, out: &str) -> TestResult<(String, u8)> {
    let verb = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: out.into(),
        retry_of: None,
        by: Some("harness".into()),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        for_item: None,
    }));
    Ok(outcome(&w.request(verb)))
}

pub fn use_plan(w: &World, name: &str) -> TestResult {
    let (out, exit) = helpers::call(
        w,
        Verb::Plan(PlanVerb::Use {
            template: name.into(),
            again: false,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )?;
    assert_eq!(exit, 0, "{out}");
    Ok(())
}

pub fn invocations(w: &World) -> Vec<String> {
    let dir = w.kb.join(format!("runs/{RUN}/invocations"));
    let mut out: Vec<String> = (std::fs::read_dir(dir).into_iter().flatten().flatten())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}
