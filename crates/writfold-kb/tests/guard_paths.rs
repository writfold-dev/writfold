//! Guard rules' `paths` and a stop rule's `repeat`, through `writfold kb
//! hook pre-tool-use` and `stop`: a rule with `paths` matches only an edit
//! whose file they admit (exclusions win in any order, decided before the
//! index is opened), and `repeat = "turn"` nudges once per user turn,
//! never while a stop hook is continuing one.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::hook::{HookEnv, HookOutput, run};
use writfold_kb::plan::PlanVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const EDITS: &str = "schema = \"writfold-guard-rule/1\"\nid = \"edits\"\nevent = \"pre-tool-use\"\ntools = [\"Edit\", \"Write\", \"MultiEdit\", \"NotebookEdit\"]\npaths = [\"**\", \"!**/*.md\"]\nrequire = [\"run.current\"]\naction = \"deny\"\n";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn rule(w: &World, id: &str, text: &str) -> TestResult {
    put(&w.kb.join(format!(".writfold/guards/{id}.toml")), text)
}

fn env(w: &World) -> HookEnv {
    HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    }
}

fn tool(w: &World, name: &str, input: Value) -> HookOutput {
    let event = json!({"session_id": "s1", "cwd": w.code.display().to_string(),
        "hook_event_name": "PreToolUse", "tool_name": name, "tool_input": input});
    run("pre-tool-use", &event.to_string(), &env(w))
}

fn edit(w: &World, file: &str) -> HookOutput {
    tool(w, "Edit", json!({"file_path": file}))
}

fn denied(out: &HookOutput) -> bool {
    out.stdout.contains("\"permissionDecision\":\"deny\"")
}

