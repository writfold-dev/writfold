//! A delivery mode that declares nothing is never ready: `deliver status`
//! reads its candidate `[blocked]`, and the delivery guard refuses its
//! bytes, as a plan step on such a mode is refused (an empty set is not
//! met: `writfold_core::delivery::ready`).
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{RUN, deliver_status, fixture, prepare, put};
use serde_json::{Value, json};
use support::{NOW, TestResult, World};
use writfold_kb::hook::{HookEnv, run};

const BODY: &str = "Batching is live in staging.\n";

/// Whether the delivery guard refuses posting `body` with `gh pr comment`.
fn guard_denies(w: &World, body: &str) -> TestResult<bool> {
    std::fs::create_dir_all(w.kb.join(".git/writfold"))?;
    writfold_kb::runs::set_current(&w.kb.join(".git/writfold"), RUN)?;
    let guard = "schema = \"writfold-guard/1\"\ncommands = [\"gh pr comment\"]\n";
    put(w, ".writfold/deliveries/guard.toml", guard)?;
    let file = w.base.join("comment.md");
    std::fs::write(&file, body)?;
    let event = json!({"session_id": "s1", "cwd": w.base, "hook_event_name": "PreToolUse",
        "tool_name": "Bash", "tool_use_id": "t1",
        "tool_input": {"command": format!("gh pr comment 42 --body-file {}", file.display())}});
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 9000),
    };
    let out = run("pre-tool-use", &event.to_string(), &env);
    if out.stdout.is_empty() {
        return Ok(false);
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    Ok(value["hookSpecificOutput"]["permissionDecision"] == "deny")
}

#[test]
fn a_mode_that_declares_nothing_is_never_ready() -> TestResult {
    let w = fixture()?;
    let mode = "schema = \"writfold-delivery-mode/1\"\nmode = \"empty\"\n";
    put(&w, ".writfold/deliveries/empty.toml", mode)?;
    let id = prepare(&w, "empty", BODY, NOW + 60)?;
    let (text, exit) = deliver_status(&w)?;
    assert!(
        text.contains(&format!("delivery {id} [blocked]")),
        "an empty mode read ready:\n{text}"
    );
    assert_eq!(exit, 2, "{text}");
    assert!(
        guard_denies(&w, BODY)?,
        "the guard let an empty mode's bytes through"
    );
    Ok(())
}
