//! G2b: `writfold kb hook pre-tool-use`, an optional guard for Claude
//! Code's PreToolUse event. It is a guard, not enforcement: it sees only
//! the tool calls the harness shows it, it reads a command line rather
//! than running it, and anything outside the configured commands passes.
//!
//! With no `.writfold/deliveries/guard.toml` in the KB (the default), or
//! no KB, it allows everything and prints nothing (fail open). With one,
//! a Bash command whose simple command starts with a configured command
//! (such as `gh pr comment`), or a call to a configured tool, is denied
//! unless the current run has a delivery candidate that is ready:
//!
//! - when the body can be read from the command (`--body TEXT`, `-b TEXT`,
//!   `--body-file PATH`, `-F PATH`, without shell expansions), a ready
//!   candidate must hold those bytes (trailing whitespace ignored);
//! - otherwise every candidate of the run must be ready, and there must be
//!   at least one.
//!
//! "Allow" means printing nothing, so Claude Code's own permission flow
//! still decides; a denial is the documented JSON with
//! `permissionDecision: "deny"` and the reason.
use crate::deliver;
use crate::deliver::config::{self, Guard};
use crate::hook::HookOutput;
use crate::index::query;
use crate::session::Session;
use crate::shell::{self, Word};
use crate::workspace::{GitTool, Workspace};
use serde_json::{Map, Value, json};
use std::path::Path;

/// The body flags read from a matched command, with whether each names a
/// file.
const BODY_FLAGS: [(&str, bool); 4] = [
    ("--body", false),
    ("-b", false),
    ("--body-file", true),
    ("-F", true),
];

fn deny(reason: String) -> HookOutput {
    let value = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": format!("writfold delivery guard (a guard, not enforcement): {reason}"),
        }
    });
    HookOutput {
        stdout: format!("{value}\n"),
        stderr: String::new(),
    }
}

/// What the matched call would post.
enum Body {
    Bytes(Vec<u8>),
    Unknown(String),
}

/// The words of the first simple command that starts with a configured
/// command, if any.
fn matched(guard: &Guard, line: &str) -> Option<Vec<Word>> {
    shell::segments(line).into_iter().find_map(|segment| {
        let words = shell::command(&segment);
        guard
            .commands
            .iter()
            .any(|c| c.len() <= words.len() && c.iter().zip(words).all(|(a, w)| *a == w.text))
            .then(|| words.to_vec())
    })
}

/// The body a matched command names, read from its flags.
fn body(words: &[Word], cwd: &Path) -> Body {
    for (i, word) in words.iter().enumerate() {
        for (flag, file) in BODY_FLAGS {
            let value = match word.text.strip_prefix(flag) {
                Some("") => words.get(i + 1),
                Some(rest) if flag.starts_with("--") && rest.starts_with('=') => {
                    return value_of(&word.text[flag.len() + 1..], word.expands, file, cwd);
                }
                _ => continue,
            };
            let Some(value) = value else {
                return Body::Unknown(format!("{flag} has no value"));
            };
            return value_of(&value.text, value.expands, file, cwd);
        }
    }
    Body::Unknown("no body flag the guard can read".into())
}

fn value_of(text: &str, expands: bool, file: bool, cwd: &Path) -> Body {
    if expands {
        return Body::Unknown("the body holds a shell expansion".into());
    }
    if !file {
        return Body::Bytes(text.as_bytes().to_vec());
    }
    if text == "-" {
        return Body::Unknown("the body is read from standard input".into());
    }
    match std::fs::read(cwd.join(text)) {
        Ok(bytes) => Body::Bytes(bytes),
        Err(e) => Body::Unknown(format!("the body file {text} is unreadable ({e})")),
    }
}

fn trimmed(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(0, |i| i + 1);
    &bytes[..end]
}

/// Decide one PreToolUse event.
pub fn pre_tool_use(
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<HookOutput, String> {
    let Some(guard) = config::guard(&ws.root).map_err(|e| e.to_string())? else {
        return Ok(HookOutput::default());
    };
    let tool = input.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let body = if guard.tools.iter().any(|t| t == tool) {
        Body::Unknown(format!("tool {tool} is guarded as a whole"))
    } else if tool == "Bash" {
        let line = input
            .get("tool_input")
            .and_then(|t| t.get("command"))
            .and_then(Value::as_str)
            .unwrap_or("");
        match matched(&guard, line) {
            Some(words) => body(&words, cwd),
            None => return Ok(HookOutput::default()),
        }
    } else {
        return Ok(HookOutput::default());
    };
    Ok(decide(ws, body, now))
}

fn decide(ws: &Workspace, body: Body, now: i64) -> HookOutput {
    let tool = match GitTool::locate() {
        Ok(tool) => tool,
        Err(e) => return deny(format!("cannot read the KB ({e})")),
    };
    let session = match Session::open(ws, Some(&tool), false, now) {
        Ok(session) => session,
        Err(e) => return deny(format!("cannot read the KB ({e})")),
    };
    let rec = crate::runs::current_of(&session)
        .and_then(|id| query::record(&session.conn, &id).ok().flatten());
    let Some(rec) = rec else {
        return deny(
            "no current KB run: start one, then writfold kb deliver prepare --mode M --from FILE"
                .into(),
        );
    };
    let cands = deliver::readiness(&ws.root, &rec);
    let blocked = |c: &deliver::candidate::Candidate, owed: &[String]| {
        format!("candidate {} owes {}", c.id, owed.join(", "))
    };
    match body {
        Body::Bytes(bytes) => {
            let same: Vec<_> = cands
                .iter()
                .filter(|(c, _)| c.bytes.as_deref().map(trimmed) == Some(trimmed(&bytes)))
                .collect();
            if same.iter().any(|(_, owed)| owed.is_empty()) {
                return HookOutput::default();
            }
            match same.first() {
                Some((c, owed)) => deny(format!(
                    "{}; see writfold kb deliver status --run {}",
                    blocked(c, owed),
                    rec.id
                )),
                None => deny(format!(
                    "no delivery candidate of run {} holds the bytes being posted ({}); prepare and check one with writfold kb deliver",
                    rec.id,
                    deliver::candidate::sha(&bytes)
                )),
            }
        }
        Body::Unknown(why) => {
            let owing: Vec<String> = cands
                .iter()
                .filter(|(_, owed)| !owed.is_empty())
                .map(|(c, owed)| blocked(c, owed))
                .collect();
            if !cands.is_empty() && owing.is_empty() {
                return HookOutput::default();
            }
            let what = match owing.is_empty() {
                true => format!("run {} has no delivery candidate", rec.id),
                false => owing.join("; "),
            };
            deny(format!(
                "{what} ({why}, so every candidate of the run must be ready)"
            ))
        }
    }
}
