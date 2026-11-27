//! `resume [<run>] --packet` (#109, RFC 2026-11-24-resume-context): one
//! bounded handover for a fresh session, built from `resume`'s rows. The
//! rows a session cannot safely continue without are required: the run,
//! its plan position, gates and deliveries still owed, unconfirmed drafts,
//! anything that did not read, and the newest checkpoint. The rest (gates
//! already met, rechecks, artifacts, proposals, coverage) are optional.
//! Required rows come first, each group in `resume`'s order; the packet is
//! the longest prefix within `--budget-tokens` by the pager's own cost
//! ([`writfold_core::packet::cut`]). It says `packet=complete` only when
//! every required row is in it and nothing it was built from was
//! unreadable ([`writfold_core::packet::complete`]); a required row that
//! does not fit makes the answer `unknown reason=budget` (exit 2).
//!
//! Ordering, classifying and costing rows are tested here; the cut and the
//! verdict are the proved kernel.
use crate::emit::{Answer, Row, row_cost};
use crate::error::Result;
use crate::session::Session;
use std::path::Path;
use writfold_core::packet::{complete, cut, required_omitted};

/// Standings of a row that say it did not read.
const UNREAD: [&str; 3] = ["invalid", "unreadable", "unverifiable"];

fn unread(row: &Row) -> bool {
    row.status.as_deref().is_some_and(|s| UNREAD.contains(&s))
        || row.flags.iter().any(|f| UNREAD.contains(&f.as_str()))
}

/// Whether a session cannot safely continue without this row.
fn required(row: &Row) -> bool {
    let status = row.status.as_deref().unwrap_or_default();
    match row.kind {
        "run" | "plan" | "draft" | "checkpoint" | "unread" => true,
        "gate" => !matches!(status, "satisfied" | "attested" | "waived"),
        "delivery" => status != "ready",
        _ => unread(row),
    }
}

pub fn packet(
    session: &Session<'_>,
    named: Option<&str>,
    cwd: &Path,
    budget: usize,
) -> Result<(Answer, String)> {
    let (mut answer, id) = super::resume(session, named, cwd)?;
    if answer.unknown.is_some() || answer.refused {
        return Ok((answer, id));
    }
    let rows = std::mem::take(&mut answer.rows);
    let (mut ordered, optional): (Vec<Row>, Vec<Row>) = rows.into_iter().partition(required);
    let needed = ordered.len();
    ordered.extend(optional);
    let costs: Vec<usize> = ordered.iter().map(row_cost).collect();
    let kept = cut(&costs, budget);
    // Files of the run left unread (#181) are their own input to the
    // verdict; `resume` marks the answer partial for them, which is not an
    // unreadable input as well.
    let files = (answer.counts.iter())
        .find(|(name, _)| *name == "unread")
        .map_or(0, |(_, n)| *n);
    let other_partial = answer.partial && files == 0;
    let unreadable = ordered.iter().filter(|r| unread(r)).count() + usize::from(other_partial);
    let missing = required_omitted(kept, needed);
    let whole = complete(kept, needed, unreadable, files);
    let dropped = ordered.len().saturating_sub(kept.max(needed));
    ordered.truncate(kept);
    answer.rows = ordered;
    answer.counts.push(("required", needed));
    answer.counts.push(("required_omitted", missing));
    answer.counts.push(("optional_omitted", dropped));
    let word = if whole { "complete" } else { "partial" };
    answer.words.push(("packet", word.into()));
    if missing > 0 {
        answer.unknown = Some("budget");
        answer.notes.push(format!(
            "{missing} of {needed} required rows did not fit in --budget-tokens {budget}: raise it, or read them all with writfold kb resume {id} (paged with --cursor)"
        ));
    } else if unreadable > 0 || files > 0 {
        answer.partial = true;
        if unreadable > 0 {
            answer.notes.push(format!(
                "{unreadable} input(s) did not read; the packet cannot say it is complete"
            ));
        }
    }
    if dropped > 0 {
        answer.notes.push(format!(
            "{dropped} optional row(s) left out to fit the budget; writfold kb resume {id} shows them"
        ));
    }
    Ok((answer, id))
}
