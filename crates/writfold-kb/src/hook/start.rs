//! `session-start`: what a session begins with, all model-free and read
//! from local files and refs.
//!
//! Parts, in the order they are shown:
//!
//! 1. **Behind remote** ([`super::remote`]): one line when the KB's branch
//!    lags its upstream as last fetched; one Git spawn, no fetch.
//! 2. **Brief** (`brief`, budget [`BRIEF_BUDGET`] tokens), as before.
//! 3. **Stale citations** ([`stale_line`]): one line counting the
//!    non-fresh verdicts `writfold kb stale` last cached, when there are
//!    any, and one line naming each declared checker not on `PATH`
//!    ([`crate::stale::checker::absent_line`]); no repository is read and
//!    nothing is spawned.
//! 4. **Current run** (`resume`, budget [`RESUME_BUDGET`]), when the
//!    current-run pointer is set.
//! 5. **Plan position** ([`super::position`]): the current step and what
//!    it owes, from `plan status`, when the current run has a plan.
//! 6. **Inbox** ([`super::inbox`]): drafts, pending proposals and runs left
//!    without an evaluation.
//!
//! The whole stays within [`CONTEXT_LIMIT`] characters. When it would not,
//! parts are cut (each at a line, with a marker) in [`DROP_ORDER`]: the
//! brief first, then the run, the inbox, the plan position, the stale
//! citations line, and the behind-remote line last.
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
    Stale,
    Run,
    Plan,
    Inbox,
}

/// Which part is cut first when the whole is over the limit.
pub const DROP_ORDER: [Part; 6] = [
    Part::Brief,
    Part::Run,
    Part::Inbox,
    Part::Plan,
    Part::Stale,
    Part::Behind,
];

/// Fit the parts (in display order) within `limit` characters: parts are
/// kept whole from the last of [`DROP_ORDER`] backwards while they fit;
/// the first that does not is cut at a line with a marker, and the rest
/// get what remains (nothing, once too little is left for a marker).
pub fn fit(parts: &[(Part, String)], limit: usize) -> String {
    kept(parts, limit).concat()
}

/// What [`fit`] keeps of each part, in the order of `parts`.
fn kept(parts: &[(Part, String)], limit: usize) -> Vec<String> {
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
    kept
}

/// Add the inbox to `parts`. It lists what the injected `resume` text does
/// not contain, so it is decided from the run part as [`fit`] keeps it.
/// A longer inbox can only shorten that part, and so only add to the
/// inbox; this repeats until the run part kept no longer changes. It ends:
/// each further round keeps strictly less of the run part.
fn with_inbox(parts: Vec<(Part, String)>, root: &Path, local: &Path) -> Vec<(Part, String)> {
    let run_at = parts.iter().position(|(p, _)| *p == Part::Run);
    let mut resume = run_at.map_or_else(String::new, |i| parts[i].1.clone());
    loop {
        let block = inbox::block(root, local, &resume);
        let mut trial = parts.clone();
        if !block.is_empty() {
            trial.push((Part::Inbox, block));
        }
        let shown = run_at.map_or_else(String::new, |i| kept(&trial, CONTEXT_LIMIT)[i].clone());
        if shown == resume {
            return trial;
        }
        resume = shown;
    }
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
    let absent = crate::stale::checker::absent_line(&ws.root);
    let lines: Vec<String> = stale_line(ws).into_iter().chain(absent).collect();
    if !lines.is_empty() {
        parts.push((Part::Stale, lines.concat()));
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
        parts = with_inbox(parts, &ws.root, local);
    }
    // Without a brief the first part must not open with a blank line.
    if let Some((_, first)) = parts.first_mut() {
        *first = first.trim_start_matches('\n').to_owned();
    }
    parts
}

/// `Citations: 4 changed, 2 gone as of the last writfold kb stale; …`:
/// the non-fresh verdicts in the index's verdict cache, worst first, for
/// citations the index still holds. `None` when the cache is empty, all
/// fresh, or there is no index file (a KB outside Git).
pub fn stale_line(ws: &Workspace) -> Option<String> {
    let file = crate::index::index_file(ws.repo.as_ref()?).ok()?;
    if !file.is_file() {
        return None;
    }
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
    let conn = rusqlite::Connection::open_with_flags(&file, flags).ok()?;
    let mut statement = conn
        .prepare(
            "SELECT k.verdict, COUNT(*) FROM cite_checks k \
             JOIN citations c ON c.path = k.path AND c.ord = k.ord \
             WHERE k.verdict != 'fresh' GROUP BY k.verdict",
        )
        .ok()?;
    let mut counts: Vec<(String, i64)> = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .ok()?
        .collect::<rusqlite::Result<_>>()
        .ok()?;
    if counts.is_empty() {
        return None;
    }
    counts.sort_by_key(|(verdict, _)| (crate::verbs::stale::severity(verdict), verdict.clone()));
    let said: Vec<String> = counts
        .iter()
        .map(|(verdict, n)| match verdict.as_str() {
            "marked" => format!("{n} marked stale"),
            _ => format!("{n} {verdict}"),
        })
        .collect();
    Some(format!(
        "\nCitations: {} as of the last `writfold kb stale`; run it to re-check\n",
        said.join(", ")
    ))
}
