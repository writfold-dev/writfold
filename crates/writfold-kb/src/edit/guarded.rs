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
use crate::guard::{log, paths, pre, reason, require};
use crate::workspace::Workspace;
use std::path::Path;
use writfold_core::guard::{Action, Decision, Fact, Mode, decide};

/// The tool name a rule lists to cover `edit apply`.
pub const TOOL: &str = "writfold:edit";

const ASKS: &str = "the rule asks, and edit apply cannot ask: meet the requirement or make the edit with a tool your harness can ask about";

/// The guard log entry for one judged rule, when the log keeps it.
fn logged(
    ws: &Workspace,
    (rule, owed): (&Rule, &[require::Owed]),
    hits: &[String],
    (top, context): (&Path, &require::Context),
    now: i64,
) -> Option<serde_json::Value> {
    if !pre::logged(rule, owed) {
        return None;
    }
    let command = hits.join(" ");
    let at = pre::Logged {
        via: "edit-apply",
        session: "",
        tool: TOOL,
        command: &command,
        dir: Some(top.display().to_string()),
        context,
    };
    Some(pre::entry(ws, rule, owed, &at, now))
}

/// Append to the guard log; a log that cannot be written changes nothing.
fn append(ws: &Workspace, entries: &[serde_json::Value]) {
    if let Some(path) = log::path(ws)
        && let Err(why) = log::append(&path, entries)
    {
        eprintln!("writfold kb hook: {why}; the decision is unchanged");
    }
}

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
    let groups = [(require::Place::At(top.to_path_buf()), rules)];
    let (evaluated, context) = require::evaluate_at(ws, now, &groups)
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut refusals = Vec::new();
    let mut audited = Vec::new();
    let mut entries = Vec::new();
    for ((rule, hits), owed) in matched.iter().zip(evaluated) {
        let Some(owed) = owed else { continue };
        entries.extend(logged(ws, (rule, &owed), hits, (top, &context), now));
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
        match rule.mode {
            Mode::Audit => audited.push(format!(
                "guard rule {} (audit) would refuse this edit: {text}; applied and logged",
                rule.id
            )),
            Mode::Enforce => refusals.push((rule.id.clone(), text)),
        }
    }
    append(ws, &entries);
    match (refusals.is_empty(), dry_run) {
        (true, _) => Ok(audited),
        (false, true) => Ok(refusals
            .into_iter()
            .map(|(id, text)| format!("guard rule {id} would refuse this edit: {text}"))
            .chain(audited)
            .collect()),
        (false, false) => {
            let texts: Vec<String> = refusals.into_iter().map(|(_, t)| t).collect();
            Err(KbError::Refused(texts.join("\n")))
        }
    }
}
