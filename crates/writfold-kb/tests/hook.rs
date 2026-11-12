//! `writfold kb hook <event>` over the fixture: what each event prints and
//! keeps, confirming a draft, and every failure mode leaving stdout empty
//! (the hook always exits 0). `WRITFOLD_KB_BLESS=1` rewrites the goldens.
mod support;

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::hook::{CONTEXT_LIMIT, HookEnv, HookOutput, cap, run};
use writfold_kb::write::Write;
use writfold_kb::write::run::FromDraft;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn golden(name: &str, actual: &str) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

fn env(w: &World) -> HookEnv {
    HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.kb.clone(),
        now: Some(NOW),
    }
}

fn hook(w: &World, event: &str, input: &Value) -> HookOutput {
    let mut out = run(event, &input.to_string(), &env(w));
    let base = w.base.display().to_string();
    out.stdout = out.stdout.replace(&base, "<WORLD>");
    out
}

/// The injected context of a `session-start` output, checked for shape.
fn context(out: &HookOutput) -> TestResult<String> {
    let value: Value = serde_json::from_str(&out.stdout)?;
    let inner = &value["hookSpecificOutput"];
    assert_eq!(inner["hookEventName"], "SessionStart");
    let text = inner["additionalContext"]
        .as_str()
        .ok_or("no additionalContext")?;
    assert!(text.chars().count() <= CONTEXT_LIMIT);
    Ok(text.to_owned())
}

fn start(cwd: &Path) -> Value {
    json!({"session_id": "s-1", "hook_event_name": "SessionStart",
           "source": "startup", "cwd": cwd})
}

fn state(w: &World) -> PathBuf {
    w.kb.join(".git/writfold")
}

fn draft(w: &World, session: &str) -> TestResult<Value> {
    let text = std::fs::read_to_string(state(w).join(format!("drafts/{session}.json")))?;
    Ok(serde_json::from_str(&text)?)
}

#[test]
fn session_start_injects_the_brief() -> TestResult {
    let w = world(&[])?;
    let out = hook(&w, "session-start", &start(&w.kb.join("harbor/log")));
    assert_eq!(out.stderr, "");
    golden("hook-session-start.txt", &context(&out)?)
}

#[test]
fn drafts_from_stop_compaction_and_end_then_confirm() -> TestResult {
    let w = world(&[])?;
    std::fs::write(state(&w).join("current-run"), format!("{RUN}\n"))?;
    let pre = hook(
        &w,
        "pre-compact",
        &json!({"session_id": "s-1", "hook_event_name": "PreCompact",
                "trigger": "auto", "custom_instructions": null, "cwd": w.kb}),
    );
    golden("hook-pre-compact.txt", &pre.stdout)?;
    let place = &draft(&w, "s-1")?["place"];
    assert_eq!(
        (&place["repo"], &place["branch"]),
        (&json!("kb"), &json!("main"))
    );
    assert_eq!(place["head"].as_str().map(str::len), Some(40));
    let message = "## Done\nReplay measured at 41 s.\nNext: close HBR-101.";
    let stop = json!({"session_id": "s-1", "hook_event_name": "Stop",
                      "stop_hook_active": false, "last_assistant_message": message});
    assert_eq!(hook(&w, "stop", &stop), HookOutput::default());
    assert_eq!(draft(&w, "s-1")?["last_message"], message);
    let post = json!({"session_id": "s-1", "hook_event_name": "PostCompact",
                      "trigger": "auto", "compact_summary": "<analysis>\nworking\n</analysis>\n\n<summary>\nSummary: replay is 41 s.\n</summary>"});
    assert_eq!(hook(&w, "post-compact", &post), HookOutput::default());
    let end = json!({"session_id": "s-1", "hook_event_name": "SessionEnd", "reason": "logout"});
    assert_eq!(hook(&w, "session-end", &end), HookOutput::default());
    // Drafts of no run (shown by resume) and of another run (listed apart).
    std::fs::write(state(&w).join("current-run"), "")?;
    let none = json!({"session_id": "s-0", "last_assistant_message": "Read the KB."});
    assert_eq!(hook(&w, "stop", &none), HookOutput::default());
    std::fs::write(state(&w).join("current-run"), "run-elsewhere\n")?;
    let other = json!({"session_id": "s-2", "last_assistant_message": "Started on lantern."});
    assert_eq!(hook(&w, "stop", &other), HookOutput::default());
    std::fs::write(state(&w).join("current-run"), format!("{RUN}\n"))?;
    let mut d = draft(&w, "s-1")?;
    d["place"]["head"] = json!("<HEAD>");
    d["place"]["worktree"] = json!("<WORLD>/kb");
    d["cwd"] = json!("<WORLD>/kb");
    golden(
        "hook-draft.json",
        &format!("{}\n", serde_json::to_string_pretty(&d)?),
    )?;
    let out = hook(&w, "session-start", &start(&w.kb.join("harbor/log")));
    golden("hook-session-start-run.txt", &context(&out)?)?;
    confirm(&w)
}

