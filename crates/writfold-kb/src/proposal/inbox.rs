//! How pending proposals are shown: rows for `proposals list`, `resume`
//! and `check`, and the session-start inbox lines. Both stores are read
//! as one queue ([`super::Stores::pending`]).
use super::{Pending, Store, Stores, codec};
use crate::emit::{More, Row};
use crate::session::Session;

/// Proposals listed at session start.
const SHOWN: usize = 3;

/// `proposal <id> [pending] <summary> @<file> !proposal !local|!tracked`,
/// with where it came from and the command that accepts it.
pub fn row(pending: &Pending) -> Row {
    let p = &pending.proposal;
    let mut origin = vec![format!("created={}", p.created)];
    origin.extend(p.session.iter().map(|s| format!("session={s}")));
    origin.extend(p.run.iter().map(|r| format!("run={r}")));
    Row {
        kind: "proposal",
        id: p.id.clone(),
        status: Some("pending".into()),
        text: codec::summary(&p.kind, &p.args),
        path: pending.file().map(|f| f.display().to_string()),
        flags: vec!["proposal".into(), pending.store().label().into()],
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
    let Some(stores) = Stores::of(session.ws) else {
        return Vec::new();
    };
    stores
        .pending()
        .iter()
        .filter(|p| p.proposal.run.as_deref().is_none_or(|r| r == run))
        .map(row)
        .collect()
}

/// The session-start inbox lines for the proposals the injected text does
/// not contain (`shown` answers that for an ID), then `(+N more
/// proposals)`; empty when there are none. A tracked proposal says so.
pub fn inbox(stores: &Stores, shown: &dyn Fn(&str) -> bool) -> Vec<String> {
    let pending: Vec<Pending> = stores
        .pending()
        .into_iter()
        .filter(|p| !shown(&p.proposal.id))
        .collect();
    let mut lines: Vec<String> = pending
        .iter()
        .take(SHOWN)
        .map(|pending| {
            let p = &pending.proposal;
            let mut line = format!("proposal {} {}", p.id, p.created);
            if pending.store() == Store::Tracked {
                line.push_str(" tracked");
            }
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
