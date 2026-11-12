//! The Claude Code plugin's hooks through the real binary and the plugin's
//! own `hooks.json` commands: a KB brief when everything is present, and
//! exit 0 with empty stdout for every missing piece (no binary on PATH, no
//! KB, no Git, unreadable input, no event).
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root(name: &str) -> Result<Root> {
    let dir = std::env::temp_dir().join(format!("writfold-hook-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("home"))?;
    fs::create_dir_all(dir.join("empty-path"))?;
    Ok(Root(fs::canonicalize(dir)?))
}

/// A one-asset KB, committed.
fn kb(root: &Path) -> Result<PathBuf> {
    let kb = root.join("kb");
    fs::create_dir_all(kb.join("inventory/assets"))?;
    fs::write(
        kb.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    for args in [
        &["init", "-q"][..],
        &["add", "-A"],
        &["-c", "commit.gpgsign=false", "commit", "-q", "-m", "kb"],
    ] {
        let ok = Command::new("git")
            .current_dir(&kb)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
            .args(args)
            .status()?
            .success();
        if !ok {
            return Err(format!("git {args:?} failed").into());
        }
    }
    Ok(kb)
}

/// Run `program args` in `root` with a clean KB environment and `stdin`.
fn run(
    root: &Path,
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
    stdin: &str,
) -> Result<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("home"))
        .env_remove("WRITFOLD_KB")
        .env_remove("KCTX_CONFIG")
        .env_remove("CLAUDE_PLUGIN_OPTION_KB_ROOT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn()?;
    // A hook that exits before reading its input (the `command -v` guard
    // when writfold is absent) closes the pipe; hosts tolerate that, so the
    // test does too. Any other write error still fails.
    match child
        .stdin
        .take()
        .ok_or("stdin")?
        .write_all(stdin.as_bytes())
    {
        Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => return Err(error.into()),
        _ => {}
    }
    Ok(child.wait_with_output()?)
}

fn silent(output: &Output, why: &str) -> Result {
    if output.status.code() != Some(0) || !output.stdout.is_empty() {
        return Err(format!(
            "{why}: exit {:?}, stdout {:?}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout)
        )
        .into());
    }
    Ok(())
}

fn context(output: &Output) -> Result<String> {
    let value: Value = serde_json::from_slice(&output.stdout)?;
    Ok(value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("no additionalContext")?
        .to_owned())
}

#[test]
fn binary_hooks_fail_silently() -> Result {
    let r = root("binary")?;
    let start = format!("{{\"session_id\": \"s\", \"cwd\": \"{}\"}}", r.0.display());
    let hook =
        |args: &[&str], env: &[(&str, &str)], stdin: &str| run(&r.0, WRITFOLD, args, env, stdin);
    silent(&hook(&["kb", "hook"], &[], "{}")?, "no event")?;
    silent(
        &hook(&["kb", "hook", "session-start"], &[], "")?,
        "no input",
    )?;
    silent(
        &hook(&["kb", "hook", "session-start"], &[], &start)?,
        "no KB",
    )?;
    let nowhere = r.0.join("nowhere").display().to_string();
    let option = [("CLAUDE_PLUGIN_OPTION_KB_ROOT", nowhere.as_str())];
    silent(
        &hook(&["kb", "hook", "session-start"], &option, &start)?,
        "KB option names no KB",
    )?;
    let kb = kb(&r.0)?.display().to_string();
    let option = [("CLAUDE_PLUGIN_OPTION_KB_ROOT", kb.as_str())];
    let no_git = [option[0], ("WRITFOLD_GIT", "/nonexistent/git")];
    silent(
        &hook(&["kb", "hook", "session-start"], &no_git, &start)?,
        "no Git",
    )?;
    let stop = "{\"session_id\": \"s\", \"last_assistant_message\": \"done\"}";
    silent(&hook(&["kb", "hook", "stop"], &option, stop)?, "stop")?;
    assert!(Path::new(&kb).join(".git/writfold/drafts/s.json").is_file());
    let started = hook(&["kb", "hook", "session-start"], &option, &start)?;
    assert_eq!(started.status.code(), Some(0));
    let text = context(&started)?;
    assert!(text.starts_with("Writfold KB brief"), "{text}");
    assert!(text.contains("draft s "), "{text}");
    Ok(())
}

/// The commands in the plugin's `hooks.json`, run as Claude Code runs a
/// shell-form hook (`sh -c`).
#[test]
fn plugin_commands_need_nothing_but_the_binary() -> Result {
    let r = root("plugin")?;
    let file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/writfold/hooks/hooks.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(file)?)?;
    let events = config["hooks"].as_object().ok_or("hooks object")?;
    let names: Vec<&String> = events.keys().collect();
    assert_eq!(
        names,
        [
            "PostCompact",
            "PreCompact",
            "PreToolUse",
            "SessionEnd",
            "SessionStart",
            "Stop"
        ]
    );
    let bin = r.0.join("bin");
    fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(WRITFOLD, bin.join("writfold"))?;
    let kb = kb(&r.0)?.display().to_string();
    let start = format!("{{\"session_id\": \"s\", \"cwd\": \"{kb}\"}}");
    let empty = r.0.join("empty-path").display().to_string();
    let with = format!("{}:/usr/bin:/bin", bin.display());
    for (event, groups) in events {
        let command = groups[0]["hooks"][0]["command"].as_str().ok_or("command")?;
        let without = run(
            &r.0,
            "/bin/sh",
            &["-c", command],
            &[("PATH", &empty)],
            &start,
        )?;
        silent(&without, &format!("{event} without writfold on PATH"))?;
        let present = run(
            &r.0,
            "/bin/sh",
            &["-c", command],
            &[("PATH", &with)],
            &start,
        )?;
        assert_eq!(present.status.code(), Some(0), "{event}");
        if event == "SessionStart" {
            assert!(context(&present)?.contains("kb="), "{event}");
        }
        if event == "PreToolUse" {
            // Off by default: a KB with no guard configuration allows a
            // publishing command by printing nothing.
            let post = format!(
                "{{\"cwd\": \"{kb}\", \"hook_event_name\": \"PreToolUse\", \"tool_name\": \"Bash\", \"tool_input\": {{\"command\": \"gh pr comment 1 --body x\"}}}}"
            );
            let output = run(&r.0, "/bin/sh", &["-c", command], &[("PATH", &with)], &post)?;
            silent(&output, "PreToolUse with no guard configured")?;
            assert!(output.stderr.is_empty(), "{event}");
            // Configured: the same command is denied (no run, no candidate).
            let guard = Path::new(&kb).join(".writfold/deliveries");
            fs::create_dir_all(&guard)?;
            fs::write(
                guard.join("guard.toml"),
                "schema = \"writfold-guard/1\"\ncommands = [\"gh pr comment\"]\n",
            )?;
            let output = run(&r.0, "/bin/sh", &["-c", command], &[("PATH", &with)], &post)?;
            assert_eq!(output.status.code(), Some(0), "{event}");
            let decision: Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(
                decision["hookSpecificOutput"]["permissionDecision"], "deny",
                "{decision}"
            );
            fs::remove_dir_all(Path::new(&kb).join(".writfold"))?;
        }
    }
    Ok(())
}
