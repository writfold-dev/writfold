//! `writfold kb hook <event>`: read the harness event's JSON on stdin and
//! hand it to `writfold_kb::hook`. Always exits 0; see that module for why.
use super::Outcome;
use std::io::Read;
use std::path::PathBuf;
use writfold_kb::hook::{HookEnv, run};

/// The most stdin a hook reads (a Stop event carries the last message).
const INPUT_LIMIT: u64 = 8 << 20;

/// The plugin's `kb_root` option reaches hooks under this name.
pub const KB_OPTION: &str = "CLAUDE_PLUGIN_OPTION_KB_ROOT";

/// Claude Code's session environment file, given to `SessionStart` hooks:
/// it runs the file before each Bash command of the session.
pub const ENV_FILE: &str = "CLAUDE_ENV_FILE";

pub fn main(args: &[String], cwd: PathBuf) -> Outcome {
    let [event] = args else {
        return Outcome {
            stdout: String::new(),
            stderr: "writfold kb hook: usage: writfold kb hook <event>\n".into(),
            code: 0,
        };
    };
    let mut input = String::new();
    let read = std::io::stdin()
        .lock()
        .take(INPUT_LIMIT)
        .read_to_string(&mut input);
    if let Err(error) = read {
        return Outcome {
            stdout: String::new(),
            stderr: format!("writfold kb hook: stdin: {error}\n"),
            code: 0,
        };
    }
    // Hand the session's ID to the shell commands the agent runs later.
    let mut exported = String::new();
    if event == "session-start"
        && let Some(file) = std::env::var_os(ENV_FILE).filter(|v| !v.is_empty())
        && let Err(error) = writfold_kb::hook::export_session(&PathBuf::from(file), &input)
    {
        exported = format!("writfold kb hook: {error}\n");
    }
    let kb = std::env::var_os(KB_OPTION)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    let output = run(
        event,
        &input,
        &HookEnv {
            kb,
            cwd,
            now: None,
            home: std::env::var_os("HOME").map(PathBuf::from),
        },
    );
    Outcome {
        stdout: output.stdout,
        stderr: format!("{exported}{}", output.stderr),
        code: 0,
    }
}
