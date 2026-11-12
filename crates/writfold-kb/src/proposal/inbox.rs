//! How pending proposals are shown: rows for `proposals list`, `resume`
//! and `check`, and the session-start inbox lines.
use super::{Proposal, codec};
use crate::emit::{More, Row};
use crate::session::Session;
use std::path::Path;

/// Proposals listed at session start.
const SHOWN: usize = 3;

/// `proposal <id> [pending] <summary> @<file> !proposal`, with where it
/// came from and the command that accepts it.
pub fn row(state: &Path, p: &Proposal) -> Row {
    let mut origin = vec![format!("created={}", p.created)];
    origin.extend(p.session.iter().map(|s| format!("session={s}")));
    origin.extend(p.run.iter().map(|r| format!("run={r}")));
    Row {
        kind: "proposal",
        id: p.id.clone(),
        status: Some("pending".into()),
        text: codec::summary(&p.kind, &p.args),
        path: super::path(state, &p.id)
            .ok()
            .map(|f| f.display().to_string()),
        flags: vec!["proposal".into()],
        more: vec![
            More {
                label: "field",
                text: origin.join(" "),
                ..More::default()
            },
            More {
                label: "accept",
                text: format!("writfold kb proposals accept {}", p.id),
                ..More::default()
            },
        ],
        ..Row::default()
    }
}

/// The pending proposals of `run`, or of no run (what `resume` shows).
pub fn rows(session: &Session<'_>, run: &str) -> Vec<Row> {
    let Some(state) = super::state_of(session.ws) else {
        return Vec::new();
    };
    super::all(&state)
        .iter()
        .filter(|p| p.run.as_deref().is_none_or(|r| r == run))
        .map(|p| row(&state, p))
        .collect()
}

/// The session-start inbox lines for the proposals `resume` did not show
/// (`current` is the run whose `resume` was injected), then `(+N more
/// proposals)`; empty when there are none.
pub fn inbox(state: &Path, current: Option<&str>) -> Vec<String> {
    let shown_by_resume =
        |p: &Proposal| current.is_some() && p.run.as_deref().is_none_or(|r| Some(r) == current);
    let pending: Vec<Proposal> = super::all(state)
        .into_iter()
        .filter(|p| !shown_by_resume(p))
        .collect();
    let mut lines: Vec<String> = pending
        .iter()
        .take(SHOWN)
        .map(|p| {
            let mut line = format!("proposal {} {}", p.id, p.created);
            if let Some(run) = &p.run {
                line.push_str(&format!(" run={run}"));
            }
            format!("{line} {}\n", codec::summary(&p.kind, &p.args))
        })
        .collect();
    if pending.len() > SHOWN {
        lines.push(format!("(+{} more proposals)\n", pending.len() - SHOWN));
    }
    lines
}
