//! #26: session start names the records its surroundings already name
//! (the working branch's ticket-shaped words, the run's task, the
//! repository's name), from the exact tier only, before the brief is cut.
mod support;

use serde_json::{Value, json};
use support::{NOW, TestResult, World, git, world};
use writfold_kb::hook::{HookEnv, run};

fn start(w: &World) -> TestResult<String> {
    let input = json!({"session_id": "s-new", "hook_event_name": "SessionStart",
                       "source": "startup", "cwd": w.code});
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.kb.clone(),
        now: Some(NOW),
    };
    let out = run("session-start", &input.to_string(), &env);
    let value: Value = serde_json::from_str(&out.stdout)?;
    let text = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("no additionalContext")?;
    Ok(text.to_owned())
}

#[test]
fn a_ticket_in_the_branch_name_is_named() -> TestResult {
    let w = world(&[])?;
    let before = start(&w)?;
    assert!(!before.contains("named by branch"), "{before}");
    git(
        &w.code,
        &["checkout", "-q", "-b", "feature/hbr-101-month-end"],
    )?;
    let after = start(&w)?;
    assert!(after.contains("Named here"), "{after}");
    assert!(
        after.contains("ticket HBR-101 [in-progress] Slow ledger flush at month end"),
        "{after}"
    );
    assert!(
        after.contains("(named by branch feature/hbr-101-month-end)"),
        "{after}"
    );
    // A branch naming no record adds no row for it.
    git(&w.code, &["checkout", "-q", "-b", "feature/zzq-999"])?;
    assert!(!start(&w)?.contains("named by branch"));
    Ok(())
}
