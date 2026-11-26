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

#[test]
fn the_example_runs_end_to_end() -> TestResult {
    let w = world(&["agent-review"])?;
    let (text, exit) = lint(&w)?;
    assert_eq!(exit, 0, "{text}");
    set(&w, "claims-review")?;
    std::fs::write(
        w.base.join("task.md"),
        "Review PR 42: \"batching halves flush time\".\n",
    )?;
    let prepare = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: "pkg".into(),
        retry_of: None,
        by: Some("harness".into()),
        run: Some(RUN.into()),
        repo: None,
    }));
    let (text, exit) = outcome(&w.request(prepare));
    assert_eq!(exit, 0, "{text}");
    let id = invocations(&w)
        .first()
        .ok_or("no invocation")?
        .trim_end_matches(".json")
        .to_owned();
    let instructions = std::fs::read_to_string(w.base.join("pkg/instructions.md"))?;
    assert!(
        instructions.contains("# Skill summarise\n\nEnd with three lines"),
        "{instructions}"
    );
    // The harness runs the worker; its report comes back as a candidate.
    std::fs::write(
        w.base.join("report.md"),
        "The claim holds: flush.rs lines 10-14.\n",
    )?;
    let p = writfold_kb::deliver::candidate::Prepare {
        mode: "claims-report".into(),
        from: w.base.join("report.md"),
        audience: None,
        destination: None,
        run: Some(RUN.into()),
        invocation: Some(id),
    };
    let (text, exit) = helpers::deliver(&w, DeliverVerb::Prepare(p), NOW + 10)?;
    assert_eq!(exit, 0, "{text}");
    let c = helpers::ids(&w)?.pop().ok_or("no candidate")?;
    let (text, _) = check(&w, &c, NOW + 20)?;
    assert!(text.contains("invocation.bound [satisfied]"), "{text}");
    let (text, _) = say(&w, Kind::Attested, &c, "report.read", NOW + 30)?;
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
    let inspect = Verb::Agent(AgentVerb::Inspect(writfold_kb::agents::Inspect {
        id: "reviewer".into(),
        repo: None,
    }));
    let (text, exit) = outcome(&w.request(inspect));
    assert_eq!(exit, 0, "{text}");
    Ok(())
}
