//! Deciding the matched pre-tool-use rules of one call: each rule's
//! decision from its facts, its effect by its mode (an audit rule never
//! decides the call), the stderr lines for audit rules and for passes
//! that rested on a waiver, and the guard log's entries.
use super::matched::Group;
use super::require::{self, Context, Owed, Place};
use super::rules::Rule;
use super::{log, reason};
use crate::workspace::Workspace;
use serde_json::{Map, Value, json};
use writfold_core::guard::{Decision, Fact, Mode, combine, decide, effect};

/// What the rules decided for the call.
pub struct Decided {
    pub decision: Decision,
    pub reasons: Vec<String>,
    pub stderr: String,
}

fn name(d: Decision) -> &'static str {
    match d {
        Decision::Pass => "pass",
        Decision::Deny => "deny",
        Decision::Ask => "ask",
    }
}

/// One log entry for a rule judged at a place.
pub struct Logged<'a> {
    pub via: &'static str,
    pub session: &'a str,
    pub tool: &'a str,
    pub command: &'a str,
    pub dir: Option<String>,
    pub context: &'a Context,
}

/// The guard log entry for one judged rule.
pub fn entry(ws: &Workspace, rule: &Rule, owed: &[Owed], at: &Logged<'_>, now: i64) -> Value {
    let decided = decide(
        true,
        rule.action,
        &owed.iter().map(|o| o.fact).collect::<Vec<_>>(),
    );
    let owes: Vec<&str> = owed
        .iter()
        .filter(|o| o.fact != Fact::Met)
        .map(|o| o.text.as_str())
        .collect();
    let waived: Vec<String> = owed
        .iter()
        .flat_map(|o| &o.waived)
        .map(|w| format!("gate {}: {} @{}", w.gate, w.text, w.path))
        .collect();
    json!({"schema": log::SCHEMA, "at": crate::time::stamp(now), "via": at.via,
        "session": at.session, "rule": rule.id, "rule_path": rule.path,
        "rule_digest": log::digest(&log::file(ws, rule)), "mode": rule.mode.name(),
        "decided": name(decided), "effect": name(effect(rule.mode, decided)),
        "tool": at.tool, "command": log::command(at.command), "dir": at.dir,
        "repo": at.context.repo, "run": at.context.run, "owes": owes, "waived": waived})
}

/// Whether a judged rule is logged: every audit evaluation, every
/// enforcing decision that is not a pass, every pass on a waiver.
pub fn logged(rule: &Rule, owed: &[Owed]) -> bool {
    let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
    let decided = decide(true, rule.action, &facts);
    let waived = owed.iter().any(|o| !o.waived.is_empty());
    rule.mode == Mode::Audit || decided != Decision::Pass || waived
}

/// The stderr lines for a judged rule that let the call through.
pub fn notes(rule: &Rule, owed: &[Owed], text: &str) -> String {
    let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
    let decided = decide(true, rule.action, &facts);
    let mut out = String::new();
    if rule.mode == Mode::Audit && decided != Decision::Pass {
        out.push_str(&format!(
            "writfold kb hook: guard rule {} (audit) would {}: {text}\n",
            rule.id,
            name(decided)
        ));
    }
    if decided == Decision::Pass {
        for w in owed.iter().flat_map(|o| &o.waived) {
            out.push_str(&format!(
                "writfold kb hook: guard rule {} passed on a waiver: gate {}\n",
                rule.id, w.gate
            ));
        }
    }
    out
}

/// Decide every group, log what the log keeps, and say what passed only
/// because of the mode or a waiver.
pub fn judge(
    ws: &Workspace,
    input: &Map<String, Value>,
    groups: &[Group<'_>],
    now: i64,
) -> Decided {
    let pairs: Vec<(Place, Vec<&Rule>)> = groups
        .iter()
        .map(|g| (g.place.clone(), g.rules.clone()))
        .collect();
    let evaluated = require::evaluate_at(ws, now, &pairs);
    let session = input
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let tool = input.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let mut out = Decided {
        decision: Decision::Pass,
        reasons: Vec::new(),
        stderr: String::new(),
    };
    let mut entries = Vec::new();
    for (group, (results, context)) in groups.iter().zip(evaluated) {
        let dir = match &group.place {
            Place::At(d) => Some(d.display().to_string()),
            Place::Unknown(_) => None,
        };
        for ((rule, owed), command) in group.rules.iter().zip(results).zip(&group.commands) {
            let Some(owed) = owed else { continue };
            let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
            let decided = decide(true, rule.action, &facts);
            let text = reason(rule, &owed, "a guard, not enforcement");
            out.stderr.push_str(&notes(rule, &owed, &text));
            if effect(rule.mode, decided) != Decision::Pass && !out.reasons.contains(&text) {
                out.reasons.push(text);
            }
            out.decision = combine(out.decision, effect(rule.mode, decided));
            if logged(rule, &owed) {
                let at = Logged {
                    via: "hook",
                    session,
                    tool,
                    command,
                    dir: dir.clone(),
                    context: &context,
                };
                entries.push(entry(ws, rule, &owed, &at, now));
            }
        }
    }
    if let Some(path) = log::path(ws)
        && let Err(why) = log::append(&path, &entries)
    {
        out.stderr.push_str(&format!(
            "writfold kb hook: {why}; the decision is unchanged\n"
        ));
    }
    out
}
