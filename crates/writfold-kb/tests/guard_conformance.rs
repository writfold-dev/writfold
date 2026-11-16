//! Guard conformance cases (`fixtures/guard-conformance/<suite>/`): each
//! case's world is built, its event is run through the real hook entry
//! point with a fixed clock, and the decision is compared with
//! `expected.json`. Every failing case is reported by ID. A second test
//! checks each suite's shape, count, licence and recorded differences.
#[path = "guard_conformance/world.rs"]
mod cases;
#[path = "guard_conformance/compare.rs"]
mod compare;
mod support;

use cases::{build, keys, placed};
use compare::{compare, observed};
use serde_json::Value;
use std::path::{Path, PathBuf};
use support::{NOW, TestResult};
use writfold_kb::hook::{HookEnv, run};

const LICENCES: [&str; 6] = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "CC0-1.0",
    "Unlicense",
];
const FILES: [&str; 4] = [
    "event.json",
    "world.json",
    "expected.json",
    "transcript.jsonl",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/guard-conformance")
}

fn json(path: &Path) -> TestResult<Value> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(serde_json::from_str(&text).map_err(|e| not_json(path, &e))?)
}

/// A case file that is not JSON, as `path:line:col: why`.
fn not_json(path: &Path, e: &serde_json::Error) -> String {
    let why = e.to_string();
    let why = why.split(" at line ").next().unwrap_or(&why).to_owned();
    format!(
        "{}:{}:{}: not JSON: {why}",
        path.display(),
        e.line(),
        e.column()
    )
}

#[test]
fn a_case_file_that_is_not_json_is_named_at_its_line_and_column() -> TestResult {
    let dir = std::env::temp_dir().join(format!("wf-conformance-json-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("event.json");
    std::fs::write(
        &file,
        "{\n  \"tool_name\": \"Bash\",\n  \"tool_input\": {\"command\" \"git commit\"}\n}\n",
    )?;
    let why = json(&file).err().map(|e| e.to_string()).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        why.contains("event.json:3:28: not JSON: expected `:`"),
        "{why}"
    );
    Ok(())
}

fn dirs(path: &Path) -> TestResult<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(path)?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    out.sort();
    Ok(out)
}

fn suites() -> TestResult<Vec<PathBuf>> {
    dirs(&root())
}

/// Run one case; `Err` names what differs.
fn case(dir: &Path) -> TestResult<()> {
    let expected = json(&dir.join("expected.json"))?;
    let allowed = [
        "schema",
        "decision",
        "decided_by",
        "reason_has",
        "upstream",
        "differs",
    ];
    keys(&expected, &allowed, &["schema", "decision"], "expected")?;
    if expected["schema"] != "writfold-guard-expected/1" {
        return Err("expected schema".into());
    }
    let raw = json(&dir.join("event.json"))?;
    let event_keys = [
        "hook_event_name",
        "tool_name",
        "tool_input",
        "cwd",
        "session_id",
        "stop_hook_active",
        "last_assistant_message",
    ];
    keys(
        &raw,
        &event_keys,
        &["hook_event_name", "cwd", "session_id"],
        "event",
    )?;
    placed(raw["cwd"].as_str().ok_or("cwd")?)?;
    let (w, home, _) = build(&json(&dir.join("world.json"))?)?;
    let event = cases::fill(&raw, &w.base, &home.display().to_string());
    let name = match event["hook_event_name"].as_str() {
        Some("PreToolUse") => "pre-tool-use",
        Some("Stop") => "stop",
        other => return Err(format!("event {other:?}").into()),
    };
    let env = HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
        home: Some(home),
    };
    let out = run(name, &event.to_string(), &env);
    let (decision, reason) = observed(name, &out.stdout)?;
    let wrong = compare(&expected, &decision, &reason)?;
    match wrong.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "{}; reason: {reason}; stderr: {}",
            wrong.join("; "),
            out.stderr
        )
        .into()),
    }
}

#[test]
fn every_case_decides_as_expected() -> TestResult {
    let mut failed = Vec::new();
    let mut ran = 0;
    for suite in suites()? {
        for dir in dirs(&suite.join("cases"))? {
            ran += 1;
            if let Err(why) = case(&dir) {
                let id = dir
                    .strip_prefix(root())
                    .unwrap_or(&dir)
                    .display()
                    .to_string();
                failed.push(format!("{id}: {why}"));
            }
        }
    }
    assert!(ran > 0, "no conformance case was run");
    assert!(
        failed.is_empty(),
        "{} of {ran} cases failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
    Ok(())
}

#[test]
fn every_suite_is_well_formed() -> TestResult {
    let readme = std::fs::read_to_string(root().join("README.md"))?;
    for suite in suites()? {
        let meta = json(&suite.join("SUITE.json"))?;
        let allowed = [
            "schema",
            "name",
            "source",
            "revision",
            "licence",
            "imported",
            "converted_by",
            "cases",
        ];
        keys(&meta, &allowed, &allowed, "SUITE.json")?;
        assert_eq!(meta["schema"], "writfold-guard-suite/1");
        let licence = meta["licence"].as_str().ok_or("licence")?;
        assert!(LICENCES.contains(&licence), "licence {licence}");
        let cases = dirs(&suite.join("cases"))?;
        assert_eq!(
            meta["cases"].as_u64(),
            Some(cases.len() as u64),
            "{}",
            suite.display()
        );
        for dir in &cases {
            let id = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            for entry in std::fs::read_dir(dir)?.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                assert!(
                    FILES.contains(&name.as_str()),
                    "{id}: unexpected file {name}"
                );
            }
            for name in &FILES[..3] {
                assert!(dir.join(name).is_file(), "{id}: missing {name}");
            }
            let expected = json(&dir.join("expected.json"))?;
            let differs = expected
                .get("upstream")
                .is_some_and(|u| u["decision"] != expected["decision"]);
            if differs {
                assert!(
                    expected["differs"].as_str().is_some_and(|d| !d.is_empty()),
                    "{id}: no differs"
                );
                assert!(
                    readme.contains(&format!(
                        "| {} | {id} |",
                        meta["name"].as_str().unwrap_or("")
                    )),
                    "{id}: not in the README table"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn reading_is_strict() -> TestResult {
    let world = |extra: Value| -> Value {
        let mut w = serde_json::json!({"schema": "writfold-guard-world/1", "home": "{root}/home",
            "kb": {"run": "none"}});
        if let (Some(map), Value::Object(more)) = (w.as_object_mut(), extra) {
            map.extend(more);
        }
        w
    };
    assert!(build(&world(serde_json::json!({}))).is_ok());
    assert!(build(&world(serde_json::json!({"colour": "blue"}))).is_err());
    assert!(build(&world(serde_json::json!({"dirs": ["/etc/elsewhere"]}))).is_err());
    assert!(build(&world(serde_json::json!({"dirs": ["{root}/../up"]}))).is_err());
    let unknown_kb = serde_json::json!({"kb": {"run": "none", "flags": []}});
    assert!(build(&world(unknown_kb)).is_err());
    let bad_gate = serde_json::json!({"kb": {"run": "open", "gates": [{"id": "u", "repo": "x", "standing": "met"}]}});
    assert!(
        build(&world(bad_gate)).is_err(),
        "a gate's repository must exist"
    );
    assert!(placed("{home}/x").is_ok() && placed("relative").is_err());
    Ok(())
}
