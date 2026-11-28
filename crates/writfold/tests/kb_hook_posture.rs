//! The guard site's posture through the real binary and the plugin's own
//! PreToolUse command (#184): the strict shim blocks with writfold off
//! PATH while the default one passes, and under a closed posture an event
//! not decided by the deadline gets the posture's decision.
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
        #[expect(
            clippy::let_underscore_must_use,
            reason = "best-effort cleanup of a temp dir"
        )]
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root(name: &str) -> Result<Root> {
    let dir = std::env::temp_dir().join(format!("writfold-posture-{name}-{}", std::process::id()));
    #[expect(
        clippy::let_underscore_must_use,
        reason = "a leftover from an earlier run may not exist"
    )]
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("home"))?;
    fs::create_dir_all(dir.join("empty-path"))?;
    fs::create_dir_all(dir.join("kb/inventory/assets"))?;
    Ok(Root(fs::canonicalize(dir)?))
}

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
        .env_remove("WRITFOLD_KB")
        .env_remove("CLAUDE_PLUGIN_OPTION_KB_ROOT")
        .env_remove("CLAUDE_PLUGIN_OPTION_POSTURE")
        .env_remove("WRITFOLD_HOOK_DEADLINE_MS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn()?;
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

fn shim() -> Result<String> {
    let file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/writfold/hooks/hooks.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(file)?)?;
    let command = config["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .ok_or("command")?;
    Ok(command.to_owned())
}

fn event(r: &Root) -> String {
    format!(
        "{{\"session_id\": \"s\", \"cwd\": \"{}\", \"hook_event_name\": \"PreToolUse\", \"tool_name\": \"Bash\", \"tool_input\": {{\"command\": \"git status\"}}}}",
        r.0.join("kb").display()
    )
}

#[test]
fn the_strict_shim_blocks_without_the_binary_and_the_default_passes() -> Result {
    let r = root("shim")?;
    let command = shim()?;
    let empty = r.0.join("empty-path").display().to_string();
    let default = run(
        &r.0,
        "/bin/sh",
        &["-c", &command],
        &[("PATH", &empty)],
        &event(&r),
    )?;
    assert_eq!(default.status.code(), Some(0));
    assert!(default.stdout.is_empty() && default.stderr.is_empty());
    for posture in ["ask", "deny"] {
        let strict = run(
            &r.0,
            "/bin/sh",
            &["-c", &command],
            &[("PATH", &empty), ("CLAUDE_PLUGIN_OPTION_POSTURE", posture)],
            &event(&r),
        )?;
        assert_eq!(strict.status.code(), Some(2), "{posture}");
        let said = String::from_utf8_lossy(&strict.stderr);
        assert!(said.contains(&format!("posture={posture}")), "{said}");
    }
    Ok(())
}

#[test]
fn a_closed_posture_decides_at_the_deadline() -> Result {
    let r = root("deadline")?;
    let kb = r.0.join("kb").display().to_string();
    let args = ["kb", "hook", "pre-tool-use", "--posture", "deny"];
    let late = run(
        &r.0,
        WRITFOLD,
        &args,
        &[("WRITFOLD_KB", &kb), ("WRITFOLD_HOOK_DEADLINE_MS", "0")],
        &event(&r),
    )?;
    assert_eq!(late.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&late.stdout)?;
    let specific = &value["hookSpecificOutput"];
    assert_eq!(specific["permissionDecision"], "deny", "{value}");
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?;
    assert!(reason.contains("did not decide within 0 ms"), "{reason}");
    let in_time = run(&r.0, WRITFOLD, &args, &[("WRITFOLD_KB", &kb)], &event(&r))?;
    assert!(
        in_time.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&in_time.stdout)
    );
    let by_option = run(
        &r.0,
        WRITFOLD,
        &["kb", "hook", "pre-tool-use"],
        &[
            ("CLAUDE_PLUGIN_OPTION_POSTURE", "ask"),
            ("WRITFOLD_KB", "/nonexistent"),
        ],
        &event(&r),
    )?;
    let value: Value = serde_json::from_slice(&by_option.stdout)?;
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"], "ask",
        "{value}"
    );
    Ok(())
}
