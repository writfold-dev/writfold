//! D19 (#13): after a routed posting call returns, the post-tool-use hook
//! reminds the agent to record the post of the candidate it carried;
//! silent for anything else.
#[path = "delivery_round/helpers.rs"]
mod helpers;
mod support;

use helpers::{CLEAN, check, file, prepare, show};
use serde_json::{Value, json};
use support::{NOW, TestResult, World};
use writfold_kb::hook::{HookEnv, HookOutput, run};

const GUARD: &str = "schema = \"writfold-guard/1\"\ncommands = []\ntools = []\nroutes = [\n  \"gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F\",\n]\n";
const PR: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nchecks = [\"paragraphs.bounded\"]\n";

fn after(w: &World, command: &str) -> HookOutput {
    let event = json!({
        "session_id": "s1",
        "cwd": w.base.display().to_string(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command},
        "tool_response": {"stdout": "https://example.invalid/pr/42#c1", "stderr": ""},
        "tool_use_id": "toolu_1",
    });
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    run("post-tool-use", &event.to_string(), &env)
}

#[test]
fn a_routed_post_is_followed_by_the_record_command() -> TestResult {
    let w = helpers::kb(&[("guard.toml", GUARD), ("pr-comment.toml", PR)])?;
    let id = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let body = file(&w, "comment.md", CLEAN)?;
    let out = after(&w, &format!("gh pr comment 42 --body-file {}", show(&body)));
    let value: Value = serde_json::from_str(&out.stdout)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    let context = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("no context")?;
    assert!(
        context.contains(&format!("writfold kb deliver posted {id} --ref URL --by WHO")),
        "{context}"
    );
    // Other bytes, or a command no route names, say nothing.
    let other = file(&w, "other.md", "something else\n")?;
    let out = after(&w, &format!("gh pr comment 42 --body-file {}", show(&other)));
    assert!(out.stdout.is_empty(), "{out:?}");
    assert!(after(&w, "ls -la").stdout.is_empty());
    Ok(())
}
