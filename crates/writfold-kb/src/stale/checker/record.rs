//! `kb stale --record`: what the checkers said in this run, written to one
//! new check receipt per checked kind (`.writfold/checks/<kind>/`), then
//! the receipts it supersedes removed from the work tree. Only a checker's
//! own statement about a token is recorded (the kernel's `recordable`),
//! taken from this run's answers before any authored mark; a kind without
//! `max_age` is not recorded, since a shared verdict needs a lifetime.
use super::super::kinds::{self, Kinds};
use super::super::{Cite, Verdict};
use super::receipt::{self, DIR, Entry, Moved, Receipt};
use super::{Answered, Answers, proto};
use crate::Request;
use crate::emit::{Frame, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, Probe, atomic};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use writfold_core::verdict::{self as kernel, Checker, Said};

/// The receipts to write, and the notes on what was left out.
#[derive(Debug, Default)]
pub struct Plan {
    pub receipts: Vec<Receipt>,
    pub notes: Vec<String>,
}

/// One token's entry, when the checker's answer may be recorded.
fn entry(cite: &Cite, answered: &Answered) -> Option<Entry> {
    let Checker::Said(said) = answered.checker else {
        return None;
    };
    if !kernel::recordable(answered.checker) {
        return None;
    }
    let moved = match (&answered.checked.verdict, said) {
        (Verdict::Moved { path, first, last }, _) => Some(Moved {
            path: path.clone(),
            first: *first,
            last: *last,
        }),
        // A `moved` reply without a valid range is not a statement.
        (_, Said::Moved) => return None,
        _ => None,
    };
    let detail = match &answered.checked.verdict {
        Verdict::Changed { detail } | Verdict::Gone { detail } => detail.clone(),
        _ => None,
    };
    Some(Entry {
        token: receipt::digest(&cite.raw),
        said,
        at: answered.at.clone(),
        detail,
        moved,
    })
}

/// Group this run's recordable answers by kind into receipts dated `now`.
pub fn plan(kinds: &Kinds, cites: &[Cite], answers: &Answers, now: i64) -> Plan {
    let mut by_kind: BTreeMap<&str, (BTreeMap<String, Entry>, BTreeSet<String>)> = BTreeMap::new();
    let mut versions: BTreeMap<&str, String> = BTreeMap::new();
    for cite in cites {
        let (Some(kind), Some(answered)) = (
            cite.kind.as_deref(),
            answers.get(&(cite.path.clone(), cite.ord)),
        ) else {
            continue;
        };
        if let Some(v) = &answered.version {
            versions.entry(kind).or_insert_with(|| v.clone());
        }
        let (entries, split) = by_kind.entry(kind).or_default();
        let Some(new) = entry(cite, answered) else {
            continue;
        };
        match entries.get(&new.token) {
            Some(old) if old.said != new.said => {
                split.insert(new.token.clone());
            }
            Some(_) => {}
            None => {
                entries.insert(new.token.clone(), new);
            }
        }
    }
    let mut out = Plan::default();
    for (kind, (mut entries, split)) in by_kind {
        if kinds.max_age(kind).is_none() {
            out.notes.push(format!(
                "not recorded: kind {kind} declares no max_age (a recorded verdict needs a lifetime)"
            ));
            continue;
        }
        if !split.is_empty() {
            entries.retain(|token, _| !split.contains(token));
            out.notes.push(format!(
                "not recorded: {} tokens of kind {kind} received different statuses in this run",
                split.len()
            ));
        }
        if entries.is_empty() {
            continue;
        }
        let name = crate::write::unique(now, kind);
        out.receipts.push(Receipt {
            path: format!("{DIR}/{kind}/{name}.json"),
            kind: kind.to_owned(),
            checker: kinds.checker(kind).unwrap_or_default().to_owned(),
            version: versions.get(kind).cloned(),
            protocol: Some(proto::NAME.to_owned()),
            checked_at: crate::time::stamp(now),
            at: now,
            verdicts: entries.into_values().collect(),
        });
    }
    out
}

/// Remove each other readable receipt of the kind (same checker) whose
/// every entry is held by `new` or is older than `max_age` now. A file
/// skipped as unreadable is never removed. Returns how many were removed.
pub fn prune(root: &Path, new: &Receipt, max_age: i64, now: i64) -> Result<usize> {
    let held: BTreeSet<&str> = new.verdicts.iter().map(|e| e.token.as_str()).collect();
    let (found, _) = receipt::read_kind(root, &new.kind, &new.checker);
    let mut removed = 0;
    for old in found.iter().filter(|r| r.path != new.path) {
        let expired = now.saturating_sub(old.at) > max_age;
        if expired || old.verdicts.iter().all(|e| held.contains(e.token.as_str())) {
            match std::fs::remove_file(root.join(&old.path)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
                _ => removed += 1,
            }
        }
    }
    Ok(removed)
}

/// Publish one receipt through the atomic write path, then prune.
fn publish(
    root: &Path,
    state: &Path,
    kinds: &Kinds,
    new: &Receipt,
    now: i64,
    probe: Probe,
) -> Result<String> {
    let op = Op::Create {
        path: new.path.clone(),
        bytes: receipt::encode(new).into_bytes(),
    };
    atomic::apply(root, state, &[op], probe)?;
    let max_age = kinds.max_age(&new.kind).unwrap_or(i64::MAX);
    let removed = prune(root, new, max_age, now)?;
    let tail = match removed {
        0 => String::new(),
        1 => " (1 superseded receipt removed)".into(),
        n => format!(" ({n} superseded receipts removed)"),
    };
    Ok(format!(
        "recorded {} verdicts of kind {} in {}{tail}",
        new.verdicts.len(),
        new.kind,
        new.path
    ))
}

/// `kb stale [<scope>] --record`: `stale`'s answer, then the receipts.
pub fn execute(ws: &Workspace, request: &Request, scope: Option<&str>, now: i64) -> Result<Output> {
    let Some(repo) = &ws.repo else {
        return Err(KbError::Refused(
            "--record writes check receipts, which needs a KB under Git".into(),
        ));
    };
    let tool = GitTool::locate()?;
    let state = repo.git_dir.join("writfold");
    let session = Session::open(ws, Some(&tool), false, now)?;
    let (mut answer, cites, judged) = crate::verbs::stale::report(&session, scope, false, false)?;
    let declared = kinds::read(&ws.root)?;
    let plan = plan(&declared, &cites, &judged.answers, now);
    answer.notes.extend(plan.notes);
    let (mut verdicts, mut files) = (0, 0);
    for new in &plan.receipts {
        match publish(&ws.root, &state, &declared, new, now, request.probe) {
            Ok(note) => {
                verdicts += new.verdicts.len();
                files += 1;
                answer.notes.push(note);
            }
            Err(error) => {
                answer.refused = true;
                answer.notes.push(error.to_string());
                break;
            }
        }
    }
    if files > 0 {
        answer
            .notes
            .push("written to the work tree; commit it to share it".into());
    }
    answer.counts.push(("recorded-now", verdicts));
    answer.counts.push(("receipts", files));
    let frame = Frame {
        verb: "stale".into(),
        args: vec![
            scope.unwrap_or_default().to_owned(),
            "offline=false".into(),
            "record=true".into(),
        ],
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    session.emit(&answer, &frame)
}
