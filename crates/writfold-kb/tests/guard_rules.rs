//! Guard rules (`.writfold/guards/<id>.toml`) through `writfold kb hook
//! pre-tool-use` and `stop`, over the harness of `tests/guard.rs`: silent
//! with no rule (and nothing opened), a matched command refused until the
//! run's gates are met with each unmet gate and its `next:` command named,
//! `repos` exclusions and repository placement, an unreadable KB taking
//! the rule's action, and the Stop nudge once per session.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::hook::{HookEnv, HookOutput, run};
use writfold_kb::plan::PlanVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const COMMIT: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\", \"git push\"]\ntools = [\"Edit\"]\nrequire = [\"run.current\", \"gates.met\"]\naction = \"deny\"\ndescription = \"commit only when the run's gates are met\"\n";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";
const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n";

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn rule(dir: &Path, id: &str, text: &str) -> TestResult {
    put(&dir.join(format!(".writfold/guards/{id}.toml")), text)
}

fn gate_def(w: &World, id: &str, text: &str) -> TestResult {
    put(&w.kb.join(format!(".writfold/gates/{id}.toml")), text)
}

fn current(w: &World) -> TestResult {
    put(&w.kb.join(".git/writfold/current-run"), &format!("{RUN}\n"))
}

fn event(cwd: &Path, name: &str, extra: Value) -> Value {
    let mut value = json!({"session_id": "s1", "cwd": cwd.display().to_string(),
                           "hook_event_name": name});
    if let (Some(map), Value::Object(more)) = (value.as_object_mut(), extra) {
        map.extend(more);
    }
    value
}

fn hook_at(w: &World, cwd: &Path, tool: &str, input: Value) -> HookOutput {
    let input = event(
        cwd,
        "PreToolUse",
        json!({"tool_name": tool, "tool_input": input}),
    );
    let env = HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    run("pre-tool-use", &input.to_string(), &env)
}

fn bash(w: &World, command: &str) -> HookOutput {
    hook_at(w, &w.code, "Bash", json!({"command": command}))
}

/// The decision and reason, or `None` when the hook stayed silent.
fn decided(out: &HookOutput) -> TestResult<Option<(String, String)>> {
    if out.stdout.is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "PreToolUse");
    let decision = specific["permissionDecision"]
        .as_str()
        .ok_or("no decision")?;
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("no reason")?;
    assert!(reason.contains("a guard, not enforcement"), "{reason}");
    Ok(Some((decision.to_owned(), reason.to_owned())))
}

fn denied(out: &HookOutput) -> TestResult<String> {
    match decided(out)? {
        Some((d, reason)) if d == "deny" => Ok(reason),
        other => Err(format!("expected a denial, got {other:?}").into()),
    }
}

