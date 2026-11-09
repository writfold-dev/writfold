//! Rows, projection order and source-mapped excerpts.
use crate::ir::{Direction, SortField, ViewPlan};
use std::cmp::Ordering;
use writfold_domain::citation::Coverage;

/// Why a search stopped before covering its whole scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stop {
    /// More matches existed than the `find` limit admits.
    HitLimit,
}

/// What `find` searched, measured by the adapter that ran it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchReport {
    /// Files fully searched over files in scope at the pinned commit.
    pub coverage: Coverage,
    pub stop: Option<Stop>,
    pub hits: usize,
}

/// Retained evidence for a row: its `writfold-citation/1` binding digest and
/// exactly the retained bytes, which equal the file bytes at the row's range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub binding: String,
    pub bytes: Vec<u8>,
}

/// One row before projection: a hit, or a retained range around a hit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Row {
    pub path: String,
    /// Half-open byte range into the file at the pinned commit.
    pub range: (u64, u64),
    /// 1-based line of `range.0`.
    pub line: u64,
    pub evidence: Option<Evidence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projected {
    pub rows: Vec<Row>,
    /// Identical rows (same path, range and binding, e.g. two hits on one
    /// line retained `around lines 0`) merged into one.
    pub merged: usize,
    /// Rows dropped by the projection's own `limit` (not resumable).
    pub limited: usize,
}

/// Sort by the declared keys, then by `(path, range)` ascending, so equal keys
/// never leave the order to the backend.
pub fn project(mut rows: Vec<Row>, plan: &ViewPlan) -> Projected {
    rows.sort_by(|a, b| {
        for &(field, direction) in &plan.sort {
            let order = compare(a, b, field);
            let order = if direction == Direction::Desc {
                order.reverse()
            } else {
                order
            };
            if order != Ordering::Equal {
                return order;
            }
        }
        compare(a, b, SortField::Path).then_with(|| compare(a, b, SortField::Range))
    });
    let before = rows.len();
    rows.dedup();
    let merged = before - rows.len();
    let limited = plan
        .limit
        .map_or(0, |limit| rows.len().saturating_sub(limit));
    rows.truncate(rows.len() - limited);
    Projected {
        rows,
        merged,
        limited,
    }
}

fn compare(a: &Row, b: &Row, field: SortField) -> Ordering {
    match field {
        SortField::Path => a.path.as_bytes().cmp(b.path.as_bytes()),
        SortField::Line => a.line.cmp(&b.line),
        SortField::Range => a.range.cmp(&b.range),
    }
}

/// A prefix of the retained bytes: at most `max_chars` Unicode scalars of
/// their longest valid UTF-8 prefix. `range` is the exact file byte range it
/// shows; `hidden` counts the retained bytes after it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Excerpt {
    pub range: (u64, u64),
    pub text: String,
    pub hidden: u64,
}

pub fn excerpt(row: &Row, max_chars: usize) -> Option<Excerpt> {
    let bytes = &row.evidence.as_ref()?.bytes;
    let valid = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(bytes.get(..error.valid_up_to())?).ok()?,
    };
    let shown = valid
        .char_indices()
        .nth(max_chars)
        .map_or(valid.len(), |(at, _)| at);
    let text = valid.get(..shown)?.to_owned();
    Some(Excerpt {
        range: (row.range.0, row.range.0 + shown as u64),
        text,
        hidden: (bytes.len() - shown) as u64,
    })
}
