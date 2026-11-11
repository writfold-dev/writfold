//! One typed query per verb, each returning an [`Answer`].
pub mod brief;
pub mod context;
pub mod index;
pub mod stale;
pub mod why;

use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec};
use crate::layout::Kind;
use crate::session::Session;
use crate::time;

/// Shared by every unknown answer.
pub const BOUNDARY: &str = "absence in the KB is not evidence of absence in the world; check the sources, then record what you find";

/// The authored vocabulary of a kind's status field. Values outside it are
/// kept as opaque text and flagged, never rejected.
fn vocabulary(kind: &str) -> &'static [&'static str] {
    match kind {
        "asset" => &["active", "deprecated", "retired"],
        "relationship" => &["confirmed", "disputed", "explicit", "inferred", "retired"],
        "decision" => &[
            "accepted",
            "applied",
            "deferred",
            "draft",
            "landing",
            "proposed",
            "regressed",
            "rejected",
            "superseded",
            "verified",
        ],
        "ticket" => &[
            "blocked",
            "done",
            "in-progress",
            "investigating",
            "n/a",
            "todo",
        ],
        "investigation" => &["abandoned", "concluded", "monitoring", "open"],
        "signature" => &[
            "confirmed",
            "fix-in-progress",
            "fixed",
            "known-error",
            "retired",
            "suspected",
        ],
        _ => &[],
    }
}

/// The static row kind for a stored kind name.
pub fn kind_label(kind: &str) -> &'static str {
    Kind::parse(kind).map_or("record", Kind::as_str)
}

/// The standard row for a record, with its vocabulary and TTL flags.
pub fn record_row(session: &Session<'_>, rec: &Rec) -> Row {
    let mut flags = Vec::new();
    if let Some(status) = &rec.status {
        let known = vocabulary(&rec.kind);
        if !known.is_empty() && !known.contains(&status.as_str()) {
            flags.push("vocab=unknown".to_owned());
        }
    }
    if rec.kind == "asset"
        && let Some(verified) = rec.field("verified_at")
        && time::expired(&verified, rec.field("ttl").as_deref(), session.now) == Some(true)
    {
        flags.push("past-ttl".to_owned());
    }
    if let (Some(successor), Some("superseded")) =
        (rec.field("superseded_by"), rec.status.as_deref())
    {
        flags.push(format!("superseded→{successor}"));
    }
    Row {
        kind: kind_label(&rec.kind),
        id: rec.id.clone(),
        status: rec.status.clone(),
        text: rec.label(),
        path: Some(rec.path.clone()),
        line: Some(rec.line),
        flags,
        ..Row::default()
    }
}

/// A continuation line for an edge: `via: <rel> <other>` at its provenance.
pub fn via(rel: &str, other: &str, path: &str, line: u32) -> More {
    More {
        label: "via",
        text: format!("{rel} {other}"),
        path: Some(path.to_owned()),
        line: Some(line),
        flags: Vec::new(),
    }
}

/// An ambiguous target: the candidates, and a refusal (exit 3).
pub fn ambiguous(session: &Session<'_>, target: &str, candidates: &[Rec]) -> Answer {
    Answer {
        rows: candidates
            .iter()
            .map(|rec| record_row(session, rec))
            .collect(),
        notes: vec![format!(
            "{target:?} names {} records; pass one ID",
            candidates.len()
        )],
        refused: true,
        ..Answer::default()
    }
}

/// An unknown answer: up to three nearest full-text rows and the boundary
/// note.
pub fn unknown(
    session: &Session<'_>,
    reason: &'static str,
    text: &str,
    kinds: &[&str],
) -> Result<Answer> {
    let mut rows = Vec::new();
    for (id, title, path) in query::nearest(&session.conn, text, 3, kinds)? {
        rows.push(Row {
            kind: "nearest",
            text: title.unwrap_or_default(),
            id,
            path: Some(path),
            ..Row::default()
        });
    }
    Ok(Answer {
        rows,
        notes: vec![BOUNDARY.to_owned()],
        unknown: Some(reason),
        ..Answer::default()
    })
}

/// Cached citation verdicts for the files given, worst first: rows for the
/// non-fresh ones, and the number of citations never checked.
pub fn cached_verdicts(
    session: &Session<'_>,
    paths: &[String],
    limit: usize,
) -> Result<(Vec<Row>, usize)> {
    let mut rows = Vec::new();
    let mut unchecked = 0usize;
    for path in paths {
        let mut statement = session.conn.prepare_cached(
            "SELECT c.ord, c.line, c.raw, k.verdict, k.repo_head, k.new_first, k.new_last, k.new_path \
             FROM citations c LEFT JOIN cite_checks k ON k.path = c.path AND k.ord = c.ord \
             WHERE c.path = ?1 AND c.illustrative = 0 ORDER BY c.ord",
        )?;
        let found = statement.query_map([path], |r| {
            Ok((
                r.get::<_, u32>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                (
                    r.get::<_, Option<u32>>(5)?,
                    r.get::<_, Option<u32>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ),
            ))
        })?;
        for item in found {
            let (line, raw, verdict, head, moved) = item?;
            match verdict.as_deref() {
                None => unchecked += 1,
                Some("fresh") => {}
                Some(verdict) => rows.push(stale::verdict_row(
                    path,
                    line,
                    &raw,
                    verdict,
                    head.as_deref(),
                    moved,
                )),
            }
        }
    }
    rows.sort_by_key(|row| stale::severity(row.status.as_deref().unwrap_or_default()));
    rows.truncate(limit);
    Ok((rows, unchecked))
}
