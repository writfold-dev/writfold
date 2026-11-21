//! `writfold kb` help: the overview for `kb help` (and `--help` before any
//! verb), a verb's own help for `kb <verb> --help` (#55); each printed
//! once, to stdout, with exit 0. A usage error is exit 1, on stderr.
use std::process::{Command, Output};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn writfold(args: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_writfold"))
        .args(args)
        .env_remove("WRITFOLD_KB")
        .env("NO_COLOR", "1")
        .current_dir(std::env::temp_dir())
        .output()?)
}

#[test]
fn help_prints_the_usage_once_to_stdout() -> Result {
    let cases: [&[&str]; 4] = [
        &["kb", "--help"],
        &["kb", "-h"],
        &["kb", "help"],
        &["kb", "--json", "--help"],
    ];
    for args in cases {
        let out = writfold(args)?;
        let stdout = String::from_utf8(out.stdout)?;
        let stderr = String::from_utf8(out.stderr)?;
        assert_eq!(out.status.code(), Some(0), "{args:?}: {stderr}");
        assert_eq!(stderr, "", "{args:?}");
        assert_eq!(
            stdout.matches("Exit codes").count(),
            1,
            "{args:?}: {stdout}"
        );
        assert!(
            stdout.starts_with("usage: writfold kb"),
            "{args:?}: {stdout}"
        );
    }
    Ok(())
}

#[test]
fn a_verb_has_its_own_help() -> Result {
    let cases: [(&[&str], &str); 5] = [
        (&["kb", "gate", "--help"], "gate run <id>"),
        (&["kb", "gate", "run", "--help"], "gate run <id>"),
        (&["kb", "find", "ledger", "--help"], "--signatures"),
        (&["kb", "help", "impact"], "--direction <DIRECTION>"),
        (&["kb", "stale", "-h"], "--cached"),
    ];
    for (args, wanted) in cases {
        let out = writfold(args)?;
        let stdout = String::from_utf8(out.stdout)?;
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert!(stdout.contains(wanted), "{args:?}: {stdout}");
        assert!(stdout.contains("Usage: writfold kb "), "{args:?}: {stdout}");
        assert!(!stdout.contains("Exit codes"), "not the overview: {args:?}");
    }
    Ok(())
}

#[test]
fn usage_errors_exit_1_with_a_suggestion() -> Result {
    let cases: [(&[&str], &str); 6] = [
        (&["kb", "brif"], "a similar subcommand exists: 'brief'"),
        (
            &["kb", "brief", "--jsn"],
            "a similar argument exists: '--json'",
        ),
        (
            &["kb", "stale", "--missing"],
            "--missing does not apply to stale",
        ),
        (
            &["kb", "stale", "--at", "x"],
            "--at COMMIT is not built in v1",
        ),
        (
            &["kb", "find", "--kind", "a", "--kind", "b"],
            "cannot be used multiple times",
        ),
        (&["kb"], "missing verb"),
    ];
    for (args, wanted) in cases {
        let out = writfold(args)?;
        let stderr = String::from_utf8(out.stderr)?;
        assert_eq!(out.status.code(), Some(1), "{args:?}: {stderr}");
        assert!(stderr.contains(wanted), "{args:?}: {stderr}");
        assert!(!stderr.contains('\x1b'), "NO_COLOR: {stderr}");
    }
    let out = writfold(&["kbb"])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)?.contains("'kb'"));
    let out = writfold(&["--version"])?;
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)?.starts_with("writfold "));
    Ok(())
}

#[test]
fn colour_follows_the_color_flag() -> Result {
    let out = writfold(&["kb", "brief", "--jsn", "--color", "always"])?;
    assert!(String::from_utf8(out.stderr)?.contains('\x1b'));
    let out = writfold(&["kb", "brief", "--jsn", "--color", "never"])?;
    assert!(!String::from_utf8(out.stderr)?.contains('\x1b'));
    Ok(())
}

/// The help's hook list is the plugin's: every `writfold kb hook <event>`
/// that `plugins/writfold/hooks/hooks.json` registers is named (#53).
#[test]
fn help_names_every_hook_the_plugin_registers() -> Result {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let hooks = std::fs::read_to_string(root.join("plugins/writfold/hooks/hooks.json"))?;
    let out = writfold(&["kb", "help"])?;
    let stdout = String::from_utf8(out.stdout)?;
    let line = (stdout.lines())
        .find(|l| l.trim_start().starts_with("hook "))
        .ok_or("a hook line in the help")?;
    let listed: Vec<&str> = line.trim_start()["hook ".len()..].split('|').collect();
    let mut registered = 0;
    for (at, _) in hooks.match_indices("writfold kb hook ") {
        let event = hooks[at + "writfold kb hook ".len()..]
            .split(|c: char| c == '"' || c.is_whitespace())
            .next()
            .unwrap_or_default();
        assert!(
            listed.contains(&event),
            "{event} is not in the help: {line}"
        );
        registered += 1;
    }
    assert_eq!(registered, listed.len(), "{line}");
    Ok(())
}
