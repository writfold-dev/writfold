//! G2b: `writfold kb hook pre-tool-use`, an optional guard for Claude
//! Code's PreToolUse event. It is a guard, not enforcement: it sees only
//! the tool calls the harness shows it, it reads a command line rather
//! than running it, and anything outside the configured commands passes.
//!
//! With no `.writfold/deliveries/guard.toml` in the KB (the default), or
//! no KB, it allows everything and prints nothing (fail open). With one:
//!
//! - a route (`routes`) names a posting command or tool, its mode and
//!   where its body is. The body must be read, and a postable candidate
//!   of that mode (same payload bytes, ready, not withdrawn, not yet
//!   recorded as posted) must hold it; a body it cannot read is denied;
//! - a listed command (`commands`) or tool (`tools`) keeps the lenient
//!   rule: known bytes need a postable candidate of any mode; unknown
//!   bytes need every live candidate of the run ready, and at least one.
//!
//! A command preceded by `cd`, `pushd` or a Git directory setting, or
//! carrying `-C`, acts somewhere else: its body file is never read from
//! the hook's directory ([`crate::shell::invocations`]).
//!
//! "Allow" means printing nothing, so Claude Code's own permission flow
//! still decides; a denial is the documented JSON with
//! `permissionDecision: "deny"` and the reason, which names the repair.
use super::delivery_body::{self as body, Matched};
use crate::deliver::config;
use crate::deliver::guarded::{self, Call};
use crate::hook::HookOutput;
use crate::index::query;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use serde_json::{Map, Value, json};
use std::path::Path;

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
    let Some(matched) = body::matched(&guard, input, cwd) else {
        return Ok(HookOutput::default());
    };
    Ok(decide(ws, &matched, now))
}

fn decide(ws: &Workspace, m: &Matched, now: i64) -> HookOutput {
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
    let found = crate::deliver::found::all(&session, &rec);
    let call = Call {
        name: &m.name,
        mode: m.mode.as_deref(),
        file_flag: m.file_flag.as_deref(),
        unread: m.unread(),
        spent_matches: false,
    };
    match guarded::decide(&ws.root, &rec.id, &found, &call, &m.read) {
        None => HookOutput::default(),
        Some(reason) => deny(reason),
    }
}