fn index_files(w: &World) -> usize {
    std::fs::read_dir(w.kb.join(".git/writfold"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("kb-index"))
        .count()
}

fn gate(w: &World, verb: GateVerb) -> TestResult<u8> {
    Ok(writfold_kb::run(&w.request(Verb::Gate(verb)))?.exit)
}

fn gate_run(w: &World, id: &str) -> TestResult<u8> {
    let verb = GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    gate(w, verb)
}

fn say(w: &World, kind: Kind, id: &str) -> TestResult<u8> {
    let verb = GateVerb::Say(Say {
        kind,
        id: id.into(),
        text: "looked at it".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    gate(w, verb)
}

#[test]
fn silent_without_rules_and_nothing_opened_until_one_matches() -> TestResult {
    let w = world(&[])?;
    assert_eq!(bash(&w, "git commit -m x"), HookOutput::default());
    assert_eq!(index_files(&w), 0, "the fast path opened the index");
    rule(&w.kb, "commit", COMMIT)?;
    assert!(decided(&bash(&w, "git status && ls"))?.is_none());
    assert!(decided(&hook_at(&w, &w.code, "Read", json!({})))?.is_none());
    assert_eq!(index_files(&w), 0, "an unmatched call opened the index");
    denied(&bash(&w, "git commit -m x"))?;
    assert_eq!(index_files(&w), 1);
    Ok(())
}

#[test]
fn matching_reads_simple_commands_and_exact_tools() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    for line in [
        "git commit -m x",
        "cargo test && git commit -m x",
        "FOO=1 git commit",
        "git push origin main",
    ] {
        let reason = denied(&bash(&w, line))?;
        assert!(
            reason.contains("writfold guard rule commit")
                && reason.contains("no current KB run")
                && reason.contains("rule: .writfold/guards/commit.toml")
                && reason.matches("no current KB run").count() == 1,
            "{line}: {reason}"
        );
    }
    for line in [
        "echo \"git commit\"",
        "git commits",
        "git status",
        "gitcommit",
    ] {
        assert!(decided(&bash(&w, line))?.is_none(), "{line}");
    }
    denied(&hook_at(&w, &w.code, "Edit", json!({"file_path": "x"})))?;
    assert!(decided(&hook_at(&w, &w.code, "Write", json!({})))?.is_none());
    Ok(())
}

#[test]
fn gates_are_named_until_met() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    gate_def(&w, "unit", UNIT)?;
    gate_def(&w, "review", REVIEW)?;
    let reason = denied(&bash(&w, "git commit -m x"))?;
    assert!(
        reason.contains("gate unit [open]; next: writfold kb gate run unit")
            && reason.contains("gate review [open]; next: writfold kb gate attest review"),
        "{reason}"
    );
    assert_eq!(gate_run(&w, "unit")?, 0);
    let reason = denied(&bash(&w, "git commit -m x"))?;
    assert!(
        !reason.contains("gate unit") && reason.contains("gate review [open]"),
        "{reason}"
    );
    say(&w, Kind::Attested, "review")?;
    assert!(decided(&bash(&w, "git commit -m x"))?.is_none());
    // The content in scope moves: both are stale again.
    std::fs::write(w.code.join("src/ledger.rs"), "pub fn write() {}\n")?;
    let reason = denied(&bash(&w, "git commit -m x"))?;
    assert!(
        reason.contains("gate unit [stale]") && reason.contains("gate review [stale]"),
        "{reason}"
    );
    say(&w, Kind::Waived, "unit")?;
    say(&w, Kind::Attested, "review")?;
    assert!(decided(&bash(&w, "git commit -m x"))?.is_none());
    Ok(())
}

#[test]
fn failed_and_unknown_gates_take_the_rules_action() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    let fail = UNIT
        .replace("unit", "fail")
        .replace("\"true\"", "\"false\"");
    let crash = UNIT
        .replace("unit", "crash")
        .replace("[\"true\"]", "[\"sh\", \"-c\", \"kill -SEGV $$\"]");
    gate_def(&w, "fail", &fail)?;
    gate_def(&w, "crash", &crash)?;
    let ask = "schema = \"writfold-guard-rule/1\"\nid = \"push\"\nevent = \"pre-tool-use\"\ncommands = [\"git push\"]\nrequire = [\"gate:fail\", \"gate:crash\"]\naction = \"ask\"\n";
    rule(&w.kb, "push", ask)?;
    assert_eq!(gate_run(&w, "fail")?, 2);
    assert_eq!(gate_run(&w, "crash")?, 2);
    let (decision, reason) = decided(&bash(&w, "git push"))?.ok_or("expected a question")?;
    assert_eq!(decision, "ask");
    assert!(
        reason.contains("gate fail [failed]") && reason.contains("gate crash [unknown]"),
        "{reason}"
    );
    Ok(())
}

#[test]
fn repos_exclusions_and_repository_placement() -> TestResult {
    let w = world(&[])?;
    let excluded = COMMIT.replace("require =", "repos = [\"*\", \"!demo-service\"]\nrequire =");
    rule(&w.kb, "commit", &excluded)?;
    assert!(decided(&bash(&w, "git commit -m x"))?.is_none());
    let only = COMMIT.replace("require =", "repos = [\"demo-service\"]\nrequire =");
    rule(&w.kb, "commit", &only)?;
    denied(&bash(&w, "git commit -m x"))?;
    // A rule placed in the repository applies only under it.
    std::fs::remove_dir_all(w.kb.join(".writfold/guards"))?;
    rule(&w.code, "commit", COMMIT)?;
    let reason = denied(&bash(&w, "git commit -m x"))?;
    assert!(
        reason.contains("demo-service/.writfold/guards/commit.toml"),
        "{reason}"
    );
    let elsewhere = hook_at(&w, &w.kb, "Bash", json!({"command": "git commit"}));
    assert!(decided(&elsewhere)?.is_none());
    Ok(())
}

#[test]
fn an_unreadable_kb_after_a_match_takes_the_rules_action() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    // The local state directory is a file: the index cannot be opened.
    std::fs::remove_dir_all(w.kb.join(".git/writfold"))?;
    std::fs::write(w.kb.join(".git/writfold"), "not a directory\n")?;
    let reason = denied(&bash(&w, "git commit -m x"))?;
    assert!(reason.contains("cannot read the KB"), "{reason}");
    rule(&w.kb, "commit", &COMMIT.replace("\"deny\"", "\"ask\""))?;
    let (decision, _) = decided(&bash(&w, "git commit -m x"))?.ok_or("expected a question")?;
    assert_eq!(decision, "ask");
    Ok(())
}

