//! Cited evidence that moved: a passed `claims.cited-fresh` receipt stands
//! only while every citation token in the candidate is still fresh, on
//! every read. Once the cited code moves, `deliver status` reads the
//! candidate `[blocked]` with the check line saying so, the plan step owes
//! its delivery, and the delivery guard refuses its bytes; moving the code
//! back restores all three. A waiver on the check still stands.
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{RUN, at_delivery, check, deliver_status, prepare, row, say, status};
use serde_json::{Value, json};
use support::{NOW, TestResult, World, git};
use writfold_kb::gate::statement::Kind;
use writfold_kb::hook::{HookEnv, run};

/// The fixture's code citation token for `src/ledger.rs`.
fn token(w: &World) -> TestResult<String> {
    let text = std::fs::read_to_string(
        w.kb.join("proposals/decision/2026-03-02-batch-ledger-writes/proposal.md"),
    )?;
    let re = regex::Regex::new(r"⟦cite kind=code[^⟧]*⟧")?;
    re.find_iter(&text)
        .map(|m| m.as_str().to_owned())
        .find(|t| t.contains("path=src/ledger.rs "))
        .ok_or_else(|| "no token for src/ledger.rs".into())
}

fn payload(w: &World) -> TestResult<String> {
    Ok(format!("The write loop {} is unchanged.\n", token(w)?))
}

/// Commit `text` as the code repository's `src/ledger.rs`.
fn ledger(w: &World, text: &str) -> TestResult {
    std::fs::write(w.code.join("src/ledger.rs"), text)?;
    git(&w.code, &["commit", "-qam", "ledger"])?;
    Ok(())
}

fn guard_denies(w: &World, body: &str) -> TestResult<bool> {
    std::fs::create_dir_all(w.kb.join(".git/writfold"))?;
    writfold_kb::runs::set_current(&w.kb.join(".git/writfold"), RUN)?;
    let guard = "schema = \"writfold-guard/1\"\ncommands = [\"gh pr comment\"]\n";
    helpers::put(w, ".writfold/deliveries/guard.toml", guard)?;
    let file = w.base.join("comment.md");
    std::fs::write(&file, body)?;
    let event = json!({"session_id": "s1", "cwd": w.base, "hook_event_name": "PreToolUse",
        "tool_name": "Bash", "tool_use_id": "t1",
        "tool_input": {"command": format!("gh pr comment 42 --body-file {}", file.display())}});
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    let out = run("pre-tool-use", &event.to_string(), &env);
    if out.stdout.is_empty() {
        return Ok(false);
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    Ok(value["hookSpecificOutput"]["permissionDecision"] == "deny")
}

#[test]
fn moved_evidence_blocks_the_candidate_everywhere_until_it_returns() -> TestResult {
    let w = at_delivery()?;
    let body = payload(&w)?;
    let id = prepare(&w, "pr-comment", &body, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let (text, exit) = deliver_status(&w)?;
    assert!(
        exit == 0 && text.contains(&format!("delivery {id} [ready]")),
        "{text}"
    );
    assert!(
        !guard_denies(&w, &body)?,
        "a ready candidate's bytes were refused"
    );
    let original = std::fs::read_to_string(w.code.join("src/ledger.rs"))?;
    ledger(&w, "// ledger writes, rewritten\npub fn write() {}\n")?;
    let (text, exit) = deliver_status(&w)?;
    assert!(
        exit == 2
            && text.contains(&format!("delivery {id} [blocked]"))
            && text.contains("claims.cited-fresh [stale] observed pass for ")
            && text.contains("; cited evidence moved: 1 of 1 citation tokens no longer fresh @")
            && text.contains(".json:1 !stale"),
        "{text}"
    );
    let (text, _) = status(&w, false)?;
    let step = row(&text, "step change#1/deliver");
    assert!(
        step[0].contains("owes: deliver:pr-comment [stale]"),
        "{text}"
    );
    assert!(
        step.iter()
            .any(|l| l.contains("owes claims.cited-fresh [stale]")),
        "{text}"
    );
    assert!(guard_denies(&w, &body)?, "the guard passed moved evidence");
    ledger(&w, &original)?;
    let (text, exit) = deliver_status(&w)?;
    assert!(
        exit == 0 && text.contains(&format!("delivery {id} [ready]")),
        "{text}"
    );
    let (text, _) = status(&w, false)?;
    assert!(
        text.contains("met: deliver:pr-comment [satisfied]"),
        "{text}"
    );
    assert!(!guard_denies(&w, &body)?);
    Ok(())
}

#[test]
fn a_waived_check_stands_when_its_evidence_moves() -> TestResult {
    let w = at_delivery()?;
    let body = payload(&w)?;
    let id = prepare(&w, "pr-comment", &body, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    say(&w, Kind::Waived, &id, "claims.cited-fresh", NOW + 180)?;
    ledger(&w, "// ledger writes, rewritten\npub fn write() {}\n")?;
    let (text, exit) = deliver_status(&w)?;
    assert!(
        exit == 0
            && text.contains(&format!("delivery {id} [ready]"))
            && text.contains("claims.cited-fresh [waived]"),
        "{text}"
    );
    let (text, _) = status(&w, false)?;
    assert!(text.contains("met: deliver:pr-comment [waived]"), "{text}");
    Ok(())
}
