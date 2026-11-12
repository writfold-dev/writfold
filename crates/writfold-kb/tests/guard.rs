//! G2b: the optional PreToolUse guard (`writfold kb hook pre-tool-use`).
//! Silent with no guard configuration; with one, a configured publishing
//! command is denied unless the current run has a ready candidate holding
//! the bytes being posted (or, when they cannot be read from the command,
//! unless every candidate of the run is ready).
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::hook::{HookEnv, HookOutput, run};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const GUARD: &str = "schema = \"writfold-guard/1\"\ncommands = [\"gh pr comment\", \"gh issue comment\"]\ntools = [\"mcp__slack__post_message\"]\n";
const MODE: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nchecks = [\"paragraphs.bounded\"]\nparagraph_words = 10\n";

fn hook(w: &World, tool: &str, input: Value) -> HookOutput {
    let event = json!({
        "session_id": "s1",
        "cwd": w.base.display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": tool,
        "tool_input": input,
        "tool_use_id": "toolu_1",
    });
    let env = HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    run("pre-tool-use", &event.to_string(), &env)
}

fn bash(w: &World, command: &str) -> HookOutput {
    hook(
        w,
        "Bash",
        json!({"command": command, "description": "post"}),
    )
}

/// The denial reason, or `None` when the hook stayed silent.
fn denied(out: &HookOutput) -> TestResult<Option<String>> {
    if out.stdout.is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "PreToolUse");
    assert_eq!(specific["permissionDecision"], "deny");
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("no reason")?;
    assert!(reason.contains("a guard, not enforcement"), "{reason}");
    Ok(Some(reason.to_owned()))
}

fn config(w: &World, name: &str, text: &str) -> TestResult {
    let dir = w.kb.join(".writfold/deliveries");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(name), text)?;
    Ok(())
}

fn deliver(w: &World, verb: DeliverVerb) -> TestResult<u8> {
    Ok(writfold_kb::run(&w.request(Verb::Deliver(verb)))?.exit)
}

fn ids(w: &World) -> Vec<String> {
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

fn prepare(w: &World, from: &Path) -> TestResult<String> {
    let before = ids(w);
    deliver(
        w,
        DeliverVerb::Prepare(Prepare {
            mode: "pr-comment".into(),
            from: from.to_path_buf(),
            audience: Some("internal".into()),
            destination: None,
            run: Some(RUN.into()),
        }),
    )?;
    ids(w)
        .into_iter()
        .find(|id| !before.contains(id))
        .ok_or_else(|| "no new candidate".into())
}

fn check(w: &World, id: &str) -> TestResult<u8> {
    deliver(
        w,
        DeliverVerb::Check {
            candidate: Some(id.into()),
            check: None,
            run: Some(RUN.into()),
        },
    )
}

#[test]
fn fails_open_without_configuration() -> TestResult {
    let w = world(&[])?;
    let out = bash(&w, "gh pr comment 42 --body 'anything at all'");
    assert_eq!(out, HookOutput::default());
    let out = hook(&w, "mcp__slack__post_message", json!({"text": "hi"}));
    assert!(denied(&out)?.is_none());
    // No KB at all: silent, with a diagnostic only on stderr.
    let env = HookEnv {
        kb: Some(w.base.join("nowhere")),
        cwd: w.base.clone(),
        now: Some(NOW),
    };
    let event = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": "gh pr comment 1 -b x"}});
    let out = run("pre-tool-use", &event.to_string(), &env);
    assert!(out.stdout.is_empty() && !out.stderr.is_empty());
    Ok(())
}

#[test]
fn denies_and_allows_the_right_calls() -> TestResult {
    let w = world(&[])?;
    config(&w, "guard.toml", GUARD)?;
    config(&w, "pr-comment.toml", MODE)?;
    let body = w.base.join("comment.md");
    std::fs::write(&body, "Batching is live in staging.\n")?;
    let post = format!("cd repo && gh pr comment 42 --body-file {}", body.display());
    // Other commands, and other tools, pass untouched.
    assert!(denied(&bash(&w, "gh pr view 42 && ls"))?.is_none());
    assert!(denied(&hook(&w, "Read", json!({"file_path": "x"})))?.is_none());
    // No current run.
    let reason = denied(&bash(&w, &post))?.ok_or("expected a denial")?;
    assert!(reason.contains("no current KB run"), "{reason}");
    let state = w.kb.join(".git/writfold");
    std::fs::create_dir_all(&state)?;
    std::fs::write(state.join("current-run"), format!("{RUN}\n"))?;
    // A run with no candidate.
    let reason = denied(&bash(&w, &post))?.ok_or("expected a denial")?;
    assert!(reason.contains("no delivery candidate"), "{reason}");
    // A candidate holding these bytes, not yet checked.
    let id = prepare(&w, &body)?;
    let reason = denied(&bash(&w, &post))?.ok_or("expected a denial")?;
    assert!(
        reason.contains(&format!("candidate {id} owes paragraphs.bounded [open]")),
        "{reason}"
    );
    // Checked and ready: allowed by staying silent (the harness's own
    // permission flow still decides).
    assert_eq!(check(&w, &id)?, 0);
    assert_eq!(bash(&w, &post), HookOutput::default());
    let inline = "GH_REPO=a/b gh pr comment 42 --body 'Batching is live in staging.'";
    assert!(denied(&bash(&w, inline))?.is_none());
    // Other bytes than any candidate's.
    let other = denied(&bash(&w, "gh issue comment 7 -b 'Something else entirely'"))?;
    assert!(other.is_some_and(|r| r.contains("holds the bytes being posted")));
    // Bytes it cannot read: every candidate of the run must be ready.
    let expanded = "gh pr comment 42 --body \"$(cat comment.md)\"";
    assert!(denied(&bash(&w, expanded))?.is_none());
    assert!(denied(&hook(&w, "mcp__slack__post_message", json!({"text": "x"})))?.is_none());
    // A second, blocked candidate blocks run-level posts but not the
    // ready candidate's own bytes.
    let long = w.base.join("long.md");
    std::fs::write(&long, format!("{}\n", "word ".repeat(30)))?;
    let blocked = prepare(&w, &long)?;
    check(&w, &blocked)?;
    let reason = denied(&bash(&w, expanded))?.ok_or("expected a denial")?;
    assert!(
        reason.contains(&format!(
            "candidate {blocked} owes paragraphs.bounded [failed]"
        )),
        "{reason}"
    );
    assert!(denied(&bash(&w, &post))?.is_none());
    let reason = denied(&bash(&w, &format!("gh pr comment 1 -F {}", long.display())))?;
    assert!(reason.is_some_and(|r| r.contains("[failed]")));
    // An edit after the check: the candidate is no longer ready.
    let dir = w.kb.join(format!("runs/{RUN}/deliveries/{id}"));
    std::fs::write(
        dir.join("candidate.md"),
        "Batching is live in staging.\n\nAnd more.\n",
    )?;
    std::fs::write(&body, "Batching is live in staging.\n\nAnd more.\n")?;
    let reason = denied(&bash(&w, &post))?.ok_or("expected a denial")?;
    assert!(reason.contains("[stale]"), "{reason}");
    Ok(())
}
