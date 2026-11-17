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
