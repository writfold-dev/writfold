//! `writfold kb` help: asked for anywhere on the line, the usage is printed
//! once, to stdout, and the exit is 0.
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn help_prints_the_usage_once_to_stdout() -> Result {
    let cases: [&[&str]; 7] = [
        &["kb", "--help"],
        &["kb", "-h"],
        &["kb", "help"],
        &["kb", "--json", "--help"],
        &["kb", "gate", "--help"],
        &["kb", "gate", "run", "--help"],
        &["kb", "find", "ledger", "--help"],
    ];
    for args in cases {
        let out = Command::new(env!("CARGO_BIN_EXE_writfold"))
            .args(args)
            .env_remove("WRITFOLD_KB")
            .current_dir(std::env::temp_dir())
            .output()?;
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

/// The help's hook list is the plugin's: every `writfold kb hook <event>`
/// that `plugins/writfold/hooks/hooks.json` registers is named (#53).
#[test]
fn help_names_every_hook_the_plugin_registers() -> Result {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let hooks = std::fs::read_to_string(root.join("plugins/writfold/hooks/hooks.json"))?;
    let out = Command::new(env!("CARGO_BIN_EXE_writfold"))
        .args(["kb", "help"])
        .output()?;
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
