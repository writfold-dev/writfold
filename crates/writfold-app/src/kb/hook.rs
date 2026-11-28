//! `writfold kb hook <event>`: read the harness event's JSON on stdin and
//! hand it to `writfold_kb::hook`. Always exits 0; see that module for why.
use super::Outcome;
use std::io::Read;
use std::path::PathBuf;
use writfold_kb::guard::check::Posture;
use writfold_kb::hook::{HookEnv, run_with};

/// The most stdin a hook reads (a Stop event carries the last message).
const INPUT_LIMIT: u64 = 8 << 20;

/// The plugin's `kb_root` option reaches hooks under this name.
pub const KB_OPTION: &str = "CLAUDE_PLUGIN_OPTION_KB_ROOT";

/// Claude Code's session environment file, given to `SessionStart` hooks:
/// it runs the file before each Bash command of the session.
pub const ENV_FILE: &str = "CLAUDE_ENV_FILE";

/// The plugin's `posture` option reaches hooks under this name (#184).
pub const POSTURE_OPTION: &str = "CLAUDE_PLUGIN_OPTION_POSTURE";

/// The registration's posture: `--posture P`, else the plugin's option,
/// else `open`. An unknown word is `deny`: a registration that asked for a
/// posture asked for one at least as strict.
fn posture(flag: Option<&String>) -> Posture {
    let written = flag
        .cloned()
        .or_else(|| std::env::var(POSTURE_OPTION).ok().filter(|v| !v.is_empty()));
    match written {
        None => Posture::Open,
        Some(text) => Posture::parse(&text).unwrap_or(Posture::Deny),
    }
}

pub fn main(args: &[String], cwd: PathBuf) -> Outcome {
    let (event, flag) = match args {
        [event] => (event, None),
        [event, key, value] if key == "--posture" => (event, Some(value)),
        _ => {
            return Outcome {
                stdout: String::new(),
                stderr:
                    "writfold kb hook: usage: writfold kb hook <event> [--posture open|ask|deny]\n"
                        .into(),
                code: 0,
            };
        }
    };
    let posture = posture(flag);
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
    let output = run_with(
        event,
        &input,
        &HookEnv {
            kb,
            cwd,
            now: None,
            home: std::env::var_os("HOME").map(PathBuf::from),
        },
        posture,
    );
    Outcome {
        stdout: output.stdout,
        stderr: format!("{exported}{}", output.stderr),
        code: 0,
    }
}
