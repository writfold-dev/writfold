//! A stop rule that blocks (#184): `action = "block"` with `limit = N`
//! ends a turn with `decision: "block"` while its requirement is unmet, at
//! most N times a turn; a session with no run is unmet under
//! `run.current`; `guard check --event stop` is the same decision.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::guard::check::{Args, run as check};
use writfold_kb::guard::rules;
use writfold_kb::hook::{HookEnv, HookOutput, run};

const BLOCK: &str = "schema = \"writfold-guard-rule/1\"\nid = \"finish\"\nevent = \"stop\"\nrequire = [\"run.current\"]\naction = \"block\"\nlimit = 2\n";

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn env(w: &World) -> HookEnv {
    HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    }
}

fn stop(w: &World, active: bool) -> HookOutput {
    let input = json!({"session_id": "s1", "cwd": w.code.display().to_string(),
        "hook_event_name": "Stop", "stop_hook_active": active,
        "last_assistant_message": "Done."});
    run("stop", &input.to_string(), &env(w))
}

fn blocked(out: &HookOutput) -> TestResult<Option<String>> {
    if out.stdout.is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    if value["decision"] != "block" {
        return Ok(None);
    }
    Ok(value["reason"].as_str().map(str::to_owned))
}

fn draft(w: &World) -> TestResult<Value> {
    let text = std::fs::read_to_string(w.kb.join(".git/writfold/drafts/s1.json"))?;
    Ok(serde_json::from_str(&text)?)
}

#[test]
fn a_blocking_rule_blocks_at_most_its_limit_a_turn() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/finish.toml"), BLOCK)?;
    let first = blocked(&stop(&w, false))?.ok_or("the first stop of a turn with no run blocks")?;
    assert!(
        first.contains("writfold guard rule finish") && first.contains("no current KB run"),
        "{first}"
    );
    assert!(first.contains("block 1 of 2 this turn"), "{first}");
    let second = blocked(&stop(&w, true))?.ok_or("the second stop blocks")?;
    assert!(second.contains("block 2 of 2"), "{second}");
    assert_eq!(blocked(&stop(&w, true))?, None, "the limit is spent");
    let spent = draft(&w)?;
    let line = spent["stop_block_spent"].as_str().ok_or("spent recorded")?;
    assert!(
        line.contains("finish (.writfold/guards/finish.toml)"),
        "{line}"
    );
    assert!(
        blocked(&stop(&w, false))?.is_some(),
        "a new turn starts at zero"
    );
    Ok(())
}

#[test]
fn check_event_stop_is_the_same_decision() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/finish.toml"), BLOCK)?;
    let args = Args {
        stop: true,
        session: Some("s1".into()),
        cwd: Some(w.code.display().to_string()),
        ..Args::default()
    };
    let out = check(&args, &env(&w));
    assert_eq!(out.exit, 3, "{} {}", out.stdout, out.stderr);
    assert!(
        out.stdout
            .starts_with("stop [block] writfold guard rule finish"),
        "{}",
        out.stdout
    );
    let bare = Args {
        stop: true,
        ..Args::default()
    };
    assert_eq!(check(&bare, &env(&w)).exit, 1);
    Ok(())
}

#[test]
fn a_stop_rule_takes_only_block_and_a_limit_in_range() -> TestResult {
    let path = ".writfold/guards/finish.toml";
    let base = "schema = \"writfold-guard-rule/1\"\nid = \"finish\"\nevent = \"stop\"\nrequire = [\"run.current\"]\n";
    let ok = rules::parse(&format!("{base}action = \"block\"\n"), "finish", path)
        .map_err(|r| writfold_kb::defs::why(&r))?;
    assert_eq!(ok.block, Some(1));
    assert_eq!(
        rules::parse(base, "finish", path)
            .map_err(|r| writfold_kb::defs::why(&r))?
            .block,
        None
    );
    for (extra, want) in [
        ("action = \"deny\"\n", "a stop rule's action is \"block\""),
        ("action = \"block\"\nlimit = 9\n", "limit"),
        ("limit = 2\n", "`limit` needs action = \"block\""),
    ] {
        let Err(report) = rules::parse(&format!("{base}{extra}"), "finish", path) else {
            return Err(format!("accepted: {extra}").into());
        };
        let said: String = report
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert!(said.contains(want), "{extra}: {said}");
    }
    Ok(())
}
