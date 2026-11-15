//! Guard rules over `edit apply`: a `pre-tool-use` rule whose `tools`
//! lists the reserved name [`TOOL`] applies to the recipes' targets, which
//! the verb knows exactly before anything is written, so the rule holds on
//! every harness, with or without hooks. The rules are those of the KB
//! root and of the nearest `.writfold/guards/` at or above the worktree's
//! top; a rule's `paths` are matched against the targets relative to that
//! top; the requirements are evaluated as the hook evaluates them, and
//! the kernel decides. It stays a guard, not enforcement: other tools can
//! still edit the file.
use crate::error::{KbError, Result};
use crate::guard::rules::{self, Event, Rule};
use crate::guard::{paths, reason, require};
use crate::workspace::Workspace;
use std::path::Path;
use writfold_core::guard::{Action, Decision, Fact, decide};

/// The tool name a rule lists to cover `edit apply`.
pub const TOOL: &str = "writfold:edit";

const ASKS: &str = "the rule asks, and edit apply cannot ask: meet the requirement or make the edit with a tool your harness can ask about";

/// Judge the rules naming [`TOOL`] for a batch with these `targets`.
/// Nothing is read when no rule directory applies. A refusal is an error
/// (exit 3) unless `dry_run`, which returns one note per refusing rule.
pub fn check(
    ws: &Workspace,
    top: &Path,
    now: i64,
    targets: &[String],
    dry_run: bool,
) -> Result<Vec<String>> {
    let dirs = rules::dirs(&ws.root, top);
    if dirs.is_empty() {
        return Ok(Vec::new());
    }
    let (found, problems) = rules::read(&dirs);
    for (path, why) in problems {
        eprintln!("writfold kb hook: guard rule {path} skipped: {why}");
    }
    let hit = |r: &Rule| -> Vec<String> {
        let admitted = targets.iter().filter(|t| paths::admits(&r.paths, Some(t)));
        admitted.cloned().collect()
    };
    let matched: Vec<(&Rule, Vec<String>)> = found
        .iter()
        .filter(|r| r.event == Event::PreToolUse && r.tools.iter().any(|t| t == TOOL))
        .map(|r| (r, hit(r)))
        .filter(|(_, hits)| !hits.is_empty())
        .collect();
    if matched.is_empty() {
        return Ok(Vec::new());
    }
    let rules: Vec<&Rule> = matched.iter().map(|(r, _)| *r).collect();
    let evaluated = require::evaluate(ws, top, now, &rules);
    let mut refusals = Vec::new();
    for ((rule, hits), owed) in matched.iter().zip(evaluated) {
        let Some(owed) = owed else { continue };
        let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
        let decided = decide(true, rule.action, &facts);
        if decided == Decision::Pass {
            continue;
        }
        let mut text = reason(rule, &owed, "a guard, not enforcement");
        text.push_str(&format!("; targets: {}", hits.join(", ")));
        if rule.action == Action::Ask {
            text.push_str(&format!("; {ASKS}"));
        }
        refusals.push((rule.id.clone(), text));
    }
    match (refusals.is_empty(), dry_run) {
        (true, _) => Ok(Vec::new()),
        (false, true) => Ok(refusals
            .into_iter()
            .map(|(id, text)| format!("guard rule {id} would refuse this edit: {text}"))
            .collect()),
        (false, false) => {
            let texts: Vec<String> = refusals.into_iter().map(|(_, t)| t).collect();
            Err(KbError::Refused(texts.join("\n")))
        }
    }
}
