//! The guard site's posture (#184): without `.writfold/guards.toml` or
//! `--posture` nothing changes; under `ask` or `deny` each undecided cause
//! gets that decision with a reason naming it; a judged decision is the
//! same under every posture; the effective posture is the stricter of the
//! file's and the registration's.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::guard::check::{Args, Posture, run as check};
use writfold_kb::hook::{HookEnv, HookOutput, run, run_with};

const COMMIT: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"run.current\"]\naction = \"deny\"\n";
const BROKEN: &str = "schema = \"writfold-guard-rule/1\"\nid = \"broken\"\nevent = \"pre-tool-use\"\ncommands = [\"git push\"]\nrequire = [\"gates.mett\"]\naction = \"deny\"\n";
const SITE: &str = "schema = \"writfold-guard-site/1\"\n";

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn env_at(w: &World, kb: &Path) -> HookEnv {
    HookEnv {
        home: None,
        kb: Some(kb.to_path_buf()),
        cwd: w.code.clone(),
        now: Some(NOW + 3600),
    }
}

fn event(w: &World, command: &str) -> String {
    json!({"session_id": "s1", "cwd": w.code.display().to_string(),
        "hook_event_name": "PreToolUse", "tool_name": "Bash",
        "tool_input": {"command": command}})
    .to_string()
}

fn hook(w: &World, input: &str, posture: Posture) -> HookOutput {
    run_with("pre-tool-use", input, &env_at(w, &w.kb), posture)
}

/// The decision (`allow` when silent) and the reason.
fn decided(out: &HookOutput) -> TestResult<(String, String)> {
    if out.stdout.is_empty() {
        return Ok(("allow".into(), String::new()));
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    let decision = specific["permissionDecision"].as_str().ok_or("decision")?;
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?;
    Ok((decision.into(), reason.into()))
}

fn word(p: Posture) -> &'static str {
    match p {
        Posture::Open => "allow",
        Posture::Ask => "ask",
        Posture::Deny => "deny",
    }
}

/// Under each posture, `input` gets the posture's word and a reason
/// naming `cause`; under `open` it passes as before.
fn undecided_by(w: &World, input: &str, cause: &str) -> TestResult {
    let open = hook(w, input, Posture::Open);
    assert_eq!(decided(&open)?.0, "allow", "{open:?}");
    assert_eq!(open, run("pre-tool-use", input, &env_at(w, &w.kb)));
    for posture in [Posture::Ask, Posture::Deny] {
        let (decision, reason) = decided(&hook(w, input, posture))?;
        assert_eq!(decision, word(posture), "{cause}: {reason}");
        assert!(
            reason.contains(cause) && reason.contains(&format!("posture {}", posture.name())),
            "{cause}: {reason}"
        );
        assert!(reason.contains("posture = \"open\""), "{reason}");
    }
    Ok(())
}

#[test]
fn an_unparsable_rule_is_undecided_under_a_closed_posture() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/broken.toml"), BROKEN)?;
    undecided_by(&w, &event(&w, "git status"), "broken.toml")
}

#[test]
fn an_unparsable_site_file_is_undecided_under_the_registration() -> TestResult {
    let w = world(&[])?;
    put(
        &w.kb.join(".writfold/guards.toml"),
        "schema = \"writfold-guard-site/1\"\nposture = \"shut\"\n",
    )?;
    undecided_by(&w, &event(&w, "git status"), "unknown posture")
}

#[test]
fn a_required_rule_that_did_not_load_is_undecided() -> TestResult {
    let w = world(&[])?;
    put(
        &w.kb.join(".writfold/guards.toml"),
        &format!("{SITE}required = [\"commit\"]\n"),
    )?;
    undecided_by(
        &w,
        &event(&w, "git status"),
        "required rule commit did not load",
    )?;
    put(&w.kb.join(".writfold/guards/commit.toml"), COMMIT)?;
    assert_eq!(
        decided(&hook(&w, &event(&w, "git status"), Posture::Deny))?.0,
        "allow"
    );
    Ok(())
}

#[test]
fn no_kb_and_input_that_is_not_an_event_are_undecided() -> TestResult {
    let w = world(&[])?;
    undecided_by(&w, "not json", "stdin is not a JSON object")?;
    let gone = w.base.join("no-such-kb");
    for posture in [Posture::Ask, Posture::Deny] {
        let out = run_with(
            "pre-tool-use",
            &event(&w, "ls"),
            &env_at(&w, &gone),
            posture,
        );
        assert_eq!(decided(&out)?.0, word(posture), "{out:?}");
    }
    let open = run_with(
        "pre-tool-use",
        &event(&w, "ls"),
        &env_at(&w, &gone),
        Posture::Open,
    );
    assert!(open.stdout.is_empty(), "{open:?}");
    Ok(())
}

#[test]
fn a_judged_decision_is_the_same_under_every_posture() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/commit.toml"), COMMIT)?;
    for command in ["git commit -m x", "git status"] {
        let open = hook(&w, &event(&w, command), Posture::Open);
        for posture in [Posture::Ask, Posture::Deny] {
            assert_eq!(hook(&w, &event(&w, command), posture), open, "{command}");
        }
    }
    Ok(())
}

#[test]
fn the_effective_posture_is_the_stricter_of_file_and_registration() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/broken.toml"), BROKEN)?;
    put(
        &w.kb.join(".writfold/guards.toml"),
        &format!("{SITE}posture = \"ask\"\n"),
    )?;
    let input = event(&w, "git status");
    assert_eq!(decided(&hook(&w, &input, Posture::Open))?.0, "ask");
    assert_eq!(decided(&hook(&w, &input, Posture::Deny))?.0, "deny");
    let args = Args {
        kind: Some("shell".into()),
        command: Some("git status".into()),
        ..Args::default()
    };
    let out = check(&args, &env_at(&w, &w.kb));
    assert_eq!(out.exit, 2, "{}", out.stdout);
    assert!(
        out.stdout.contains("basis=undecided posture=ask"),
        "{}",
        out.stdout
    );
    let bare = Args {
        kind: Some("shell".into()),
        posture: Some("deny".into()),
        ..Args::default()
    };
    assert_eq!(
        check(&bare, &env_at(&w, &w.kb)).exit,
        3,
        "a usage error under deny"
    );
    Ok(())
}
