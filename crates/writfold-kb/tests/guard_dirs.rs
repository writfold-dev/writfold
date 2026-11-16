//! Guard rules judge the repository a command acts on: through `cd`,
//! groups and `git -C`, with a repository's own rules applying only to
//! the commands that act under it, a directory that cannot be told taking
//! the rule's action, and a linked worktree named after its repository.
mod support;

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;
use writfold_kb::hook::{HookEnv, run};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const COMMIT: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\", \"git push\"]\nrequire = [\"run.current\", \"gates.met\"]\naction = \"deny\"\n";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";

fn put(path: &Path, text: &str) -> TestResult {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn rule(dir: &Path, id: &str, text: &str) -> TestResult {
    put(&dir.join(format!(".writfold/guards/{id}.toml")), text)
}

/// A world with the run current, gate `unit` defined and met for
/// demo-service, and a second repository `other` where it is open.
fn two_repos() -> TestResult<(World, PathBuf)> {
    let w = world(&[])?;
    put(&w.kb.join(".git/writfold/current-run"), &format!("{RUN}\n"))?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    let other = w.base.join("other");
    put(&other.join("src/ledger.rs"), "pub fn other() {}\n")?;
    git(&other, &["init", "-q"])?;
    git(&other, &["add", "-A"])?;
    git(&other, &["commit", "-q", "-m", "other"])?;
    let verb = GateVerb::Run {
        id: "unit".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    assert_eq!(writfold_kb::run(&w.request(Verb::Gate(verb)))?.exit, 0);
    Ok((w, other))
}

/// The decision (`pass` when silent) and reason for a Bash call at `cwd`.
fn decide(
    w: &World,
    cwd: &Path,
    command: &str,
    home: Option<&Path>,
) -> TestResult<(String, String)> {
    let input = json!({"session_id": "s1", "cwd": cwd, "hook_event_name": "PreToolUse",
        "tool_name": "Bash", "tool_input": {"command": command}});
    let env = HookEnv {
        home: home.map(Path::to_path_buf),
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 3600),
    };
    let out = run("pre-tool-use", &input.to_string(), &env);
    if out.stdout.is_empty() {
        return Ok(("pass".into(), String::new()));
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let specific = &value["hookSpecificOutput"];
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?;
    let decision = specific["permissionDecision"].as_str().ok_or("decision")?;
    Ok((decision.to_owned(), reason.to_owned()))
}

fn verdict(w: &World, cwd: &Path, command: &str) -> TestResult<String> {
    Ok(decide(w, cwd, command, None)?.0)
}

#[test]
fn each_command_is_judged_where_it_acts() -> TestResult {
    let (w, other) = two_repos()?;
    rule(&w.kb, "commit", COMMIT)?;
    let code = w.code.clone();
    assert_eq!(verdict(&w, &code, "git commit -m x")?, "pass");
    assert_eq!(verdict(&w, &other, "git commit -m x")?, "deny");
    for line in [
        "cd ../other && git commit",
        "git -C ../other commit",
        "git -C ../other --no-pager commit",
        "cd .. && git -C other commit",
    ] {
        let (d, reason) = decide(&w, &code, line, None)?;
        assert_eq!(d, "deny", "{line}");
        assert!(reason.contains("gate unit [open]"), "{reason}");
    }
    for line in [
        "(cd ../other && true); git commit",
        "cd ../other | true; git commit",
    ] {
        assert_eq!(verdict(&w, &code, line)?, "pass", "{line}");
    }
    assert_eq!(
        verdict(&w, &other, "cd ../demo-service && git commit")?,
        "pass"
    );
    assert_eq!(
        verdict(&w, &other, "git -C ../demo-service commit")?,
        "pass"
    );
    // The command after a here-document is still read; its body is not.
    let body = "cat > notes.md <<EOF\ngit commit -m x\nEOF";
    assert_eq!(verdict(&w, &other, body)?, "pass");
    assert_eq!(verdict(&w, &other, &format!("{body}\ngit push"))?, "deny");
    for line in [
        "git commit --help",
        "cat README.md",
        "grep -r \"git commit\" .",
        "writfold kb find ledger",
    ] {
        assert_eq!(verdict(&w, &other, line)?, "pass", "{line}");
    }
    // Home is expanded only when known.
    let home = Some(w.base.as_path());
    assert_eq!(decide(&w, &code, "git -C ~/other commit", home)?.0, "deny");
    assert_eq!(
        decide(&w, &code, "git -C \"$HOME/demo-service\" commit", home)?.0,
        "pass"
    );
    Ok(())
}

#[test]
fn a_directory_that_cannot_be_told_takes_the_action() -> TestResult {
    let (w, _) = two_repos()?;
    rule(&w.kb, "commit", COMMIT)?;
    let code = w.code.clone();
    for (line, why) in [
        ("git -C \"$R\" commit", "(git commit: a variable)"),
        ("git -C b* commit", "(git commit: a pattern)"),
        ("cd - && git commit", "(git commit: an earlier directory"),
        (
            "GIT_DIR=x git commit",
            "(git commit: a Git directory set apart",
        ),
        (
            "git -C ~/x commit",
            "(git commit: the home directory, which is not known here)",
        ),
    ] {
        let (d, reason) = decide(&w, &code, line, None)?;
        assert_eq!(d, "deny", "{line}");
        assert!(
            reason.contains("cannot tell which repository this command acts on "),
            "{reason}"
        );
        assert!(reason.contains(why), "{reason}");
        assert!(reason.contains("name the directory literally"), "{reason}");
    }
    // A rule with repos applies: the repository cannot be told.
    let repos = COMMIT.replace("require =", "repos = [\"elsewhere\"]\nrequire =");
    rule(&w.kb, "commit", &repos)?;
    assert_eq!(verdict(&w, &code, "git -C \"$R\" commit")?, "deny");
    assert_eq!(verdict(&w, &code, "git commit")?, "pass");
    rule(&w.kb, "commit", &COMMIT.replace("\"deny\"", "\"ask\""))?;
    assert_eq!(verdict(&w, &code, "git -C \"$R\" commit")?, "ask");
    // run.current needs no repository: it is judged as usual.
    let run_only = COMMIT.replace("\"run.current\", \"gates.met\"", "\"run.current\"");
    rule(&w.kb, "commit", &run_only)?;
    assert_eq!(verdict(&w, &code, "git -C \"$R\" commit")?, "pass");
    Ok(())
}

#[test]
fn more_than_four_directories_are_not_judged() -> TestResult {
    let (w, _) = two_repos()?;
    rule(&w.kb, "commit", COMMIT)?;
    let code = w.code.clone();
    let four = "git commit; git -C src commit; git -C /tmp commit; git -C .. commit";
    let (d, reason) = decide(&w, &code, four, None)?;
    assert!(!reason.contains("more directories"), "{d}: {reason}");
    let five = format!("{four}; git -C ../other/src commit");
    let (d, reason) = decide(&w, &code, &five, None)?;
    assert_eq!(d, "deny");
    assert!(
        reason.contains("more directories than one call judges"),
        "{reason}"
    );
    Ok(())
}

#[test]
fn a_repository_rule_applies_to_the_commands_acting_under_it() -> TestResult {
    let (w, other) = two_repos()?;
    let code = w.code.clone();
    rule(&code, "commit", COMMIT)?;
    assert_eq!(verdict(&w, &code, "git -C ../other commit")?, "pass");
    assert_eq!(
        verdict(&w, &other, "git -C ../demo-service commit")?,
        "pass"
    );
    rule(
        &other,
        "other-commit",
        &COMMIT.replace("id = \"commit\"", "id = \"other-commit\""),
    )?;
    let (d, reason) = decide(&w, &code, "git -C ../other commit", None)?;
    assert_eq!(d, "deny");
    assert!(
        reason.contains("rule other-commit") && !reason.contains("rule commit "),
        "{reason}"
    );
    // An unresolved directory: the KB root's rules only, and here none.
    assert_eq!(verdict(&w, &other, "git -C \"$R\" commit")?, "pass");
    Ok(())
}

#[test]
fn a_linked_worktree_answers_to_its_repository() -> TestResult {
    let (w, other) = two_repos()?;
    git(&w.code, &["worktree", "add", "-q", "../demo-wt"])?;
    git(&other, &["worktree", "add", "-q", "../other-wt"])?;
    let demo_wt = w.base.join("demo-wt");
    let other_wt = w.base.join("other-wt");
    // demo-service by its map entry; other by its main clone's name. The
    // rule owes an undefined gate, so it denies wherever it applies.
    let named = |repos: &str| {
        COMMIT
            .replace("\"run.current\", \"gates.met\"", "\"gate:review\"")
            .replace("require =", &format!("repos = [\"{repos}\"]\nrequire ="))
    };
    rule(&w.kb, "commit", &named("demo-service"))?;
    assert_eq!(verdict(&w, &demo_wt, "git commit")?, "deny");
    assert_eq!(verdict(&w, &other_wt, "git commit")?, "pass");
    rule(&w.kb, "commit", &named("other"))?;
    assert_eq!(verdict(&w, &other_wt, "git commit")?, "deny");
    // A map entry for the clone names its worktrees too.
    let map = w.kb.join(".git/writfold/repos");
    let listed = std::fs::read_to_string(&map)?;
    put(&map, &format!("{listed}ledger {}\n", other.display()))?;
    rule(&w.kb, "commit", &named("ledger"))?;
    assert_eq!(verdict(&w, &other_wt, "git commit")?, "deny");
    rule(&w.kb, "commit", &named("demo-wt"))?;
    assert_eq!(verdict(&w, &demo_wt, "git commit")?, "pass");
    // The receipt filed in the clone stands in its worktree.
    rule(&w.kb, "commit", COMMIT)?;
    assert_eq!(verdict(&w, &demo_wt, "git commit")?, "pass");
    Ok(())
}
