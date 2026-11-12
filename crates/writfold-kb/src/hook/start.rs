//! `session-start`: what a session begins with, all model-free and read
//! from local files and refs.
//!
//! Parts, in the order they are shown:
//!
//! 1. **Behind remote** ([`super::remote`]): one line when the KB's branch
//!    lags its upstream as last fetched; one Git spawn, no fetch.
//! 2. **Brief** (`brief`, budget [`BRIEF_BUDGET`] tokens), as before.
//! 3. **Current run** (`resume`, budget [`RESUME_BUDGET`]), when the
//!    current-run pointer is set.
//! 4. **Plan position** ([`super::position`]): the current step and what
//!    it owes, from `plan status`, when the current run has a plan.
//! 5. **Inbox** ([`super::inbox`]): drafts, pending proposals and runs left
//!    without an evaluation.
//!
//! The whole stays within [`CONTEXT_LIMIT`] characters. When it would not,
//! parts are cut (each at a line, with a marker) in [`DROP_ORDER`]: the
//! brief first, then the run, the inbox, the plan position, and the
//! behind-remote line last.
//!
//! After a compaction (`source` = `"compact"`) only the plan position is
//! injected again: the earlier context was summarised, not dropped, so the
//! brief is not sent twice, but the exact step and its owed gates are the
//! part a summary is most likely to blur.
use super::{HookOutput, inbox, position, remote, state, verb};
use crate::plan::PlanVerb;
use crate::workspace::Workspace;
use crate::{Verb, runs};
use serde_json::json;
use std::path::Path;

/// The most characters `session-start` injects.
pub const CONTEXT_LIMIT: usize = 9000;
/// Token budgets of the verbs `session-start` runs.
pub const BRIEF_BUDGET: usize = 1200;
pub const RESUME_BUDGET: usize = 500;
pub const PLAN_BUDGET: usize = 600;

/// The parts, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Behind,
    Brief,
    Run,
    Plan,
    Inbox,
}

/// Which part is cut first when the whole is over the limit.
pub const DROP_ORDER: [Part; 5] = [
    Part::Brief,
    Part::Run,
    Part::Inbox,
    Part::Plan,
    Part::Behind,
];

/// Fit the parts (in display order) within `limit` characters: parts are
/// kept whole from the last of [`DROP_ORDER`] backwards while they fit;
/// the first that does not is cut at a line with a marker, and the rest
/// get what remains (nothing, once too little is left for a marker).
pub fn fit(parts: &[(Part, String)], limit: usize) -> String {
    let mut kept: Vec<String> = vec![String::new(); parts.len()];
    let mut left = limit;
    for part in DROP_ORDER.iter().rev() {
        let Some(i) = parts.iter().position(|(p, _)| p == part) else {
            continue;
        };
        let text = &parts[i].1;
        let size = text.chars().count();
        kept[i] = if size <= left {
            text.clone()
        } else if left > 120 {
            super::cap(text, left)
        } else {
            String::new()
        };
        left = left.saturating_sub(kept[i].chars().count());
    }
    kept.concat()
}

/// The plan position block of the current run, if it has a plan.
fn plan_block(ws: &Workspace, cwd: &Path, now: i64, run: &str, header: &str) -> Option<String> {
    let status = Verb::Plan(PlanVerb::Status {
        run: Some(run.to_owned()),
        repo: None,
    });
    let text = verb(ws, cwd, now, status, PLAN_BUDGET)?;
    position::block(&text, run, header)
}

pub fn session_start(
    ws: &Workspace,
    cwd: &Path,
    now: i64,
    source: Option<&str>,
) -> Result<HookOutput, String> {
    let local = state(ws).ok();
    let current = local.as_deref().and_then(runs::current);
    let parts = if source == Some("compact") {
        let run = current.as_deref().ok_or("no current run to re-inject")?;
        let header = "Plan position after compaction (`writfold kb plan status`; `writfold kb resume` has the rest)";
        let block = plan_block(ws, cwd, now, run, header).ok_or("no plan to re-inject")?;
        vec![(Part::Plan, block.trim_start().to_owned())]
    } else {
        startup(ws, cwd, now, local.as_deref(), current.as_deref())
    };
    let context = fit(&parts, CONTEXT_LIMIT);
    if context.is_empty() {
        return Err("no brief, run, plan or inbox to inject".into());
    }
    let out = json!({"hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": context,
    }});
    Ok(HookOutput {
        stdout: format!("{out}\n"),
        stderr: String::new(),
    })
}

fn startup(
    ws: &Workspace,
    cwd: &Path,
    now: i64,
    local: Option<&Path>,
    current: Option<&str>,
) -> Vec<(Part, String)> {
    let mut parts = Vec::new();
    if let Some(line) = ws.repo.as_ref().and_then(|r| remote::behind(r, now)) {
        parts.push((Part::Behind, line));
    }
    let brief = Verb::Brief {
        project: None,
        all: false,
    };
    if let Some(found) = verb(ws, cwd, now, brief, BRIEF_BUDGET) {
        parts.push((Part::Brief, format!("Writfold KB brief (`writfold kb brief`). Rows end in their source; re-check any row with a ! flag before relying on it.\n{found}")));
    }
    if let Some(run) = current {
        let resume = Verb::Resume {
            run: Some(run.to_owned()),
        };
        if let Some(found) = verb(ws, cwd, now, resume, RESUME_BUDGET) {
            parts.push((
                Part::Run,
                format!("\nCurrent run (`writfold kb resume`):\n{found}"),
            ));
        }
        let header = "Plan position (`writfold kb plan status`)";
        if let Some(block) = plan_block(ws, cwd, now, run, header) {
            parts.push((Part::Plan, block));
        }
    }
    if let Some(local) = local {
        let block = inbox::block(&ws.root, local, current);
        if !block.is_empty() {
            parts.push((Part::Inbox, block));
        }
    }
    // Without a brief the first part must not open with a blank line.
    if let Some((_, first)) = parts.first_mut() {
        *first = first.trim_start_matches('\n').to_owned();
    }
    parts
}
