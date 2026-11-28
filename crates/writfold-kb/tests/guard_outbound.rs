//! The outbound remainder (#187): under an `unclassified` policy every
//! simple command and tool call of a call is classified. A command no
//! route, list or `local` entry names passes under `pass` and `log`, asks
//! under `ask` and is denied under `deny`, with the two ways out; an opaque
//! program is unclassified; `log` keeps the command in the guard log; a
//! line takes its strictest member; a route reads a positional body file.
#[path = "delivery_round/helpers.rs"]
mod helpers;
mod support;

use helpers::{CLEAN, check, file, hook, prepare, show};
use serde_json::{Value, json};
use support::{NOW, TestResult, World};
use writfold_kb::Verb;
use writfold_kb::guard::GuardVerb;
use writfold_kb::hook::HookOutput;

const PR: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nchecks = [\"paragraphs.bounded\"]\n";

fn guard_toml(policy: Option<&str>) -> String {
    let mut text = String::from(
        "schema = \"writfold-guard/1\"\nroutes = [\n  \"gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F\",\n  \"gh gist create: mode=pr-comment file=@last flags=--desc,-d\",\n]\nlocal = [\"ls\", \"cargo test\", \"git status\"]\nlocal_tools = [\"Edit\"]\n",
    );
    if let Some(p) = policy {
        text.push_str(&format!("unclassified = \"{p}\"\n"));
    }
    text
}

fn world(policy: Option<&str>) -> TestResult<World> {
    helpers::kb(&[("guard.toml", &guard_toml(policy)), ("pr-comment.toml", PR)])
}

/// The decision (`allow` when silent) and the reason.
fn decided(out: &HookOutput) -> TestResult<(String, String)> {
    if out.stdout.is_empty() {
        return Ok(("allow".into(), String::new()));
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    let d = specific["permissionDecision"].as_str().ok_or("decision")?;
    let r = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?;
    Ok((d.into(), r.into()))
}

fn bash(w: &World, line: &str) -> TestResult<(String, String)> {
    decided(&hook(w, "Bash", json!({"command": line})))
}

#[test]
fn both_reproductions_follow_the_policy() -> TestResult {
    for line in [
        "gh gist new notes.md",
        "bash -c \"git commit -m x\"",
        "eval \"$(cat x)\"",
    ] {
        for (policy, want) in [
            ("pass", "allow"),
            ("log", "allow"),
            ("ask", "ask"),
            ("deny", "deny"),
        ] {
            let w = world(Some(policy))?;
            let (decision, reason) = bash(&w, line)?;
            assert_eq!(decision, want, "{line} under {policy}: {reason}");
            if want != "allow" {
                assert!(
                    reason.contains("route it") && reason.contains("add it to local"),
                    "{line}: {reason}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn without_a_policy_nothing_changes_and_local_calls_never_refuse() -> TestResult {
    let w = world(None)?;
    assert_eq!(bash(&w, "gh gist new notes.md")?.0, "allow");
    let w = world(Some("deny"))?;
    assert_eq!(bash(&w, "ls -la && cargo test && git status")?.0, "allow");
    assert_eq!(
        decided(&hook(&w, "Edit", json!({"file_path": "x"})))?.0,
        "allow"
    );
    assert_eq!(
        decided(&hook(&w, "Write", json!({"file_path": "x"})))?.0,
        "deny"
    );
    Ok(())
}

#[test]
fn a_line_takes_its_strictest_member() -> TestResult {
    let w = world(Some("ask"))?;
    let (decision, reason) = bash(&w, "ls && curl -X POST example.org")?;
    assert_eq!(decision, "ask", "{reason}");
    // A routed member with no candidate is denied, which wins over the ask.
    let (decision, _) = bash(
        &w,
        "ls && gh pr comment 1 --body hello && curl -X POST example.org",
    )?;
    assert_eq!(decision, "deny");
    Ok(())
}

#[test]
fn log_passes_and_guard_log_lists_the_command() -> TestResult {
    let w = world(Some("log"))?;
    assert_eq!(bash(&w, "curl -X POST example.org")?.0, "allow");
    assert_eq!(bash(&w, "curl -X POST example.org")?.0, "allow");
    let (text, _) = w.run(&w.request(Verb::Guard(GuardVerb::Log {
        rule: None,
        since: None,
    })))?;
    assert!(text.contains("curl -X POST seen=2"), "{text}");
    assert!(text.contains("unclassified=2"), "{text}");
    Ok(())
}

#[test]
fn a_positional_body_file_matches_a_ready_candidate() -> TestResult {
    let w = world(None)?;
    let body = file(&w, "notes.md", CLEAN)?;
    let line = format!("gh gist create --desc summary {}", show(&body));
    let (decision, reason) = bash(&w, &line)?;
    assert_eq!(decision, "deny", "no candidate yet: {reason}");
    let id = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    assert_eq!(bash(&w, &line)?.0, "allow");
    let unlisted = format!("gh gist create --public yes {}", show(&body));
    let (decision, reason) = bash(&w, &unlisted)?;
    assert_eq!(decision, "deny");
    assert!(
        reason.contains("--public is not in the route's flags="),
        "{reason}"
    );
    Ok(())
}