#[test]
fn a_malformed_rule_is_skipped_and_check_reports_it() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", &COMMIT.replace("\"deny\"", "\"warn\""))?;
    let out = bash(&w, "git commit -m x");
    assert!(out.stdout.is_empty(), "{out:?}");
    assert!(
        out.stderr.contains(".writfold/guards/commit.toml") && out.stderr.contains("deny or ask"),
        "{out:?}"
    );
    let (text, exit) = w.run(&w.request(Verb::Check { scope: None }))?;
    assert_ne!(exit, 0, "{text}");
    assert!(
        text.contains("check guards [attention]") && text.contains("commit.toml"),
        "{text}"
    );
    Ok(())
}

fn stop(w: &World, active: bool) -> HookOutput {
    let input = event(
        &w.code,
        "Stop",
        json!({"stop_hook_active": active, "last_assistant_message": "Done."}),
    );
    let env = HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    run("stop", &input.to_string(), &env)
}

fn nudged(w: &World) -> TestResult<bool> {
    let text = std::fs::read_to_string(w.kb.join(".git/writfold/drafts/s1.json"))?;
    let value: Value = serde_json::from_str(&text)?;
    Ok(value.get("nudged").is_some())
}

#[test]
fn the_stop_nudge_comes_once_and_never_while_active() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    gate_def(&w, "unit", UNIT)?;
    put(
        &w.kb.join(".writfold/plans/change.toml"),
        "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"implement: unit\"]\n",
    )?;
    let set = PlanVerb::Set {
        template: Some("change".into()),
        from: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    assert_eq!(writfold_kb::run(&w.request(Verb::Plan(set)))?.exit, 0);
    // Only a pre-tool-use rule: the Stop hook keeps its draft and adds
    // nothing.
    rule(&w.kb, "commit", COMMIT)?;
    assert_eq!(stop(&w, false), HookOutput::default());
    assert!(!nudged(&w)?);
    let reflect = "schema = \"writfold-guard-rule/1\"\nid = \"finish-step\"\nevent = \"stop\"\nrequire = [\"step.met\"]\ndescription = \"finish the plan step before ending\"\n";
    rule(&w.kb, "finish-step", reflect)?;
    assert_eq!(stop(&w, true), HookOutput::default());
    assert!(!nudged(&w)?, "nudged while a stop hook was active");
    let out = stop(&w, false);
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "Stop");
    let text = specific["additionalContext"].as_str().ok_or("no context")?;
    assert!(
        text.contains("writfold guard rule finish-step")
            && text.contains("step implement owes unit [open]; next: writfold kb gate run unit"),
        "{text}"
    );
    assert!(value.get("decision").is_none(), "{value}");
    assert!(nudged(&w)?);
    assert_eq!(stop(&w, false), HookOutput::default(), "nudged twice");
    Ok(())
}

#[test]
fn rule_files_refuse_what_they_cannot_mean() {
    use writfold_kb::guard::rules::parse;
    let path = ".writfold/guards/commit.toml";
    assert!(parse(COMMIT, "commit", path).is_ok());
    for bad in [
        COMMIT.replace("commit\"\nevent", "other\"\nevent"),
        COMMIT.replace("\"deny\"", "\"warn\""),
        COMMIT.replace("run.current", "run.open"),
        COMMIT.replace("\"gates.met\"", "\"run.current\""),
        COMMIT.replace("\"gates.met\"", "\"gate:Bad Id\""),
        COMMIT.replace("require = [\"run.current\", \"gates.met\"]", "require = []"),
        COMMIT.replace("pre-tool-use", "post-tool-use"),
        format!("{COMMIT}paths = [\"*.rs\"]\n"),
        COMMIT.replace("writfold-guard-rule/1", "writfold-guard/1"),
        COMMIT.replace("require =", "repos = [\"a/b\"]\nrequire ="),
        "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\nrequire = [\"run.current\"]\naction = \"deny\"\n".into(),
        "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"stop\"\nrequire = [\"step.met\"]\naction = \"deny\"\n".into(),
        "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"stop\"\ncommands = [\"git commit\"]\nrequire = [\"step.met\"]\n".into(),
    ] {
        assert!(parse(&bad, "commit", path).is_err(), "accepted {bad:?}");
    }
}
