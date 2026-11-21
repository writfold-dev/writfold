//! The loop recipes of `docs/guide/recipes/` (#80) run as published: the
//! outer loop drives a scripted agent (one action per session, following
//! `plan status`'s `next:` line, as a real agent would) until the plan is
//! complete, or stops on a spent loop; the hard Stop hook blocks while a
//! round is left and lets the turn end on a spent loop.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One session of the scripted agent.
const AGENT: &str = r#"#!/bin/sh
s=$(writfold kb plan status)
case "$s" in
*"attempt#${FIX_AT:-0}/change [current]"*) touch src/fixed; writfold kb plan advance ;;
*"/change [current]"*) writfold kb plan advance ;;
*"next: writfold kb gate run fixed"*) writfold kb gate run fixed ;;
*"next: writfold kb plan again"*) writfold kb plan again ;;
*"next: writfold kb plan advance"*) writfold kb plan advance ;;
esac >/dev/null 2>&1
exit 0
"#;

struct Demo(PathBuf);

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn put(path: &Path, text: &str) -> Result {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn sh(dir: &Path, demo: &Demo, args: &[&str]) -> Result<(String, i32)> {
    let bin = demo.0.join("bin");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("sh")
        .args(args)
        .current_dir(dir)
        .env("PATH", path)
        .env("WRITFOLD_KB", demo.0.join("kb"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .env_remove("WRITFOLD_SESSION")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((text, out.status.code().unwrap_or(-1)))
}

/// A KB with the `fix` plan (`converge: fixed loop=attempt max=3`) set on
/// a new run in the subject repository `code`.
fn demo(name: &str) -> Result<Demo> {
    let dir = std::env::temp_dir().join(format!("writfold-recipes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let d = Demo(dir.clone());
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), {
        std::fs::create_dir_all(dir.join("bin"))?;
        dir.join("bin/writfold")
    })?;
    put(&dir.join("bin/agent"), AGENT)?;
    put(&dir.join("code/src/a.rs"), "fn a() {}\n")?;
    let kb = dir.join("kb");
    put(
        &kb.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    put(
        &kb.join(".writfold/gates/fixed.toml"),
        "schema = \"writfold-gate/1\"\nid = \"fixed\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"test\", \"-f\", \"src/fixed\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n",
    )?;
    put(
        &kb.join(".writfold/plans/attempt.toml"),
        "schema = \"writfold-plan/1\"\nname = \"attempt\"\nsteps = [\"change:\"]\n",
    )?;
    put(
        &kb.join(".writfold/plans/fix.toml"),
        "schema = \"writfold-plan/1\"\nname = \"fix\"\nsteps = [\"converge: fixed loop=attempt max=3\"]\n",
    )?;
    let code = dir.join("code");
    for (place, cmds) in [
        (
            &kb,
            "git init -q -b main && git add -A && git commit -qm kb",
        ),
        (
            &code,
            "git init -q -b main && git add -A && git commit -qm code",
        ),
    ] {
        let (text, exit) = sh(place, &d, &["-c", cmds])?;
        assert_eq!(exit, 0, "{text}");
    }
    let (text, exit) = sh(
        &code,
        &d,
        &[
            "-c",
            "writfold kb checkpoint --new-run --project demo --task t --note n && writfold kb plan set fix",
        ],
    )?;
    assert_eq!(exit, 0, "{text}");
    Ok(d)
}

fn recipe(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/guide/recipes")
        .join(name)
        .display()
        .to_string()
}

#[test]
fn the_outer_loop_runs_until_the_plan_is_complete() -> Result {
    let d = demo("complete")?;
    let code = d.0.join("code");
    let agent = d.0.join("bin/agent").display().to_string();
    let script = format!(
        "FIX_AT=2 exec sh {} 30 -- sh {agent}",
        recipe("loop-rounds.sh")
    );
    let (text, exit) = sh(&code, &d, &["-c", &script])?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("plan complete"), "{text}");
    let (status, _) = sh(&code, &d, &["-c", "writfold kb plan status"])?;
    assert!(
        status.contains("loop: attempt, 2 rounds of at most 3"),
        "{status}"
    );
    Ok(())
}

#[test]
fn the_outer_loop_stops_on_a_spent_loop_and_the_hook_agrees() -> Result {
    let d = demo("spent")?;
    let code = d.0.join("code");
    let agent = d.0.join("bin/agent").display().to_string();
    let hook = recipe("loop-stop-hook.sh");
    // Mid-loop, after the first round failed: the hook blocks the turn.
    for _ in 0..2 {
        sh(&code, &d, &[&agent])?;
    }
    let (blocked, exit) = sh(&code, &d, &["-c", &format!("echo '{{}}' | sh {hook}")])?;
    assert_eq!(exit, 0, "{blocked}");
    assert!(blocked.contains("\"decision\": \"block\""), "{blocked}");
    let script = format!("exec sh {} 30 -- sh {agent}", recipe("loop-rounds.sh"));
    let (text, exit) = sh(&code, &d, &["-c", &script])?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("loop spent: stop and report"), "{text}");
    let (open, exit) = sh(&code, &d, &["-c", &format!("echo '{{}}' | sh {hook}")])?;
    assert_eq!(
        (open.as_str(), exit),
        ("", 0),
        "a spent loop lets the turn end"
    );
    Ok(())
}
