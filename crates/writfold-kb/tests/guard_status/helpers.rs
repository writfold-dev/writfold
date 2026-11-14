use super::support::{NOW, TestResult, World};
use serde_json::{Value, json};
use std::path::Path;
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::guard::GuardVerb;
use writfold_kb::hook::{HookEnv, HookOutput};

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";
pub const COMMIT: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\", \"git push\"]\ntools = [\"Edit\"]\nrequire = [\"run.current\", \"gates.met\"]\naction = \"deny\"\n";
pub const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";
pub const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n";
pub const STOP: &str = "schema = \"writfold-guard-rule/1\"\nid = \"finish\"\nevent = \"stop\"\nrequire = [\"step.met\"]\n";

pub fn put(path: &Path, text: &str) -> TestResult {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn rule(root: &Path, id: &str, text: &str) -> TestResult {
    put(&root.join(format!(".writfold/guards/{id}.toml")), text)
}

pub fn definition(w: &World, id: &str, text: &str) -> TestResult {
    put(&w.kb.join(format!(".writfold/gates/{id}.toml")), text)
}

pub fn current(w: &World) -> TestResult {
    put(&w.kb.join(".git/writfold/current-run"), &format!("{RUN}\n"))
}

pub fn status(w: &World) -> TestResult<(Value, u8)> {
    let mut request = w.request(Verb::Guard(GuardVerb::Status));
    request.cwd = w.code.clone();
    request.now = Some(NOW + 3600);
    request.json = true;
    let output = writfold_kb::run(&request)?;
    Ok((serde_json::from_str(&output.text)?, output.exit))
}

pub fn row<'a>(answer: &'a Value, id: &str) -> TestResult<&'a Value> {
    answer["rows"]
        .as_array()
        .ok_or("rows")?
        .iter()
        .find(|r| r["id"] == id)
        .ok_or_else(|| format!("missing {id}: {answer}").into())
}

pub fn hook(w: &World, stop: bool, session: &str) -> HookOutput {
    let mut input = json!({"session_id": session, "cwd": w.code,
        "hook_event_name": if stop { "Stop" } else { "PreToolUse" }});
    if stop {
        input["stop_hook_active"] = json!(false);
        input["last_assistant_message"] = json!("Done.");
    } else {
        input["tool_name"] = json!("Bash");
        input["tool_input"] = json!({"command": "git commit -m x"});
    }
    writfold_kb::hook::run(
        if stop { "stop" } else { "pre-tool-use" },
        &input.to_string(),
        &HookEnv {
            kb: Some(w.kb.clone()),
            cwd: w.base.clone(),
            now: Some(NOW + 3600),
        },
    )
}

pub fn parity(w: &World, expected: &str) -> TestResult<Value> {
    let (answer, exit) = status(w)?;
    let guard = row(&answer, "commit")?;
    assert_eq!(guard["status"], format!("would-{expected}"));
    assert_eq!(exit, if expected == "pass" { 0 } else { 2 });
    let out = hook(w, false, "commit-session");
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    if expected == "pass" {
        assert!(out.stdout.is_empty(), "{}", out.stdout);
        assert_eq!(guard["more"], json!([]));
        assert_eq!(guard["flags"], json!([]));
    } else {
        let value: Value = serde_json::from_str(&out.stdout)?;
        let specific = &value["hookSpecificOutput"];
        assert_eq!(specific["permissionDecision"], expected);
        let reason = specific["permissionDecisionReason"]
            .as_str()
            .ok_or("reason")?;
        for owed in guard["more"].as_array().ok_or("more")? {
            assert_eq!(owed["label"], "owes");
            assert!(
                reason.contains(owed["text"].as_str().ok_or("owed text")?),
                "{reason}"
            );
        }
        assert_eq!(guard["flags"], json!([expected]));
        assert_eq!(answer["trailer"]["counts"][expected], 1);
    }
    Ok(answer)
}

pub fn check(w: &World, id: &str) -> TestResult<u8> {
    Ok(writfold_kb::run(&w.request(Verb::Gate(GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    })))?
    .exit)
}

pub fn say(w: &World, kind: Kind, id: &str) -> TestResult {
    let output = writfold_kb::run(&w.request(Verb::Gate(GateVerb::Say(Say {
        kind,
        id: id.into(),
        text: "looked at it".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    }))))?;
    assert_eq!(output.exit, 0);
    Ok(())
}
