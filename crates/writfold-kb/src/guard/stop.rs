//! A stop rule that blocks (#184): with `action = "block"` and `limit = N`
//! (default 1, at most 8) a `stop` rule whose requirement is unmet ends
//! the turn with `decision: "block"` and its reason, at most `limit` times
//! a turn ([`writfold_core::site::block_again`]). The count lives in the
//! session's draft: Claude Code marks a Stop that follows a block with
//! `stop_hook_active`, so a Stop without it starts a turn at zero. When the
//! limit is spent the turn ends, and the next session's inbox says which
//! rule was still unmet. A stop rule without `action` nudges as before.
use super::reason;
use super::require;
use super::rules::{Event, Rule};
use crate::draft;
use crate::workspace::Workspace;
use serde_json::{Map, Value};
use std::path::Path;
use writfold_core::guard::{Fact, all_met};
use writfold_core::site::block_again;

/// The draft key counting this turn's blocks.
pub const BLOCKED: &str = "stop_blocked";
/// The draft key naming the blocking rules a turn ended with unmet.
pub const SPENT: &str = "stop_block_spent";

fn count(d: &Map<String, Value>, active: bool) -> usize {
    match active {
        false => 0,
        true => d
            .get(BLOCKED)
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(0),
    }
}

/// The reason to block the end of this turn, or `None`.
pub fn blocks(
    ws: &Workspace,
    input: &Map<String, Value>,
    (cwd, local, session): (&Path, &Path, &str),
    now: i64,
    found: &[Rule],
) -> Result<Option<String>, String> {
    let rules: Vec<&Rule> = found
        .iter()
        .filter(|r| r.event == Event::Stop && r.block.is_some())
        .collect();
    if rules.is_empty() {
        return Ok(None);
    }
    let active = input
        .get("stop_hook_active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut d = draft::load(local, session)
        .ok()
        .flatten()
        .unwrap_or_default();
    let blocked = count(&d, active);
    let evaluated = require::evaluate(ws, cwd, now, &rules);
    let (mut reasons, mut spent) = (Vec::new(), Vec::new());
    for (rule, owed) in rules.iter().zip(evaluated) {
        let Some(owed) = owed else { continue };
        let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
        let unmet = !all_met(&facts);
        let limit = rule.block.unwrap_or(1);
        if block_again(unmet, blocked, limit) {
            let tail = format!(
                "a guard, not enforcement; block {} of {limit} this turn",
                blocked + 1
            );
            reasons.push(reason(rule, &owed, &tail));
        } else if unmet {
            spent.push(format!("{} ({})", rule.id, rule.path));
        }
    }
    let next = match reasons.is_empty() {
        true => 0,
        false => blocked + 1,
    };
    d.insert(BLOCKED.into(), Value::from(next));
    if !spent.is_empty() {
        let line = format!("{} {}", crate::time::stamp(now), spent.join(", "));
        d.insert(SPENT.into(), Value::from(line));
    }
    draft::save(local, session, &d).map_err(|e| e.to_string())?;
    Ok((!reasons.is_empty()).then(|| reasons.join("\n")))
}

/// The inbox lines for sessions whose turn ended with a blocking rule
/// still unmet.
pub fn inbox(drafts: &[(String, Map<String, Value>)]) -> Vec<String> {
    drafts
        .iter()
        .filter_map(|(session, d)| {
            let spent = d.get(SPENT).and_then(Value::as_str)?;
            Some(format!(
                "stop-block {session} {spent}: the turn ended with the rule unmet\n"
            ))
        })
        .collect()
}
