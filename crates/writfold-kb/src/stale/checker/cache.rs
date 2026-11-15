//! What a reader that does not ask the checker sees for a checked token:
//! the newest of this machine's cached row (`cite_checks`, written only by
//! `kb stale`) and the check receipts' entries for the token's digest,
//! picked by [`writfold_core::verdict::newest`] and aged against the kind's
//! `max_age`. A verdict read from a receipt is labelled `recorded` and is
//! never written to the cache.
use super::super::kinds::Kinds;
use super::super::{Checked, Cite, Recorded, Verdict};
use super::receipt::{Entry, Receipt, Receipts};
use crate::error::Result;
use crate::session::Session;
use crate::time;
use writfold_core::verdict::{self as kernel, Checker, Pick, Said, Statement};

pub const EXPIRED: &str = "check-expired";

/// One cached row: the verdict, the moved range and path, the head, the
/// detail and when it was checked.
struct Row {
    said: String,
    first: Option<u32>,
    last: Option<u32>,
    path: Option<String>,
    head: String,
    detail: Option<String>,
    checked_at: String,
}

/// The cached reply as the kernel names it.
fn said(verdict: &str) -> Checker {
    match verdict {
        "fresh" => Checker::Said(Said::Fresh),
        "moved" => Checker::Said(Said::Moved),
        "changed" => Checker::Said(Said::Changed),
        "gone" => Checker::Said(Said::Gone),
        "unverifiable" => Checker::Said(Said::Unverifiable),
        _ => Checker::Failed,
    }
}

/// `<N>d` or `<N>h` (whole units, rounded down) for an age in seconds.
fn age_text(age: Option<i64>) -> String {
    match age {
        Some(a) if a >= 86_400 => format!("{}d", a / 86_400),
        Some(a) => format!("{}h", a.max(0) / 3600),
        None => "unreadable checked_at".into(),
    }
}

/// The row `kb stale` last cached for the same token text from the same
/// checker (this citation's row first).
fn row(session: &Session<'_>, name: &str, cite: &Cite) -> Result<Option<Row>> {
    let head = format!("checker:{name}");
    Ok(rusqlite::OptionalExtension::optional(session.conn.query_row(
        "SELECT k.verdict, k.new_first, k.new_last, k.new_path, k.repo_head, k.detail, k.checked_at \
         FROM cite_checks k JOIN citations c ON c.path = k.path AND c.ord = k.ord \
         WHERE c.raw = ?1 AND (k.repo_head = ?2 OR k.repo_head GLOB ?2 || '@*') \
         ORDER BY (k.path = ?3 AND k.ord = ?4) DESC, k.path, k.ord LIMIT 1",
        (&cite.raw, &head, &cite.path, cite.ord),
        |r| {
            Ok(Row {
                said: r.get(0)?,
                first: r.get(1)?,
                last: r.get(2)?,
                path: r.get(3)?,
                head: r.get(4)?,
                detail: r.get(5)?,
                checked_at: r.get(6)?,
            })
        },
    ))?)
}

/// The cached row as a verdict, aged against `max_age`.
fn from_row(row: Row, age: Option<i64>, max_age: Option<i64>) -> Checked {
    let verdict = match (row.said.as_str(), row.first, row.last) {
        _ if kernel::aged(said(&row.said), age, max_age) == Checker::Expired => {
            Verdict::Unverifiable(format!("{EXPIRED} ({})", age_text(age)))
        }
        ("fresh", _, _) => Verdict::Fresh,
        ("moved", Some(first), Some(last)) => Verdict::Moved {
            path: row.path,
            first,
            last,
        },
        ("changed", _, _) => Verdict::Changed { detail: row.detail },
        ("gone", _, _) => Verdict::Gone { detail: row.detail },
        ("unverifiable", _, _) => Verdict::Unverifiable(row.detail.unwrap_or(row.said)),
        _ => Verdict::Unverifiable("not-checked".into()),
    };
    Checked {
        verdict,
        head: row.head,
        recorded: None,
    }
}

