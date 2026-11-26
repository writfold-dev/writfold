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

/// `agent prepare reviewer` with `--retry-of`.
fn retry(w: &World, out: &str, of: &str) -> (String, u8) {
    let verb = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: out.into(),
        retry_of: Some(of.into()),
        by: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        for_item: None,
    }));
    outcome(&w.request(verb))
}

/// Pins: `--retry-of ID` with an existing invocation records the link in
/// the new record and its package copy, and the first reads `retried`
/// (no report names it) in `resume` and its run row, while the retry is
/// `prepared`.
#[test]
fn a_retry_links_the_records_and_the_first_reads_retried() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    assert_eq!(prepare(&w, "pkg")?.1, 0);
    let first = invocations(&w)[0].trim_end_matches(".json").to_owned();
    let (text, exit) = retry(&w, "pkg2", &first);
    assert_eq!(exit, 0, "{text}");
    assert_eq!(invocations(&w).len(), 2);
    let copy = std::fs::read_to_string(w.base.join("pkg2/invocation.json"))?;
    let v: serde_json::Value = serde_json::from_str(&copy)?;
    assert_eq!(v["retry_of"], first.as_str(), "{copy}");
    let (text, _) = outcome(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }));
    assert!(
        text.contains("1 retried") && text.contains("1 prepared"),
        "{text}"
    );
    let row = (text.lines()).find(|l| l.starts_with(&format!("invocation {first} ")));
    assert!(row.is_some_and(|r| r.contains("[retried]")), "{text}");
    Ok(())
}

/// Pins: `--retry-of` naming an invocation the run does not have is
/// refused (a usage error) before anything is recorded or written.
#[test]
fn a_retry_of_no_such_invocation_is_refused() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    let (text, exit) = retry(&w, "pkg", "nope");
    assert_eq!(exit, 1, "{text}");
    assert!(text.contains("has no such invocation"), "{text}");
    assert!(text.contains("nope"), "{text}");
    assert!(invocations(&w).is_empty());
    assert!(!w.base.join("pkg").exists());
    Ok(())
}

/// Pins: definitions that live in the repository under review (not the KB
/// root) are found by `agent inspect --repo` and `agent prepare`, and the
/// record names them as `<repo>:<path>`.
#[test]
fn definitions_in_the_repository_are_found() -> TestResult {
    let w = setup()?;
    for rel in [
        ".writfold/tools/repo.read.toml",
        ".writfold/skills/verify-claims.toml",
        ".writfold/agents/reviewer.toml",
    ] {
        let to = w.code.join(rel);
        std::fs::create_dir_all(to.parent().ok_or("parent")?)?;
        std::fs::rename(w.kb.join(rel), to)?;
    }
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    let inspect = Verb::Agent(AgentVerb::Inspect(writfold_kb::agents::Inspect {
        id: "reviewer".into(),
        repo: Some(w.code.clone()),
    }));
    let (text, exit) = outcome(&w.request(inspect));
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("demo-service:.writfold/agents/reviewer.toml"),
        "{text}"
    );
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 0, "{text}");
    let copy = std::fs::read_to_string(w.base.join("pkg/invocation.json"))?;
    let v: serde_json::Value = serde_json::from_str(&copy)?;
    let paths: Vec<&str> = (v["definitions"].as_array().ok_or("definitions")?.iter())
        .filter_map(|d| d["path"].as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "demo-service:.writfold/agents/reviewer.toml",
            "demo-service:.writfold/skills/verify-claims.toml",
            "demo-service:.writfold/tools/repo.read.toml",
        ]
    );
    Ok(())
}