fn index_files(w: &World) -> usize {
    std::fs::read_dir(w.kb.join(".git/writfold"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("kb-index"))
        .count()
}

#[test]
fn a_markdown_edit_passes_while_a_source_edit_is_denied() -> TestResult {
    let w = world(&[])?;
    rule(&w, "edits", EDITS)?;
    let code = w.code.display().to_string();
    for md in [
        format!("{code}/README.md"),
        format!("{code}/docs/a/b.md"),
        "docs/notes.md".into(),
        "src/../docs/x.md".into(),
    ] {
        assert_eq!(edit(&w, &md).stdout, "", "{md}");
    }
    assert_eq!(index_files(&w), 0, "an excluded path opened the index");
    for rs in [
        format!("{code}/src/ledger.rs"),
        "src/new.rs".into(),
        "./Cargo.toml".into(),
    ] {
        assert!(denied(&edit(&w, &rs)), "{rs}");
    }
    let multi = tool(&w, "MultiEdit", json!({"file_path": "src/ledger.rs"}));
    assert!(denied(&multi));
    let notebook = tool(&w, "NotebookEdit", json!({"notebook_path": "nb.ipynb"}));
    assert!(denied(&notebook));
    let notes = tool(&w, "NotebookEdit", json!({"notebook_path": "nb.md"}));
    assert_eq!(notes.stdout, "");
    // No file named, or a file in no repository: the rule does not match.
    assert_eq!(tool(&w, "Write", json!({})).stdout, "");
    let loose = w.base.join("loose/x.rs").display().to_string();
    assert_eq!(edit(&w, &loose).stdout, "", "outside every repository");
    Ok(())
}

#[test]
fn exclusions_win_in_any_order_and_paths_are_repository_relative() -> TestResult {
    let w = world(&[])?;
    let reversed = EDITS.replace("[\"**\", \"!**/*.md\"]", "[\"!**/*.md\", \"**\"]");
    rule(&w, "edits", &reversed)?;
    assert_eq!(edit(&w, "README.md").stdout, "");
    assert!(denied(&edit(&w, "src/ledger.rs")));
    let narrow = EDITS.replace("[\"**\", \"!**/*.md\"]", "[\"src/**\", \"!src/gen/**\"]");
    rule(&w, "edits", &narrow)?;
    assert!(denied(&edit(&w, "src/ledger.rs")));
    assert_eq!(edit(&w, "src/gen/out.rs").stdout, "");
    assert_eq!(edit(&w, "Cargo.toml").stdout, "");
    // Anchored at the file's repository, not at the event's cwd.
    let deep = w.code.join("src");
    let event = json!({"session_id": "s1", "cwd": deep.display().to_string(),
        "hook_event_name": "PreToolUse", "tool_name": "Edit",
        "tool_input": {"file_path": "ledger.rs"}});
    assert!(denied(&run("pre-tool-use", &event.to_string(), &env(&w))));
    Ok(())
}

#[test]
fn paths_and_repeat_are_refused_where_they_cannot_mean_anything() {
    use writfold_kb::guard::rules::parse;
    let path = ".writfold/guards/edits.toml";
    assert!(parse(EDITS, "edits", path).is_ok());
    let stop = "schema = \"writfold-guard-rule/1\"\nid = \"edits\"\nevent = \"stop\"\nrequire = [\"step.met\"]\n";
    for good in [
        format!("{stop}repeat = \"turn\"\n"),
        format!("{stop}repeat = \"session\"\n"),
    ] {
        assert!(parse(&good, "edits", path).is_ok(), "refused {good:?}");
    }
    for bad in [
        EDITS.replace("tools =", "commands = [\"git commit\"]\ntools ="),
        EDITS.replace(
            "tools = [\"Edit\", \"Write\", \"MultiEdit\", \"NotebookEdit\"]",
            "commands = [\"git commit\"]",
        ),
        EDITS.replace("[\"**\", \"!**/*.md\"]", "[\"!**/*.md\"]"),
        EDITS.replace("[\"**\", \"!**/*.md\"]", "[\"/src/**\"]"),
        EDITS.replace("[\"**\", \"!**/*.md\"]", "[\"\"]"),
        format!("{EDITS}repeat = \"turn\"\n"),
        format!("{stop}repeat = \"always\"\n"),
        format!("{stop}paths = [\"**\"]\n"),
    ] {
        assert!(parse(&bad, "edits", path).is_err(), "accepted {bad:?}");
    }
}

fn stop(w: &World, active: bool) -> HookOutput {
    let event = json!({"session_id": "s1", "cwd": w.code.display().to_string(),
        "hook_event_name": "Stop", "stop_hook_active": active,
        "last_assistant_message": "Done."});
    run("stop", &event.to_string(), &env(w))
}

fn context(out: &HookOutput) -> TestResult<String> {
    let value: Value = serde_json::from_str(&out.stdout)?;
    let text = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("no context")?;
    Ok(text.to_owned())
}

/// A current run whose plan's step owes the `unit` gate.
fn owing(w: &World) -> TestResult {
    put(&w.kb.join(".git/writfold/current-run"), &format!("{RUN}\n"))?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
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
    Ok(())
}

const STEP: &str =
    "schema = \"writfold-guard-rule/1\"\nid = \"ID\"\nevent = \"stop\"\nrequire = [\"step.met\"]\n";

#[test]
fn repeat_turn_nudges_every_turn_but_never_while_active() -> TestResult {
    let w = world(&[])?;
    owing(&w)?;
    rule(
        &w,
        "each-turn",
        &format!("{}repeat = \"turn\"\n", STEP.replace("ID", "each-turn")),
    )?;
    assert_eq!(stop(&w, true), HookOutput::default(), "nudged while active");
    let first = context(&stop(&w, false))?;
    assert!(
        first.contains("writfold guard rule each-turn (a guard, not enforcement; once a turn)")
            && first.contains("will not raise it again this turn"),
        "{first}"
    );
    // The continuation it caused ends with the flag set: silent.
    assert_eq!(stop(&w, true), HookOutput::default(), "looped");
    // The next user turn: it fires again.
    let second = context(&stop(&w, false))?;
    assert!(second.contains("each-turn"), "{second}");
    Ok(())
}

#[test]
fn a_session_rule_beside_a_turn_rule_still_fires_once() -> TestResult {
    let w = world(&[])?;
    owing(&w)?;
    rule(&w, "once", &STEP.replace("ID", "once"))?;
    rule(
        &w,
        "each-turn",
        &format!("{}repeat = \"turn\"\n", STEP.replace("ID", "each-turn")),
    )?;
    let first = context(&stop(&w, false))?;
    assert!(
        first.contains("rule once (a guard, not enforcement; once this session)")
            && first.contains("rule each-turn"),
        "{first}"
    );
    let second = context(&stop(&w, false))?;
    assert!(
        !second.contains("rule once") && second.contains("rule each-turn"),
        "{second}"
    );
    // Only the session rule: silent from now on.
    std::fs::remove_file(w.kb.join(".writfold/guards/each-turn.toml"))?;
    assert_eq!(stop(&w, false), HookOutput::default());
    Ok(())
}