/// A recorded entry as a verdict, aged against `max_age`, labelled with
/// its receipt.
fn from_entry(
    cite: &Cite,
    receipt: &Receipt,
    entry: &Entry,
    age: i64,
    max_age: Option<i64>,
) -> Checked {
    let aged = kernel::aged(Checker::Said(entry.said), Some(age), max_age);
    let detail = entry.detail.clone();
    let verdict = match (aged, &entry.moved) {
        (Checker::Expired, _) => {
            Verdict::Unverifiable(format!("{EXPIRED} ({})", age_text(Some(age))))
        }
        (Checker::Said(Said::Fresh), _) => Verdict::Fresh,
        (Checker::Said(Said::Moved), Some(m)) => Verdict::Moved {
            path: m
                .path
                .clone()
                .filter(|p| cite.field("path").as_ref() != Some(p)),
            first: m.first,
            last: m.last,
        },
        (Checker::Said(Said::Changed), _) => Verdict::Changed { detail },
        (Checker::Said(Said::Gone), _) => Verdict::Gone { detail },
        _ => Verdict::Unverifiable("not-checked".into()),
    };
    let mut head = format!("checker:{}", receipt.checker);
    if let Some(version) = &receipt.version {
        head = format!("{head}@{version}");
        if let Some(at) = &entry.at {
            head = format!("{head}:{at}");
        }
    }
    Checked {
        verdict,
        head,
        recorded: Some(Recorded {
            checked_at: receipt.checked_at.clone(),
            path: receipt.path.clone(),
            checker: receipt.checker.clone(),
            version: receipt.version.clone(),
        }),
    }
}

/// The recorded statements for the token (dated no later than an hour
/// ahead), with their receipts.
fn statements<'a>(
    session: &Session<'_>,
    kind: &str,
    cite: &Cite,
    receipts: &'a Receipts,
) -> Vec<(Statement, &'a Receipt, &'a Entry)> {
    receipts
        .lookup(kind, &cite.raw)
        .into_iter()
        .filter_map(|(receipt, entry)| {
            let age = kernel::recorded_age(session.now, receipt.at)?;
            let said = entry.said;
            Some((
                Statement {
                    said,
                    age,
                    recorded: true,
                },
                receipt,
                entry,
            ))
        })
        .collect()
}

/// The newest statement about the token: the cached row or a receipt's
/// entry, aged. `None` when neither exists.
pub fn pick(
    session: &Session<'_>,
    kinds: &Kinds,
    kind: &str,
    cite: &Cite,
    receipts: &Receipts,
) -> Result<Option<Checked>> {
    let name = kinds.checker(kind).unwrap_or_default();
    let max_age = kinds.max_age(kind);
    let row = row(session, name, cite)?;
    let age = row
        .as_ref()
        .map(|r| time::parse(&r.checked_at).map(|at| session.now.saturating_sub(at)));
    let local = match (&row, said(row.as_ref().map_or("", |r| r.said.as_str()))) {
        (Some(_), Checker::Said(said)) => Some(Statement {
            said,
            age: age.flatten().unwrap_or(i64::MAX),
            recorded: false,
        }),
        _ => None,
    };
    let recorded = statements(session, kind, cite, receipts);
    let list: Vec<Statement> = recorded.iter().map(|(s, _, _)| *s).collect();
    Ok(match kernel::newest(local, &list, max_age) {
        Some(Pick::Recorded(i)) => recorded
            .get(i)
            .map(|(s, receipt, entry)| from_entry(cite, receipt, entry, s.age, max_age)),
        _ => row.map(|r| from_row(r, age.flatten(), max_age)),
    })
}

/// The newest usable recorded verdict alone (within `max_age`), for a
/// token whose checker is absent from this machine.
pub fn recorded(
    session: &Session<'_>,
    kinds: &Kinds,
    kind: &str,
    cite: &Cite,
    receipts: &Receipts,
) -> Option<Checked> {
    let max_age = kinds.max_age(kind);
    let recorded = statements(session, kind, cite, receipts);
    let list: Vec<Statement> = recorded.iter().map(|(s, _, _)| *s).collect();
    let Some(Pick::Recorded(i)) = kernel::newest(None, &list, max_age) else {
        return None;
    };
    let (s, receipt, entry) = recorded.get(i)?;
    let checked = from_entry(cite, receipt, entry, s.age, max_age);
    let expired =
        matches!(&checked.verdict, Verdict::Unverifiable(why) if why.starts_with(EXPIRED));
    (!expired).then_some(checked)
}
