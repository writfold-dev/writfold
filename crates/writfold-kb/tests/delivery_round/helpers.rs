//! Shared steps for the delivery-modes tests: a world with a current run,
//! the deliver verbs (a refusal read as exit 3 with its reason), and the
//! PreToolUse hook.
#![allow(
    dead_code,
    reason = "shared by several test binaries; each one uses only part of it"
)]
use crate::support::{NOW, TestResult, World, world};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::error::KbError;
use writfold_kb::hook::{HookEnv, HookOutput, run};

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";
pub const CLEAN: &str = "Batching is live in staging.\n\nThe flush is measured next.\n";

/// A world whose current run is `RUN`, with `files` under `.writfold/deliveries/`.
pub fn kb(files: &[(&str, &str)]) -> TestResult<World> {
    let w = world(&[])?;
    let dir = w.kb.join(".writfold/deliveries");
    std::fs::create_dir_all(&dir)?;
    for (name, text) in files {
        std::fs::write(dir.join(name), text)?;
    }
    let state = w.kb.join(".git/writfold");
    std::fs::create_dir_all(&state)?;
    std::fs::write(state.join("current-run"), format!("{RUN}\n"))?;
    Ok(w)
}

/// Text and exit; a refusal is exit 3 with its reason.
pub fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    match writfold_kb::run(&request) {
        Ok(out) => Ok((out.text, out.exit)),
        Err(KbError::Refused(why)) => Ok((why, 3)),
        Err(e) => Err(e.into()),
    }
}

/// The verb's stderr drawing as well.
pub fn drawn(w: &World, verb: DeliverVerb, at: i64) -> TestResult<String> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    Ok(writfold_kb::run(&request)?.drawn.unwrap_or_default())
}

pub fn ids(w: &World) -> Vec<String> {
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

pub fn file(w: &World, name: &str, text: &str) -> TestResult<PathBuf> {
    let path = w.base.join(name);
    std::fs::write(&path, text)?;
    Ok(path)
}

pub fn prepare(w: &World, mode: &str, text: &str, at: i64) -> TestResult<String> {
    let from = file(w, &format!("draft-{at}.md"), text)?;
    let before = ids(w);
    let verb = DeliverVerb::Prepare(Prepare {
        mode: mode.into(),
        from,
        audience: None,
        destination: Some("harbor#42".into()),
        run: Some(RUN.into()),
        invocation: None,
    });
    let (out, exit) = deliver(w, verb, at)?;
    assert_eq!(exit, 0, "{out}");
    ids(w)
        .into_iter()
        .find(|id| !before.contains(id))
        .ok_or_else(|| format!("no new candidate\n{out}").into())
}

pub fn check(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: Some(id.into()),
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

pub fn status(w: &World, all: bool) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all,
    };
    deliver(w, verb, NOW + 9000)
}

pub fn withdraw(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Withdraw {
        candidate: Some(id.into()),
        reason: "pasted the wrong text".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

pub fn posted(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Posted {
        candidate: Some(id.into()),
        reference: "https://github.com/acme/harbor/pull/42#issuecomment-1".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

pub fn payload_path(w: &World, id: &str, name: &str) -> PathBuf {
    w.kb.join(format!("runs/{RUN}/deliveries/{id}/{name}"))
}

pub fn hook(w: &World, tool: &str, input: Value) -> HookOutput {
    let event = json!({
        "session_id": "s1",
        "cwd": w.base.display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": tool,
        "tool_input": input,
        "tool_use_id": "toolu_1",
    });
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    run("pre-tool-use", &event.to_string(), &env)
}

/// The guard's reason when it denies `command`; `None` when it allows.
pub fn guard(w: &World, command: &str) -> TestResult<Option<String>> {
    denied(&hook(w, "Bash", json!({"command": command})))
}

pub fn denied(out: &HookOutput) -> TestResult<Option<String>> {
    if out.stdout.is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let reason = value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .ok_or("no reason")?;
    Ok(Some(reason.to_owned()))
}

pub fn show(p: &Path) -> String {
    p.display().to_string()
}
