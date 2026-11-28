//! One event at the guard site (#183): the delivery guard, then the rules,
//! each judged where the command it matched acts ([`super::matched`]).
//! `kb hook pre-tool-use` renders the result as Claude Code expects;
//! `kb guard check` prints it, logs `via: check` and exits by it.
use super::{delivery, matched, pre, problems_note};
use crate::workspace::Workspace;
use serde_json::{Map, Value};
use std::path::Path;
use writfold_core::guard::Decision;
use writfold_core::site::Basis;

/// What the site decided for one event.
pub struct Sited {
    pub decision: Decision,
    pub basis: Basis,
    pub reasons: Vec<String>,
    pub stderr: String,
    /// The delivery guard denied it (its reason is the only one).
    pub delivery: bool,
    /// One row per judged rule.
    pub rules: Vec<Value>,
}

impl Sited {
    pub fn passed(basis: Basis, stderr: String) -> Self {
        Sited {
            decision: Decision::Pass,
            basis,
            reasons: Vec::new(),
            stderr,
            delivery: false,
            rules: Vec::new(),
        }
    }
}

/// Decide one event, given as the canonical map ([`super::event`]).
pub fn decide(
    ws: &Workspace,
    input: &Map<String, Value>,
    (cwd, home): (&Path, Option<&Path>),
    now: i64,
) -> Sited {
    let mut stderr = String::new();
    match delivery::judge(ws, input, cwd, now) {
        Ok(Some(reason)) => {
            return Sited {
                decision: Decision::Deny,
                basis: Basis::Judged(Decision::Deny),
                reasons: vec![reason],
                stderr,
                delivery: true,
                rules: Vec::new(),
            };
        }
        Ok(None) => {}
        Err(why) => stderr = format!("writfold kb hook: {why}\n"),
    }
    let call = matched::Call::read(input, cwd, home);
    let Some(loaded) = matched::Loaded::find(&ws.root, &call.places()) else {
        return Sited::passed(Basis::NoRules, stderr);
    };
    stderr.push_str(&problems_note(&loaded.problems));
    let groups = matched::groups(&loaded, &call);
    if groups.is_empty() {
        return Sited::passed(Basis::NoMatch, stderr);
    }
    let decided = pre::judge(ws, input, &groups, now);
    stderr.push_str(&decided.stderr);
    Sited {
        decision: decided.decision,
        basis: Basis::Judged(decided.decision),
        reasons: decided.reasons,
        stderr,
        delivery: false,
        rules: decided.rules,
    }
}
