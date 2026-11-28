//! `writfold kb guard check` (#183): the hook's decision for any harness.
//! For a table of events the hook and `guard check` agree; a shell call
//! under a binding's other shell tool name is matched by a `commands`
//! rule; the exit code is the decision; the guard log says `via: check`;
//! a binding file's faults are reported at their line and column.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::guard::binding;
use writfold_kb::guard::check::{Args, Checked, run as check};
use writfold_kb::hook::{HookEnv, HookOutput, run};

const COMMIT: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\", \"git push\"]\ntools = [\"Edit\"]\nrequire = [\"run.current\", \"gates.met\"]\naction = \"deny\"\ndescription = \"commit only when the run's gates are met\"\n";
const ASK: &str = "schema = \"writfold-guard-rule/1\"\nid = \"post\"\nevent = \"pre-tool-use\"\ntools = [\"mcp__slack__post\"]\nrequire = [\"run.current\"]\naction = \"ask\"\n";
const EDITS: &str = "schema = \"writfold-guard-rule/1\"\nid = \"edits\"\nevent = \"pre-tool-use\"\ntools = [\"Write\"]\npaths = [\"**\", \"!**/*.md\"]\nrequire = [\"run.current\"]\naction = \"deny\"\n";
const HARNESS: &str = "schema = \"writfold-harness/1\"\nid = \"other\"\nsession = \"sessionId\"\ntool = \"toolName\"\ninput = \"args\"\nshell = [\"bash: command\", \"terminal: commandLine\"]\nfile_write = [\"edit: path\"]\nreports = [\"shell\", \"file.write\"]\n";

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn rules(w: &World) -> TestResult {
    put(&w.kb.join(".writfold/guards/commit.toml"), COMMIT)?;
    put(&w.kb.join(".writfold/guards/post.toml"), ASK)?;
    put(&w.kb.join(".writfold/guards/edits.toml"), EDITS)
}

fn env(w: &World) -> HookEnv {
    HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.code.clone(),
        now: Some(NOW + 3600),
    }
}

fn claude(w: &World, tool: &str, input: Value) -> Value {
    json!({"session_id": "s1", "cwd": w.code.display().to_string(),
        "hook_event_name": "PreToolUse", "tool_name": tool, "tool_input": input})
}

fn hook(w: &World, event: &Value) -> HookOutput {
    run("pre-tool-use", &event.to_string(), &env(w))
}

fn from_event(w: &World, event: &Value, harness: Option<&str>) -> Checked {
    let args = Args {
        from_event: Some(event.to_string()),
        harness: harness.map(str::to_owned),
        json: true,
        ..Args::default()
    };
    check(&args, &env(w))
}

