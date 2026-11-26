//! `needs` at `agent prepare` (#154): a gate a skill or profile needs is
//! judged for the run and its subject as a guard rule's `gate:<id>` is.
//! Prepare is refused while the gate is open, failed, stale or cannot be
//! judged, and admitted when it is met. A file without `needs` keeps its
//! closure digest; `kb lint` reports a need no gate defines.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use agent_support::{AGENT, SKILL, invocations, prepare, setup, use_plan};
use helpers::{RUN, put, set};
use support::{TestResult, World, git};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

fn gate(argv: &str) -> String {
    format!(
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = {argv}\ntimeout_seconds = 30\nscope = [\"src/**\"]\n"
    )
}

fn run_gate(w: &World) -> TestResult<u8> {
    Ok(writfold_kb::run(&w.request(Verb::Gate(GateVerb::Run {
        id: "unit".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    })))?
    .exit)
}

fn needing() -> TestResult<World> {
    let w = setup()?;
    put(&w, ".writfold/gates/unit.toml", &gate("[\"false\"]"))?;
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("model =", "needs = [\"unit\"]\nmodel ="),
    )?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    Ok(w)
}

fn refused(w: &World, out: &str, want: &str) -> TestResult {
    let (text, exit) = prepare(w, out)?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("agent reviewer needs gate unit"), "{text}");
    assert!(text.contains(want), "{want:?} not in {text}");
    assert!(invocations(w).is_empty());
    Ok(())
}

#[test]
fn a_needed_gate_open_failed_met_then_stale() -> TestResult {
    let w = needing()?;
    refused(&w, "pkg", "is not met")?;
    run_gate(&w)?;
    refused(&w, "pkg", "[failed]")?;
    put(&w, ".writfold/gates/unit.toml", &gate("[\"true\"]"))?;
    assert_eq!(run_gate(&w)?, 0);
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 0, "{text}");
    // The record keeps what each need stood on (#167).
    let record = std::fs::read_to_string(w.base.join("pkg/invocation.json"))?;
    assert!(record.contains("\"needs\""), "{record}");
    assert!(record.contains("\"gate\": \"unit\""), "{record}");
    assert!(record.contains("/gates/unit-"), "{record}");
    // The subject moves under the gate's scope: the pass is stale.
    std::fs::create_dir_all(w.code.join("src"))?;
    std::fs::write(w.code.join("src/new.rs"), "pub fn new() {}\n")?;
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "-m", "change"])?;
    let (text, exit) = prepare(&w, "pkg2")?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("[stale]"), "{text}");
    assert_eq!(invocations(&w).len(), 1);
    Ok(())
}

#[test]
fn a_skills_need_counts_and_an_undefined_gate_is_a_lint_error() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    put(
        &w,
        ".writfold/skills/verify-claims.toml",
        &SKILL.replace("plan =", "needs = [\"unit\"]\nplan ="),
    )?;
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("skill verify-claims needs gate unit"),
        "{text}"
    );
    let mut lint = w.request(Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: false,
    });
    lint.json = true;
    let out = writfold_kb::run(&lint)?;
    assert_eq!(out.exit, 2, "{}", out.text);
    assert!(
        out.text.contains("ref.undefined") && out.text.contains("no gate \\\"unit\\\""),
        "{}",
        out.text
    );
    Ok(())
}

#[test]
fn no_needs_keeps_the_closure_digest() -> TestResult {
    let w = setup()?;
    let digest = |w: &World| -> TestResult<String> {
        let mut r = w.request(Verb::Agent(writfold_kb::agents::AgentVerb::Inspect(
            writfold_kb::agents::Inspect {
                id: "reviewer".into(),
                repo: None,
            },
        )));
        r.json = true;
        let v: serde_json::Value = serde_json::from_str(&writfold_kb::run(&r)?.text)?;
        Ok(v["trailer"]["digest"].as_str().unwrap_or("").to_owned())
    };
    // A file without `needs` adds no field to the digest: removing the
    // key returns the digest it had.
    let before = digest(&w)?;
    // The digest the same files had before `needs` existed, recorded with
    // the tree at 90c3ba40 (#168): an always-written marker would move it.
    assert_eq!(
        before,
        "sha256:48cb46fc65ec25471f74004309e9062854195630af17f303ea2b3fa94cf4efc6"
    );
    put(&w, ".writfold/gates/unit.toml", &gate("[\"true\"]"))?;
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("model =", "needs = [\"unit\"]\nmodel ="),
    )?;
    assert_ne!(digest(&w)?, before, "a need is behaviour");
    put(&w, ".writfold/agents/reviewer.toml", AGENT)?;
    assert_eq!(digest(&w)?, before);
    Ok(())
}

/// With no subject repository the gate cannot be judged: unknown, refused.
#[test]
fn a_need_that_cannot_be_judged_is_refused() -> TestResult {
    let w = needing()?;
    let verb = Verb::Agent(writfold_kb::agents::AgentVerb::Prepare(
        writfold_kb::agents::prepare::Prepare {
            id: "reviewer".into(),
            input: "task.md".into(),
            out: "pkg".into(),
            retry_of: None,
            by: None,
            run: Some(RUN.into()),
            repo: None,
            for_item: None,
        },
    ));
    let (text, exit) = agent_support::outcome(&w.request(verb));
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("needs gate unit, which cannot be judged"),
        "{text}"
    );
    assert!(invocations(&w).is_empty());
    Ok(())
}

/// A profile's own need naming no gate is a lint error, as a skill's is.
#[test]
fn a_profile_need_no_gate_defines_is_reported() -> TestResult {
    let w = setup()?;
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("model =", "needs = [\"ghost\"]\nmodel ="),
    )?;
    let mut lint = w.request(Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: false,
    });
    lint.json = true;
    let out = writfold_kb::run(&lint)?;
    assert_eq!(out.exit, 2, "{}", out.text);
    assert!(out.text.contains("no gate \\\"ghost\\\""), "{}", out.text);
    Ok(())
}

/// A waived need stands as the gate's `accept` says: the default accepts a
/// waiver, so prepare is admitted, and the record keeps the waiver it
/// stood on.
#[test]
fn a_waived_need_stands_as_accept_says() -> TestResult {
    let w = needing()?;
    run_gate(&w)?;
    refused(&w, "pkg", "[failed]")?;
    let say = GateVerb::Say(writfold_kb::gate::Say {
        kind: writfold_kb::gate::statement::Kind::Waived,
        id: "unit".into(),
        text: "the suite is known broken upstream".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(say)))?;
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 0, "{text}");
    let record = std::fs::read_to_string(w.base.join("pkg/invocation.json"))?;
    assert!(record.contains("/statements/unit-"), "{record}");
    Ok(())
}
