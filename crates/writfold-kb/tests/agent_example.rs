//! The agent example (#151): `fixtures/kb/agent-review/`, which `kb init
//! --example agent` writes, lints clean and runs end to end: the plan the
//! skill names, `agent prepare`, the worker's report delivered and bound
//! to its invocation, a person's statement, and the step advanced. Every
//! TOML example in `docs/guide/agents.md` is a file Writfold reads.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use agent_support::{invocations, outcome};
use helpers::{RUN, advance, call, check, say, set};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::agents::AgentVerb;
use writfold_kb::agents::prepare::Prepare;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::gate::statement::Kind;

fn lint(w: &World) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: true,
        records: false,
    });
    request.json = true;
    Ok(outcome(&request))
}

fn inspect(w: &World) -> (String, u8) {
    let verb = Verb::Agent(AgentVerb::Inspect(writfold_kb::agents::Inspect {
        id: "reviewer".into(),
        repo: None,
    }));
    outcome(&w.request(verb))
}

fn prepare(w: &World, out: &str) -> TestResult<String> {
    let verb = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: out.into(),
        retry_of: None,
        by: Some("harness".into()),
        run: Some(RUN.into()),
        repo: None,
        for_item: None,
    }));
    let before = invocations(w);
    let (text, exit) = outcome(&w.request(verb));
    assert_eq!(exit, 0, "{text}");
    let new = invocations(w).into_iter().find(|i| !before.contains(i));
    Ok(new
        .ok_or("no invocation")?
        .trim_end_matches(".json")
        .to_owned())
}

/// The fake worker reads the package and writes `report`.
fn work(w: &World, pkg: &str, report: &str) -> TestResult {
    let status = std::process::Command::new("sh")
        .arg(w.kb.join("worker.sh"))
        .arg(w.base.join(pkg))
        .arg(w.base.join(report))
        .status()?;
    assert!(status.success());
    Ok(())
}

/// `deliver prepare --invocation`; returns the candidate.
fn deliver(w: &World, report: &str, id: &str, at: i64) -> TestResult<String> {
    let before = helpers::ids(w)?;
    let p = writfold_kb::deliver::candidate::Prepare {
        mode: "claims-report".into(),
        from: w.base.join(report),
        audience: None,
        destination: None,
        run: Some(RUN.into()),
        invocation: Some(id.to_owned()),
    };
    let (text, exit) = helpers::deliver(w, DeliverVerb::Prepare(p), at)?;
    assert_eq!(exit, 0, "{text}");
    (helpers::ids(w)?.into_iter())
        .find(|c| !before.contains(c))
        .ok_or_else(|| "no candidate".into())
}

fn example() -> TestResult<World> {
    let w = world(&["agent-review"])?;
    let (text, exit) = lint(&w)?;
    assert_eq!(exit, 0, "{text}");
    set(&w, "claims-review")?;
    std::fs::write(
        w.base.join("task.md"),
        "Review PR 42: \"batching halves flush time\".\n",
    )?;
    Ok(w)
}

#[test]
fn the_example_runs_end_to_end_with_a_fake_worker() -> TestResult {
    let w = example()?;
    let (text, exit) = inspect(&w);
    assert_eq!(exit, 0, "{text}");
    let id = prepare(&w, "pkg")?;
    let instructions = std::fs::read_to_string(w.base.join("pkg/instructions.md"))?;
    assert!(
        instructions.contains("# Skill summarise\n\nEnd with three lines"),
        "{instructions}"
    );
    work(&w, "pkg", "report.md")?;
    let c = deliver(&w, "report.md", &id, NOW + 10)?;
    let (text, _) = check(&w, &c, NOW + 20)?;
    assert!(text.contains("invocation.bound [satisfied]"), "{text}");
    assert!(text.contains("claims.cited-fresh [satisfied]"), "{text}");
    // A second, identical report changes no judgment.
    let again = deliver(&w, "report.md", &id, NOW + 25)?;
    assert!(
        check(&w, &again, NOW + 26)?
            .0
            .contains("invocation.bound [satisfied]")
    );
    let (text, _) = say(&w, Kind::Attested, &again, "report.read", NOW + 30)?;
    assert!(text.contains("[ready]"), "{text}");
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let (text, _) = call(
        &w,
        Verb::Resume {
            run: Some(RUN.into()),
        },
    )?;
    assert!(text.contains("1 delivered"), "{text}");
    Ok(())
}

/// Binding is not correctness: a report that says the wrong thing passes
/// `invocation.bound`; only the person's statement speaks to its content.
#[test]
fn a_wrong_report_still_passes_the_binding() -> TestResult {
    let w = example()?;
    let id = prepare(&w, "pkg")?;
    std::fs::write(w.base.join("wrong.md"), "Every claim is false.\n")?;
    let c = deliver(&w, "wrong.md", &id, NOW + 10)?;
    let (text, _) = check(&w, &c, NOW + 20)?;
    assert!(text.contains("invocation.bound [satisfied]"), "{text}");
    assert!(text.contains("report.read [open]"), "{text}");
    Ok(())
}

/// A skill edited after prepare and before any check fails the binding.
#[test]
fn a_skill_edited_before_the_check_fails() -> TestResult {
    let w = example()?;
    let id = prepare(&w, "pkg")?;
    work(&w, "pkg", "report.md")?;
    let skill = w.kb.join(".writfold/skills/verify-claims.toml");
    let text = std::fs::read_to_string(&skill)?;
    std::fs::write(
        &skill,
        text.replace("quote the lines that do", "quote them"),
    )?;
    let c = deliver(&w, "report.md", &id, NOW + 10)?;
    let (text, _) = check(&w, &c, NOW + 20)?;
    assert!(
        text.contains("invocation.bound [failed]") && text.contains("closure-moved"),
        "{text}"
    );
    Ok(())
}

/// Each fenced TOML block of the guide whose first line names its file
/// (`# .writfold/<dir>/<file>.toml`).
fn guide_files() -> TestResult<Vec<(String, String)>> {
    let guide = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/agents.md");
    let text = std::fs::read_to_string(guide)?;
    let mut out = Vec::new();
    for block in text.split("```toml\n").skip(1) {
        let body = block.split("```").next().unwrap_or_default();
        let first = body.lines().next().unwrap_or_default();
        if let Some(path) = first
            .strip_prefix("# ")
            .filter(|p| p.starts_with(".writfold/"))
        {
            out.push((path.to_owned(), body.to_owned()));
        }
    }
    Ok(out)
}

#[test]
fn every_toml_example_in_the_guide_reads() -> TestResult {
    let files = guide_files()?;
    assert!(files.len() >= 5, "{files:?}");
    let w = world(&[])?;
    for (path, body) in &files {
        helpers::put(&w, path, body)?;
    }
    helpers::put(&w, "docs/summarise.md", "Three lines.\n")?;
    let (text, exit) = lint(&w)?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = inspect(&w);
    assert_eq!(exit, 0, "{text}");
    Ok(())
}
