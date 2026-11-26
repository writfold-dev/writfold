//! `agent prepare` (#147, #148): each of the six refusals writes nothing,
//! exits 3 and names its `next:` step; an admitted prepare records one
//! invocation at the current step and writes the package outside the KB;
//! a skill's plan counts once `plan use` splices it; two prepares with
//! equal inputs are two invocations; the record holds no absolute path.
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

mod agent_support;

use agent_support::{AGENT, invocations, outcome, prepare, setup, use_plan};
use helpers::{RUN, put, set};
use support::{TestResult, World};
use writfold_kb::Verb;
use writfold_kb::agents::AgentVerb;
use writfold_kb::agents::prepare::Prepare;
use writfold_kb::plan::PlanVerb;

fn refused(w: &World, out: &str, want: &[&str]) -> TestResult {
    let (text, exit) = prepare(w, out)?;
    assert_eq!(exit, 3, "{text}");
    for part in want {
        assert!(text.contains(part), "{part:?} not in {text}");
    }
    assert!(invocations(w).is_empty(), "{:?}", invocations(w));
    assert!(!w.base.join(out).exists(), "a package was written");
    Ok(())
}

#[test]
fn no_current_step_then_a_missing_plan_are_refused() -> TestResult {
    let w = setup()?;
    refused(
        &w,
        "pkg",
        &["has no current step", "next: writfold kb plan use"],
    )?;
    set(&w, "change")?;
    refused(
        &w,
        "pkg",
        &[
            "skill verify-claims names plan claims-review, which is not in this run",
            "next: writfold kb plan use claims-review",
        ],
    )?;
    use_plan(&w, "claims-review")?;
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("at change#1/implement/claims-review#1/review"),
        "{text}"
    );
    assert_eq!(invocations(&w).len(), 1);
    Ok(())
}

#[test]
fn a_closure_fault_and_an_unread_input_are_refused() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("tools = [\"repo.read\"]\n", ""),
    )?;
    refused(
        &w,
        "pkg",
        &[
            "does not grant it",
            "next: writfold kb agent inspect reviewer",
        ],
    )?;
    put(&w, ".writfold/agents/reviewer.toml", AGENT)?;
    std::fs::remove_file(w.base.join("task.md"))?;
    refused(&w, "pkg", &["--input task.md", "next: pass a readable"])
}

#[test]
fn a_closed_run_and_a_partial_packet_are_refused() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    let packet = AGENT.replace("model =", "packet = \"required\"\nmodel =");
    put(&w, ".writfold/agents/reviewer.toml", &packet)?;
    let verb = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: "pkg".into(),
        retry_of: None,
        by: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        for_item: None,
    }));
    let mut request = w.request(verb.clone());
    request.budget = 20;
    let out = outcome(&request);
    assert_eq!(out.1, 3, "{}", out.0);
    assert!(out.0.contains("requires a context packet"), "{}", out.0);
    assert!(invocations(&w).is_empty());
    // With the whole packet it is admitted, and the package holds it.
    let (text, exit) = outcome(&w.request(verb));
    assert_eq!(exit, 0, "{text}");
    assert!(w.base.join("pkg/packet.md").is_file());
    let close = writfold_kb::write::status::SetStatus {
        target: RUN.into(),
        state: "closed".into(),
        ..Default::default()
    };
    let out =
        writfold_kb::run(&w.request(Verb::Write(writfold_kb::write::Write::SetStatus(close))))?;
    assert_eq!(out.exit, 0, "{}", out.text);
    refused_more(&w)
}

fn refused_more(w: &World) -> TestResult {
    let (text, exit) = prepare(w, "pkg2")?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("is closed; next: writfold kb status"),
        "{text}"
    );
    assert_eq!(invocations(w).len(), 1);
    Ok(())
}

#[test]
fn the_record_and_package_hold_paths_and_digests() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 0, "{text}");
    let (again, exit) = prepare(&w, "pkg2")?;
    assert_eq!(exit, 0, "{again}");
    // Equal inputs are two invocations.
    let names = invocations(&w);
    assert_eq!(names.len(), 2, "{names:?}");
    let copy = std::fs::read_to_string(w.base.join("pkg/invocation.json"))?;
    let v: serde_json::Value = serde_json::from_str(&copy)?;
    let id = v["id"].as_str().ok_or("id")?;
    let record = std::fs::read_to_string(w.kb.join(format!("runs/{RUN}/invocations/{id}.json")))?;
    assert_eq!(v["schema"], "writfold-invocation/1");
    assert_eq!(v["agent"], "reviewer");
    assert_eq!(v["step"], "change#1/implement/claims-review#1/review");
    assert_eq!(v["input"]["name"], "task.md");
    assert_eq!(v["by"], "harness");
    let base = w.base.to_string_lossy().into_owned();
    assert!(!record.contains(&base), "an absolute path in {record}");
    let kinds: Vec<&str> = (v["definitions"].as_array().ok_or("definitions")?.iter())
        .filter_map(|d| d["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["agent", "skill", "tool"]);
    let instructions = std::fs::read_to_string(w.base.join("pkg/instructions.md"))?;
    assert_eq!(
        instructions,
        "# Agent reviewer\n\nReview only the supplied change.\n\n# Skill verify-claims\n\nCheck each claim against the diff.\n"
    );
    assert_eq!(std::fs::read(w.base.join("pkg/input"))?, b"Review PR 42.\n");
    assert_eq!(copy, record);
    // A non-empty --out is refused before anything is recorded.
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 1, "{text}");
    assert_eq!(invocations(&w).len(), 2);
    Ok(())
}

/// The files under `dir`, with their bytes.
fn files(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        match path.is_dir() {
            true => out.extend(files(&path)),
            false => out.extend(std::fs::read(&path).ok().map(|b| (path, b))),
        }
    }
    out
}

static INJECT: std::sync::Mutex<Vec<(std::path::PathBuf, Vec<u8>)>> =
    std::sync::Mutex::new(Vec::new());

fn inject(step: writfold_kb::write::Step) {
    if step != writfold_kb::write::Step::Locked {
        return;
    }
    for (path, bytes) in INJECT
        .lock()
        .map(|mut i| std::mem::take(&mut *i))
        .unwrap_or_default()
    {
        let _ = std::fs::write(path, bytes);
    }
}

/// Another session skips the step between this prepare's read and its
/// lock: the record is refused, not written at a stale step.
#[test]
fn a_step_that_moved_under_the_lock_is_refused() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    let plan = w.kb.join(format!("runs/{RUN}/plan"));
    let before = files(&plan);
    let skip = Verb::Plan(PlanVerb::Skip {
        reason: "not needed".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    assert_eq!(helpers::call(&w, skip)?.1, 0);
    let written: Vec<_> = files(&plan)
        .into_iter()
        .filter(|f| !before.contains(f))
        .collect();
    assert!(!written.is_empty());
    for (path, _) in &written {
        std::fs::remove_file(path)?;
    }
    *INJECT.lock().map_err(|_| "poisoned")? = written;
    let mut request = w.request(Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: "pkg".into(),
        retry_of: None,
        by: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        for_item: None,
    })));
    request.probe = inject;
    let (text, exit) = outcome(&request);
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("is no longer the current step"), "{text}");
    assert!(invocations(&w).is_empty());
    assert!(!w.base.join("pkg").exists());
    Ok(())
}