fn confirm(w: &World) -> TestResult {
    let from = |session: &str| {
        Verb::Write(Write::FromDraft(FromDraft {
            session: session.into(),
            run: None,
            new_run: None,
        }))
    };
    let (text, exit) = w.run(&w.request(from("s-1")))?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("draft s-1 confirmed and removed"), "{text}");
    let file = std::fs::read_to_string(w.kb.join(format!("runs/{RUN}/checkpoint.md")))?;
    let entry = "\n## 2026-11-11T00:00:00Z\n\nSummary: replay is 41 s.\n\n\
                 (draft from session s-1 repo=kb branch=main head=";
    assert!(file.contains(entry) && file.ends_with(")\n"), "{file}");
    assert!(!state(w).join("drafts/s-1.json").exists());
    let again = writfold_kb::run(&w.request(from("s-1")));
    assert!(matches!(again, Err(e) if e.exit_code() == 3));
    // A draft holding only mechanical facts has nothing to confirm.
    let pre = json!({"session_id": "s-3", "cwd": w.kb});
    assert!(!hook(w, "pre-compact", &pre).stdout.is_empty());
    let empty = writfold_kb::run(&w.request(from("s-3")));
    assert!(matches!(empty, Err(e) if e.to_string().contains("no text yet")));
    let (text, _) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(!text.contains("[differs]"), "{text}");
    Ok(())
}

/// Every failure leaves stdout empty and says why on stderr only.
#[test]
fn failures_are_silent() -> TestResult {
    let w = world(&[])?;
    let quiet = |out: HookOutput, why: &str| -> TestResult {
        assert_eq!(out.stdout, "", "{why}");
        assert!(
            out.stderr.starts_with("writfold kb hook: "),
            "{why}: {}",
            out.stderr
        );
        Ok(())
    };
    let stop = json!({"session_id": "s-9", "last_assistant_message": "x"});
    quiet(run("session-begin", "{}", &env(&w)), "unknown event")?;
    quiet(run("stop", "", &env(&w)), "empty stdin")?;
    quiet(run("stop", "not json", &env(&w)), "bad JSON")?;
    quiet(run("stop", "[1]", &env(&w)), "not an object")?;
    let wrong = json!({"session_id": "s-9", "hook_event_name": "PreCompact"});
    quiet(hook(&w, "stop", &wrong), "event mismatch")?;
    let mut missing = env(&w);
    missing.kb = Some(w.base.join("nowhere"));
    quiet(run("session-start", &stop.to_string(), &missing), "no KB")?;
    quiet(
        run("stop", &stop.to_string(), &missing),
        "no KB for a draft",
    )?;
    quiet(
        hook(&w, "stop", &json!({"last_assistant_message": "x"})),
        "no session",
    )?;
    let bad = json!({"session_id": "../escape", "last_assistant_message": "x"});
    quiet(hook(&w, "stop", &bad), "unsafe session")?;
    quiet(
        hook(&w, "stop", &json!({"session_id": "s-9"})),
        "no message",
    )?;
    let blank = json!({"session_id": "s-9", "compact_summary": "  "});
    quiet(hook(&w, "post-compact", &blank), "no summary")?;
    let end = json!({"session_id": "s-9", "reason": "other"});
    quiet(hook(&w, "session-end", &end), "no draft to end")?;
    assert!(
        !state(&w).join("drafts").exists(),
        "no failure wrote a draft"
    );
    // A KB outside Git: the brief still comes; drafts cannot be kept.
    let plain = w.base.join("plain");
    std::fs::create_dir_all(plain.join("inventory/assets"))?;
    let mut outside = env(&w);
    outside.kb = Some(plain.clone());
    quiet(run("stop", &stop.to_string(), &outside), "KB not under Git")?;
    let started = run("session-start", &start(&plain).to_string(), &outside);
    assert!(context(&started)?.contains("kb=none"));
    Ok(())
}

#[test]
fn the_context_cap_cuts_at_a_line() {
    let text = "row\n".repeat(5000);
    let cut = cap(&text, CONTEXT_LIMIT);
    assert!(cut.chars().count() <= CONTEXT_LIMIT);
    assert!(cut.ends_with("run the verbs above for the rest]\n"));
    assert!(cut.lines().rev().nth(1) == Some("row"));
    assert_eq!(cap("short\n", CONTEXT_LIMIT), "short\n");
}
