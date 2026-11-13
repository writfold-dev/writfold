//! The session-start inbox: what waits on someone, all of it local and
//! none of it in the KB yet.
//!
//! - **Drafts** whose row the injected `resume` text does not contain.
//! - **Pending proposals**, local or tracked, whose row the injected
//!   `resume` text does not contain ([`crate::proposal::inbox`]). This is decided from the text
//!   as injected, after `resume`'s token budget and the session-start cut,
//!   not from what `resume` would list at an unlimited budget.
//! - **Runs left without an evaluation.** A session is known to have
//!   ended only through its draft (`SessionEnd` marks it `ended`; its
//!   `updated` stamp is then the end). Such a session's run is listed when
//!   all three hold: the run is open; its newest checkpoint entry is older
//!   than the session's end (or it has none); and its plan has a step
//!   named `reflect` whose last occurrence is neither done nor skipped.
//!   Each part is read from files (the run's manifest and sidecar,
//!   `checkpoint.md`, the plan's event files), so the rule is
//!   deterministic. A run whose draft was confirmed or deleted, or whose
//!   plan has no `reflect` step, is not listed.
use super::draft_line;
use crate::draft;
use crate::plan::fold;
use crate::time;
use serde_json::{Map, Value};
use std::path::Path;

/// Lines of each kind listed before `(+N more …)`.
const SHOWN: usize = 3;

type Draft = (String, Map<String, Value>);

/// Whether `resume` (as injected) holds the row `<kind> <id> [...`.
fn listed(resume: &str, kind: &str, id: &str) -> bool {
    let head = format!("{kind} {id} [");
    resume.lines().any(|line| line.starts_with(&head))
}

/// The inbox block, or an empty string when nothing waits. `resume` is
/// the current run's part exactly as injected (empty when there is none).
pub fn block(root: &Path, state: &Path, resume: &str) -> String {
    let all = draft::all(state);
    let drafts: Vec<&Draft> = all
        .iter()
        .filter(|(session, _)| !listed(resume, "draft", session))
        .collect();
    let mut text = String::new();
    if !drafts.is_empty() {
        for (session, d) in drafts.iter().take(SHOWN) {
            text.push_str(&draft_line(session, d));
        }
        more(&mut text, drafts.len(), "drafts");
        text.push_str("  confirm: writfold kb checkpoint --from-draft SESSION\n");
    }
    let stores = crate::proposal::Stores {
        state: state.to_path_buf(),
        tracked: root.join(crate::proposal::TRACKED_DIR),
    };
    let proposals = crate::proposal::inbox(&stores, &|id| listed(resume, "proposal", id));
    if !proposals.is_empty() {
        for line in &proposals {
            text.push_str(line);
        }
        text.push_str("  review: writfold kb proposals show ID (accept only when the user asks)\n");
    }
    let runs = unevaluated(root, &all);
    if !runs.is_empty() {
        for line in runs.iter().take(SHOWN) {
            text.push_str(line);
        }
        more(&mut text, runs.len(), "runs");
        text.push_str(
            "  next: writfold kb plan status --run RUN, then its reflect step (the writfold:evaluate skill)\n",
        );
    }
    if text.is_empty() {
        return text;
    }
    format!("\nInbox (local only; nothing here is in the KB until confirmed or accepted):\n{text}")
}

fn more(text: &mut String, count: usize, what: &str) {
    if count > SHOWN {
        text.push_str(&format!("(+{} more {what})\n", count - SHOWN));
    }
}

/// `unevaluated <run> session <id> ended <stamp> (<reason>): <step> is not done`,
/// one per run (its newest ended session), newest first.
pub fn unevaluated(root: &Path, drafts: &[Draft]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for (session, d) in drafts {
        let (Some(run), Some(reason), Some(ended)) = (
            draft::text(d, "run"),
            draft::text(d, "ended"),
            draft::text(d, "updated"),
        ) else {
            continue;
        };
        if seen.contains(&run) {
            continue;
        }
        seen.push(run.clone());
        if let Some(step) = owes_reflection(root, &run, &ended) {
            out.push(format!(
                "unevaluated {run} session {session} ended {ended} ({reason}): {step} is not done\n"
            ));
        }
    }
    out
}

/// The path of the run's open `reflect` step, when the run is open and
/// has no checkpoint entry since `ended`.
fn owes_reflection(root: &Path, run: &str, ended: &str) -> Option<String> {
    if !draft::valid_session(run) {
        return None;
    }
    let dir = format!("runs/{run}");
    let read = |name: &str| -> Option<Map<String, Value>> {
        let text = std::fs::read_to_string(root.join(&dir).join(name)).ok()?;
        match serde_json::from_str(&text) {
            Ok(Value::Object(map)) => Some(map),
            _ => None,
        }
    };
    let manifest = read("manifest.json")?;
    let state = read(crate::runs::SIDECAR)
        .and_then(|m| draft::text(&m, "state"))
        .or_else(|| draft::text(&manifest, "state"))
        .unwrap_or_else(|| "open".into());
    if state != "open" {
        return None;
    }
    let end = time::parse(ended)?;
    let checkpoints = std::fs::read_to_string(root.join(&dir).join("checkpoint.md"));
    let last = checkpoints
        .ok()
        .and_then(|t| crate::parse::sections(&t).last().map(|s| s.name.clone()))
        .and_then(|name| time::parse(&name));
    if last.is_some_and(|at| at >= end) {
        return None;
    }
    let graph = fold::read(root, &dir, run);
    let step = graph.steps.iter().rev().find(|s| s.id == "reflect")?;
    (!step.closed()).then(|| step.path.clone())
}
