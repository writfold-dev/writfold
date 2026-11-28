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

/// The full reason of a delivery denial.
fn reason(why: String) -> String {
    format!("writfold delivery guard (a guard, not enforcement): {why}")
}

/// A delivery denial as Claude Code's PreToolUse output.
pub fn deny_output(reason: &str) -> HookOutput {
    let value = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    });
    HookOutput {
        stdout: format!("{value}\n"),
        stderr: String::new(),
    }
}

/// Decide one PreToolUse event: the denial's full reason, or `None`.
pub fn judge(
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<Option<String>, String> {
    let Some(guard) = config::guard(&ws.root).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let Some(matched) = body::matched(&guard, input, cwd) else {
        return Ok(None);
    };
    Ok(decide(ws, &matched, now).map(reason))
}

/// Decide one PreToolUse event as Claude Code's output.
pub fn pre_tool_use(
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<HookOutput, String> {
    Ok(judge(ws, input, cwd, now)?
        .map(|r| deny_output(&r))
        .unwrap_or_default())
}

fn decide(ws: &Workspace, m: &Matched, now: i64) -> Option<String> {
    let tool = match GitTool::locate() {
        Ok(tool) => tool,
        Err(e) => return Some(format!("cannot read the KB ({e})")),
    };
    let session = match Session::open(ws, Some(&tool), false, now) {
        Ok(session) => session,
        Err(e) => return Some(format!("cannot read the KB ({e})")),
    };
    let rec = crate::runs::current_of(&session)
        .and_then(|id| query::record(&session.conn, &id).ok().flatten());
    let Some(rec) = rec else {
        return Some(
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
    guarded::decide(&ws.root, &rec.id, &found, &call, &m.read)
}

/// D19: after a routed call returns, remind the agent to record the post
/// of the candidate it carried. Silent when no guard is configured, the
/// call matches no route or command, or no candidate holds its bytes.
pub fn post_tool_use(
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<HookOutput, String> {
    let Some(guard) = config::guard(&ws.root).map_err(|e| e.to_string())? else {
        return Ok(HookOutput::default());
    };
    let Some(m) = body::matched(&guard, input, cwd) else {
        return Ok(HookOutput::default());
    };
    let tool = GitTool::locate().map_err(|e| e.to_string())?;
    let session = Session::open(ws, Some(&tool), false, now).map_err(|e| e.to_string())?;
    let rec = crate::runs::current_of(&session)
        .and_then(|id| query::record(&session.conn, &id).ok().flatten());
    let Some(rec) = rec else {
        return Ok(HookOutput::default());
    };
    let found = crate::deliver::found::all(&session, &rec);
    let call = Call {
        name: &m.name,
        mode: m.mode.as_deref(),
        file_flag: m.file_flag.as_deref(),
        unread: m.unread(),
        spent_matches: false,
    };
    let Some(f) = guarded::posting(&found, &call, &m.read) else {
        return Ok(HookOutput::default());
    };
    let context = format!(
        "writfold: if {} posted it, record the post: writfold kb deliver posted {} --ref URL --by WHO (a statement; Writfold sees no post)",
        m.name, f.c.id
    );
    let value = json!({
        "hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": context}
    });
    Ok(HookOutput {
        stdout: format!("{value}\n"),
        stderr: String::new(),
    })
}
