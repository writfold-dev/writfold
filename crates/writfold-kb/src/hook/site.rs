//! `kb hook pre-tool-use` under a posture (#184). Under `open` (the
//! default) the hook decides as it always has. Under `ask` or `deny` an
//! event it cannot decide gets the posture's decision with the cause:
//! input that is not an event, no KB, and an answer not ready by the
//! deadline (`WRITFOLD_HOOK_DEADLINE_MS`, 8 s here, below the plugin's
//! 10 s timeout). A hook that is killed or never runs is outside what the
//! binary can close.
use super::HookOutput;
use crate::guard::site::Sited;
use crate::workspace::Workspace;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use writfold_core::site::Posture;

/// The default deadline under a closed posture, in milliseconds.
pub const DEADLINE_MS: u64 = 8_000;

/// The deadline: `WRITFOLD_HOOK_DEADLINE_MS`, else `default` ms.
pub fn deadline(default: u64) -> std::time::Duration {
    let ms = std::env::var("WRITFOLD_HOOK_DEADLINE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default);
    std::time::Duration::from_millis(ms)
}

/// An event the hook could not decide, under `posture`: the posture's
/// decision, rendered; under `open`, the quiet note it always was.
pub fn undecided(posture: Posture, why: String) -> HookOutput {
    let note = format!("writfold kb hook: {why}\n");
    match posture {
        Posture::Open => HookOutput {
            stdout: String::new(),
            stderr: note,
        },
        _ => crate::guard::render(Sited::undecided(posture, why, note)),
    }
}

/// Decide one PreToolUse event; under a closed posture, within the
/// deadline. The decision goes on in a thread the hook abandons at the
/// deadline (as session start's brief does, #69).
pub fn pre_tool_use(
    ws: &Workspace,
    input: &Map<String, Value>,
    (cwd, home): (&Path, Option<&Path>),
    now: i64,
    posture: Posture,
) -> HookOutput {
    if posture == Posture::Open {
        return crate::guard::pre_tool_use(ws, input, (cwd, home), now, posture);
    }
    let owned = (
        ws.clone(),
        input.clone(),
        cwd.to_path_buf(),
        home.map(PathBuf::from),
    );
    let (send, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (ws, input, cwd, home) = owned;
        let out = crate::guard::pre_tool_use(&ws, &input, (&cwd, home.as_deref()), now, posture);
        #[expect(
            clippy::let_underscore_must_use,
            reason = "a send after the deadline has no reader, by design"
        )]
        let _ = send.send(out);
    });
    let limit = deadline(DEADLINE_MS);
    answer.recv_timeout(limit).unwrap_or_else(|_| {
        let why = format!("the guard did not decide within {} ms", limit.as_millis());
        undecided(posture, why)
    })
}
