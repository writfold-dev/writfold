//! The guards behind `writfold kb hook pre-tool-use` and `stop`. Each is a
//! guard, not enforcement: it sees only the calls the harness shows it and
//! reads a command line rather than running it.
//!
//! - The delivery guard ([`delivery`], `.writfold/deliveries/guard.toml`)
//!   runs first, unchanged; its denial wins.
//! - Guard rules ([`rules`], `.writfold/guards/<id>.toml`) refuse a named
//!   command or tool until the facts they require hold ([`require`]):
//!   `deny` or `ask`, decided by [`writfold_core::guard::decide`]. With no
//!   rule directory nothing is opened (the fast path); an unparsable rule
//!   file is skipped with a diagnostic; a matched rule whose requirement
//!   cannot be evaluated takes its action. A rule with `paths` matches
//!   only a call whose file they admit ([`paths`]), decided before the
//!   index is opened.
//! - A `stop` rule adds context ([`stop`]) once per session, or with
//!   `repeat = "turn"` once per user turn, and never while
//!   `stop_hook_active` is set; the session flag is its local draft.
pub mod delivery;
pub mod paths;
pub mod require;
pub mod rules;
pub mod status;

/// One read-only guard verb.
#[derive(Clone, Debug)]
pub enum GuardVerb {
    Status,
}

use crate::draft;
use crate::hook::HookOutput;
use crate::workspace::Workspace;
use require::Owed;
use rules::{Event, Rule};
use serde_json::{Map, Value, json};
use std::path::Path;
use writfold_core::guard::{Decision, Fact, Repeat, combine, decide, nudge_with};

/// The draft field that records this session's Stop nudge.
pub const NUDGED: &str = "nudged";

/// Why a rule did not pass: its ID, description and what is owed.
fn reason(rule: &Rule, owed: &[Owed], tail: &str) -> String {
    let mut what: Vec<&str> = Vec::new();
    for o in owed.iter().filter(|o| o.fact != Fact::Met) {
        if !what.contains(&o.text.as_str()) {
            what.push(&o.text);
        }
    }
    let description = match rule.description.is_empty() {
        true => String::new(),
        false => format!("{}; ", rule.description),
    };
    format!(
        "writfold guard rule {} ({tail}): {description}owes: {} (rule: {})",
        rule.id,
        what.join("; "),
        rule.path
    )
}

fn quiet_with(stderr: String) -> HookOutput {
    HookOutput {
        stdout: String::new(),
        stderr,
    }
}

fn problems_note(problems: &[(String, String)]) -> String {
    problems
        .iter()
        .map(|(path, why)| format!("writfold kb hook: guard rule {path} skipped: {why}\n"))
        .collect()
}

/// Decide one PreToolUse event: the delivery guard, then the rules.
pub fn pre_tool_use(
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<HookOutput, String> {
    let mut stderr = match delivery::pre_tool_use(ws, input, cwd, now) {
        Ok(out) if !out.stdout.is_empty() => return Ok(out),
        Ok(out) => out.stderr,
        Err(why) => format!("writfold kb hook: {why}\n"),
    };
    let dirs = rules::dirs(&ws.root, cwd);
    if dirs.is_empty() {
        return Ok(quiet_with(stderr));
    }
    let (found, problems) = rules::read(&dirs);
    stderr.push_str(&problems_note(&problems));
    let tool = input.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let line = input
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let file = found
        .iter()
        .any(|r| !r.paths.is_empty())
        .then(|| paths::target(input, cwd))
        .flatten();
    let matched: Vec<&Rule> = found
        .iter()
        .filter(|r| r.event == Event::PreToolUse)
        .filter(|r| {
            (r.tools.iter().any(|t| t == tool) && paths::admits(&r.paths, file.as_deref()))
                || (tool == "Bash" && r.matches_line(line))
        })
        .collect();
    if matched.is_empty() {
        return Ok(quiet_with(stderr));
    }
    let evaluated = require::evaluate(ws, cwd, now, &matched);
    let mut decision = Decision::Pass;
    let mut reasons = Vec::new();
    for (rule, owed) in matched.iter().zip(evaluated) {
        let Some(owed) = owed else { continue };
        let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
        let decided = decide(true, rule.action, &facts);
        if decided != Decision::Pass {
            reasons.push(reason(rule, &owed, "a guard, not enforcement"));
            decision = combine(decision, decided);
        }
    }
    let verdict = match decision {
        Decision::Pass => return Ok(quiet_with(stderr)),
        Decision::Deny => "deny",
        Decision::Ask => "ask",
    };
    let value = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": verdict,
            "permissionDecisionReason": reasons.join("\n"),
        }
    });
    Ok(HookOutput {
        stdout: format!("{value}\n"),
        stderr,
    })
}

/// The Stop nudge: when a `stop` rule exists and its requirements are not
/// all met, add context once per session (or, with `repeat = "turn"`,
/// once per user turn). Called after the Stop draft is
/// saved; with no stop rule it opens nothing and spawns nothing.
pub fn stop(
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<HookOutput, String> {
    let dirs = rules::dirs(&ws.root, cwd);
    if dirs.is_empty() {
        return Ok(HookOutput::default());
    }
    let (found, problems) = rules::read(&dirs);
    let stderr = problems_note(&problems);
    let stops: Vec<&Rule> = found.iter().filter(|r| r.event == Event::Stop).collect();
    if stops.is_empty() {
        return Ok(quiet_with(stderr));
    }
    let active = input
        .get("stop_hook_active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let local = match &ws.repo {
        Some(repo) => repo.git_dir.join("writfold"),
        None => return Ok(quiet_with(stderr)),
    };
    let session = input
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let Ok(Some(mut d)) = draft::load(&local, session) else {
        return Ok(quiet_with(stderr));
    };
    let nudged = d.contains_key(NUDGED);
    // Only the rules that may still nudge are evaluated: none while a stop
    // hook is already continuing the turn, and no once-a-session rule
    // after the session's nudge.
    let due: Vec<&Rule> = stops
        .into_iter()
        .filter(|r| nudge_with(r.repeat, active, nudged, &[Fact::Unmet]))
        .collect();
    if due.is_empty() {
        return Ok(quiet_with(stderr));
    }
    let evaluated = require::evaluate(ws, cwd, now, &due);
    let mut texts = Vec::new();
    let mut again = false;
    for (rule, owed) in due.iter().zip(evaluated) {
        let Some(owed) = owed else { continue };
        let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
        if nudge_with(rule.repeat, active, nudged, &facts) {
            let tail = match rule.repeat {
                Repeat::Session => "a guard, not enforcement; once this session",
                Repeat::Turn => "a guard, not enforcement; once a turn",
            };
            again |= rule.repeat == Repeat::Turn;
            texts.push(reason(rule, &owed, tail));
        }
    }
    if texts.is_empty() {
        return Ok(quiet_with(stderr));
    }
    d.insert(NUDGED.into(), crate::time::stamp(now).into());
    draft::save(&local, session, &d).map_err(|e| e.to_string())?;
    texts.push(
        match again {
            false => "Act on this or say why not; Writfold will not raise it again this session.",
            true => "Act on this or say why not; Writfold will not raise it again this turn.",
        }
        .into(),
    );
    let value = json!({
        "hookSpecificOutput": {
            "hookEventName": "Stop",
            "additionalContext": texts.join("\n"),
        }
    });
    Ok(HookOutput {
        stdout: format!("{value}\n"),
        stderr,
    })
}
