//! A current run per session through the real binary: the session-start
//! hook hands the session's ID to later shell commands (`CLAUDE_ENV_FILE`),
//! a verb run with `WRITFOLD_SESSION` writes and reads that session's
//! pointer, and a shell without it reads the clone's shared pointer.
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
    let dir = std::env::temp_dir().join(format!("writfold-seat-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("home"))?;
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

/// Run `writfold args` in `root` for `session` (none: a plain shell).
fn run(root: &Path, args: &[&str], env: &[(&str, &str)], stdin: &str) -> Result<Output> {
    let mut command = Command::new(WRITFOLD);
    command
        .args(args)
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("home"))
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_SESSION")
        .env_remove("CLAUDE_ENV_FILE")
        .env_remove("CLAUDE_PLUGIN_OPTION_KB_ROOT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin")?
        .write_all(stdin.as_bytes())?;
    Ok(child.wait_with_output()?)
}

fn ok(output: Output) -> Result<String> {
    if !output.status.success() {
        return Err(format!(
            "exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// The run a session's `resume` shows.
fn resumed(root: &Path, kb: &str, env: &[(&str, &str)]) -> Result<String> {
    let text = ok(run(root, &["kb", "resume", "--kb", kb], env, "")?)?;
    let first = text.lines().next().unwrap_or_default();
    Ok(first.split(' ').nth(1).unwrap_or_default().to_owned())
}

/// Session start: the hook appends the session's ID to the file Claude
/// Code runs before each Bash command.
#[test]
fn the_session_start_hook_exports_the_session() -> Result {
    let r = root("export")?;
    let kb = kb(&r.0)?.display().to_string();
    let env_file = r.0.join("session-env.sh");
    let env_path = env_file.display().to_string();
    let start = format!(
        "{{\"session_id\": \"sess-1\", \"hook_event_name\": \"SessionStart\", \"cwd\": \"{}\"}}",
        r.0.display()
    );
    let hook_env = [
        ("CLAUDE_PLUGIN_OPTION_KB_ROOT", kb.as_str()),
        ("CLAUDE_ENV_FILE", env_path.as_str()),
    ];
    let hook = ["kb", "hook", "session-start"];
    let started = run(&r.0, &hook, &hook_env, &start)?;
    assert_eq!(started.status.code(), Some(0));
    assert!(!started.stdout.is_empty(), "the context is still printed");
    assert_eq!(
        fs::read_to_string(&env_file)?,
        "export WRITFOLD_SESSION=sess-1\n"
    );
    // The same session starting again (a resume) adds no second line.
    run(&r.0, &hook, &hook_env, &start)?;
    assert_eq!(
        fs::read_to_string(&env_file)?,
        "export WRITFOLD_SESSION=sess-1\n"
    );
    // Without CLAUDE_ENV_FILE the hook writes nowhere and still answers.
    fs::remove_file(&env_file)?;
    let plain = run(&r.0, &hook, &hook_env[..1], &start)?;
    assert_eq!(plain.status.code(), Some(0));
    assert!(!env_file.exists());
    Ok(())
}

#[test]
fn verbs_keep_a_run_per_session() -> Result {
    let r = root("session")?;
    let kb = kb(&r.0)?.display().to_string();

    // Two sessions each start a run on the same clone.
    let new_run = |session: &str, task: &str| -> Result<String> {
        let args = [
            "kb",
            "checkpoint",
            "--new-run",
            "--project",
            "demo",
            "--task",
            task,
            "--note",
            "start",
            "--kb",
            kb.as_str(),
        ];
        ok(run(&r.0, &args, &[("WRITFOLD_SESSION", session)], "")?)?;
        let pointer = Path::new(&kb)
            .join(".git/writfold/current-run.d")
            .join(format!("session-{session}"));
        Ok(fs::read_to_string(pointer)?.trim().to_owned())
    };
    let first = new_run("sess-1", "one")?;
    let second = new_run("sess-2", "two")?;
    assert_ne!(first, second);

    // Each session resumes its own run; a shell with no session reads the
    // shared pointer, which is the run made current last.
    assert_eq!(
        resumed(&r.0, &kb, &[("WRITFOLD_SESSION", "sess-1")])?,
        first
    );
    assert_eq!(
        resumed(&r.0, &kb, &[("WRITFOLD_SESSION", "sess-2")])?,
        second
    );
    assert_eq!(resumed(&r.0, &kb, &[])?, second);
    // A value that is not a session ID is not a session.
    assert_eq!(resumed(&r.0, &kb, &[("WRITFOLD_SESSION", "../x")])?, second);

    // A reviewer's session attaches to the first run.
    let here = [
        "kb",
        "resume",
        first.as_str(),
        "--here",
        "--kb",
        kb.as_str(),
    ];
    let text = ok(run(&r.0, &here, &[("WRITFOLD_SESSION", "review-1")], "")?)?;
    assert!(
        text.contains("is now the current run here (session review-1"),
        "{text}"
    );
    assert_eq!(
        resumed(&r.0, &kb, &[("WRITFOLD_SESSION", "review-1")])?,
        first
    );
    assert_eq!(
        resumed(&r.0, &kb, &[("WRITFOLD_SESSION", "sess-2")])?,
        second
    );
    Ok(())
}
