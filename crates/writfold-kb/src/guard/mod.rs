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
pub mod binding;
pub mod check;
pub mod delivery;
mod delivery_body;
mod empty;
pub mod event;
pub mod log;
pub mod matched;
pub mod paths;
pub mod posture;
pub(crate) mod pre;
mod report;
pub mod require;
pub mod rule_check;
mod rule_lists;
pub mod rules;
pub mod site;
pub mod status;
pub mod waived;

/// One read-only guard verb.
#[derive(Clone, Debug)]
pub enum GuardVerb {
    Status,
    /// `guard log [--rule ID] [--since AGE]` (AGE in seconds).
    Log {
        rule: Option<String>,
        since: Option<i64>,
    },
}

use crate::draft;
use crate::hook::HookOutput;
use crate::workspace::Workspace;
pub use report::execute as log_report;
use require::Owed;
use rules::{Event, Rule};
use serde_json::{Map, Value, json};
use std::path::Path;
use writfold_core::guard::{Decision, Fact, Repeat, nudge_with};

/// The draft field that records this session's Stop nudge.
pub const NUDGED: &str = "nudged";

/// Why a rule did not pass: its ID, description and what is owed.
pub(crate) fn reason(rule: &Rule, owed: &[Owed], tail: &str) -> String {
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

fn problems_note(problems: &[crate::defs::Problem]) -> String {
    problems.iter().map(crate::hook::broken::skipped).collect()
}

/// Decide one PreToolUse event ([`site::decide`]) under the
/// registration's posture and render it as Claude Code expects.
pub fn pre_tool_use(
    ws: &Workspace,
    input: &Map<String, Value>,
    places: (&Path, Option<&Path>),
    now: i64,
    posture: writfold_core::site::Posture,
) -> HookOutput {
    render(site::decide(ws, input, places, now, posture))
}

/// A site's decision as Claude Code's PreToolUse output: nothing on a
/// pass, its JSON on a denial or a question.
pub fn render(sited: site::Sited) -> HookOutput {
    if sited.delivery {
        let reason = sited.reasons.first().map(String::as_str).unwrap_or("");
        return delivery::deny_output(reason);
    }
    let verdict = match sited.decision {
        Decision::Pass => return quiet_with(sited.stderr),
        Decision::Deny => "deny",
        Decision::Ask => "ask",
    };
    let value = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": verdict,
            "permissionDecisionReason": sited.reasons.join("\n"),
        }
    });
    HookOutput {
        stdout: format!("{value}\n"),
        stderr: sited.stderr,
    }
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
    let local = match &ws.repo {
        Some(repo) => repo.git_dir.join("writfold"),
        None => return Ok(quiet_with(stderr)),
    };
    let session = input
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut texts = nudges(ws, input, (cwd, &local, session), now, &found)?;
    if let Some(block) = broken_once(&local, session, &problems, now)? {
        texts.insert(0, block);
    }
    if texts.is_empty() {
        return Ok(quiet_with(stderr));
    }
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

/// The draft key that records the session was told of broken rules.
const BROKEN_NOTED: &str = "broken_rules_noted";

/// The block of guard rule files that do not parse, once per session.
fn broken_once(
    local: &Path,
    session: &str,
    problems: &[crate::defs::Problem],
    now: i64,
) -> Result<Option<String>, String> {
    if problems.is_empty() {
        return Ok(None);
    }
    let Ok(Some(mut d)) = draft::load(local, session) else {
        return Ok(None);
    };
    if d.contains_key(BROKEN_NOTED) {
        return Ok(None);
    }
    let broken = crate::hook::broken::GUARD;
    let lines: Vec<String> = problems
        .iter()
        .map(|p| crate::hook::broken::entry(p, broken))
        .collect();
    d.insert(BROKEN_NOTED.into(), crate::time::stamp(now).into());
    draft::save(local, session, &d).map_err(|e| e.to_string())?;
    Ok(Some(
        crate::hook::broken::block(&lines).trim_end().to_owned(),
    ))
}

/// The stop rules' nudges that are due, with the closing line; empty
/// when none is.
fn nudges(
    ws: &Workspace,
    input: &Map<String, Value>,
    (cwd, local, session): (&Path, &Path, &str),
    now: i64,
    found: &[Rule],
) -> Result<Vec<String>, String> {
    let stops: Vec<&Rule> = found.iter().filter(|r| r.event == Event::Stop).collect();
    if stops.is_empty() {
        return Ok(Vec::new());
    }
    let active = input
        .get("stop_hook_active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let Ok(Some(mut d)) = draft::load(local, session) else {
        return Ok(Vec::new());
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
        return Ok(Vec::new());
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
        return Ok(texts);
    }
    d.insert(NUDGED.into(), crate::time::stamp(now).into());
    draft::save(local, session, &d).map_err(|e| e.to_string())?;
    texts.push(
        match again {
            false => "Act on this or say why not; Writfold will not raise it again this session.",
            true => "Act on this or say why not; Writfold will not raise it again this turn.",
        }
        .into(),
    );
    Ok(texts)
}