/// The hook's decision and reasons (`allow` when silent).
fn hooked(out: &HookOutput) -> TestResult<(String, Vec<String>)> {
    if out.stdout.is_empty() {
        return Ok(("allow".into(), Vec::new()));
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("no reason")?;
    let decision = specific["permissionDecision"]
        .as_str()
        .ok_or("no decision")?;
    Ok((decision.into(), reason.lines().map(str::to_owned).collect()))
}

fn checked(out: &Checked) -> TestResult<(String, Vec<String>)> {
    let value: Value = serde_json::from_str(&out.stdout)?;
    let decision = value["decision"].as_str().ok_or("no decision")?.to_owned();
    let reasons = value["reasons"].as_array().ok_or("no reasons")?;
    let reasons = reasons
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let exit = match decision.as_str() {
        "allow" => 0,
        "ask" => 2,
        _ => 3,
    };
    assert_eq!(out.exit, exit, "{}", out.stdout);
    Ok((decision, reasons))
}

#[test]
fn hook_and_check_agree_on_a_table_of_events() -> TestResult {
    let w = world(&[])?;
    rules(&w)?;
    let code = w.code.display().to_string();
    let table = [
        ("Bash", json!({"command": "git commit -m x"})),
        ("Bash", json!({"command": "cargo test && git commit -m x"})),
        ("Bash", json!({"command": "env FOO=1 git push origin main"})),
        (
            "Bash",
            json!({"command": format!("git -C {code} commit -m x")}),
        ),
        ("Bash", json!({"command": "git status"})),
        ("Bash", json!({"command": "echo \"git commit\""})),
        ("Edit", json!({"file_path": "src/a.rs"})),
        ("Write", json!({"file_path": format!("{code}/src/a.rs")})),
        ("Write", json!({"file_path": format!("{code}/README.md")})),
        ("mcp__slack__post", json!({"text": "hi"})),
        ("Read", json!({"file_path": "src/a.rs"})),
    ];
    let mut seen = Vec::new();
    for (tool, input) in table {
        let event = claude(&w, tool, input.clone());
        let by_hook = hooked(&hook(&w, &event))?;
        let by_check = checked(&from_event(&w, &event, None))?;
        assert_eq!(by_hook, by_check, "{tool} {input}");
        seen.push(by_hook.0);
    }
    for d in ["allow", "ask", "deny"] {
        assert!(
            seen.iter().any(|s| s == d),
            "the table never reached {d}: {seen:?}"
        );
    }
    Ok(())
}

#[test]
fn flags_describe_the_same_event() -> TestResult {
    let w = world(&[])?;
    rules(&w)?;
    let shell = Args {
        kind: Some("shell".into()),
        command: Some("git commit -m x".into()),
        json: true,
        ..Args::default()
    };
    let by_flags = checked(&check(&shell, &env(&w)))?;
    let by_hook = hooked(&hook(
        &w,
        &claude(&w, "Bash", json!({"command": "git commit -m x"})),
    ))?;
    assert_eq!(by_flags, by_hook);
    let tool = Args {
        kind: Some("tool".into()),
        tool: Some("mcp__slack__post".into()),
        ..Args::default()
    };
    let out = check(&tool, &env(&w));
    assert_eq!(out.exit, 2, "{}", out.stdout);
    assert!(out.stdout.contains("guard post [ask]"), "{}", out.stdout);
    assert!(
        out.stdout
            .contains("# decision=ask basis=judged posture=open"),
        "{}",
        out.stdout
    );
    let bare = Args {
        kind: Some("shell".into()),
        ..Args::default()
    };
    let out = check(&bare, &env(&w));
    assert_eq!(out.exit, 1);
    assert!(
        out.stderr.contains("--kind shell needs --command"),
        "{}",
        out.stderr
    );
    Ok(())
}

#[test]
fn a_second_shell_tool_name_is_matched_by_a_commands_rule() -> TestResult {
    let w = world(&[])?;
    rules(&w)?;
    put(&w.kb.join(".writfold/harness/other.toml"), HARNESS)?;
    for tool in ["bash", "terminal"] {
        let field = if tool == "bash" {
            "command"
        } else {
            "commandLine"
        };
        let event =
            json!({"sessionId": "s1", "toolName": tool, "args": {field: "git commit -m x"}});
        let out = from_event(&w, &event, Some("other"));
        let (decision, reasons) = checked(&out)?;
        assert_eq!(decision, "deny", "{tool}: {}", out.stdout);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("writfold guard rule commit")),
            "{reasons:?}"
        );
        let value: Value = serde_json::from_str(&out.stdout)?;
        assert_eq!(value["event"]["kind"], "shell");
        assert_eq!(value["event"]["tool"], tool);
    }
    let edit = json!({"sessionId": "s1", "toolName": "edit", "args": {"path": "notes.md"}});
    assert_eq!(checked(&from_event(&w, &edit, Some("other")))?.0, "allow");
    Ok(())
}

#[test]
fn the_guard_log_says_via_check() -> TestResult {
    let w = world(&[])?;
    rules(&w)?;
    let event = claude(&w, "Bash", json!({"command": "git commit -m x"}));
    hook(&w, &event);
    from_event(&w, &event, None);
    let log = std::fs::read_to_string(w.kb.join(".git/writfold/guard-log.jsonl"))?;
    let via: Vec<String> = log
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| v["via"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(via, ["hook", "check"], "{log}");
    Ok(())
}

#[test]
fn a_binding_file_reports_each_fault_at_its_place() -> TestResult {
    let text = "schema = \"writfold-harness/1\"\nid = \"other\"\nshell = [\"bash command\", \"bash: command\"]\nfile_write = [\"bash: path\"]\nreports = [\"shel\"]\ncolour = 1\n";
    let Err(report) = binding::check(text, "other", ".writfold/harness/other.toml") else {
        return Err("a broken binding was accepted".into());
    };
    let rendered: String = report
        .diagnostics
        .iter()
        .map(|d| format!("{}\n", d.message))
        .collect();
    for want in [
        "harness binding needs `session`",
        "\"bash command\" is not `tool name: input field`",
        "tool \"bash\" is bound twice",
        "unknown kind \"shel\"",
        "colour",
    ] {
        assert!(rendered.contains(want), "{want}: {rendered}");
    }
    let lines: Vec<u32> = report
        .diagnostics
        .iter()
        .filter_map(|d| d.at.as_ref().map(|a| a.line))
        .collect();
    assert!(
        lines.contains(&3) && lines.contains(&4) && lines.contains(&5),
        "{lines:?}"
    );
    assert!(binding::check(HARNESS, "other", ".writfold/harness/other.toml").is_ok());
    Ok(())
}

#[test]
fn kb_lint_checks_binding_files() -> TestResult {
    let w = world(&[])?;
    put(
        &w.kb.join(".writfold/harness/other.toml"),
        "schema = \"writfold-harness/1\"\nid = \"other\"\nsession = \"s\"\nreports = [\"shel\"]\n",
    )?;
    let mut request = w.request(writfold_kb::Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: false,
    });
    request.json = true;
    let (text, exit) = w.run(&request)?;
    assert_ne!(exit, 0, "{text}");
    let body: Value = serde_json::from_str(&text)?;
    let found = body["diagnostics"].as_array().ok_or("diagnostics")?;
    assert!(
        found
            .iter()
            .any(|d| d["file"] == ".writfold/harness/other.toml" && d["line"] == 4),
        "{text}"
    );
    Ok(())
}
