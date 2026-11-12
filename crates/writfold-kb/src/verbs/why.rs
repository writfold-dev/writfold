//! `why <id|text>`: why did we decide X.
use super::{BOUNDARY, record_row, stale::verdict_row, unknown};
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec, Resolved};
use crate::session::Session;
use std::collections::BTreeSet;

/// Rationale section names tried in order when none are configured.
pub const RATIONALE: [&str; 6] = [
    "Decision",
    "Rationale",
    "Why",
    "Why general / justification",
    "Context",
    "Proposed change",
];

fn rank(status: Option<&str>) -> u8 {
    match status {
        Some("accepted" | "applied" | "verified") => 0,
        Some("landing") => 1,
        Some("proposed") => 2,
        Some("draft" | "deferred") => 3,
        _ => 4,
    }
}

fn live(status: Option<&str>) -> bool {
    rank(status) <= 1
}

/// Candidate decisions: an exact target, then title, ID or `target` text,
/// then full-text matches.
fn candidates(session: &Session<'_>, text: &str) -> Result<Vec<Rec>> {
    if let Resolved::One(rec) = query::resolve(&session.conn, text)?
        && rec.kind == "decision"
    {
        return Ok(vec![*rec]);
    }
    let lowered = text.to_lowercase();
    let mut found = query::records(
        &session.conn,
        "kind = 'decision' AND (instr(lower(id), ?1) > 0 OR instr(lower(coalesce(title, '')), ?1) > 0 \
         OR instr(lower(coalesce(json_extract(extra, '$.target'), '')), ?1) > 0) ORDER BY id",
        &[&lowered],
    )?;
    for (id, _, _) in query::nearest(&session.conn, text, 10, &["decision"])? {
        found.extend(query::record(&session.conn, &id)?);
    }
    let mut seen = BTreeSet::new();
    found.retain(|rec| seen.insert(rec.id.clone()));
    Ok(found)
}

/// Follow `superseded_by` to the live record; returns it and the chain.
pub(crate) fn follow(session: &Session<'_>, rec: Rec) -> Result<(Rec, Vec<String>)> {
    let mut chain = Vec::new();
    let mut current = rec;
    while matches!(current.status.as_deref(), Some("superseded" | "rejected")) && chain.len() < 16 {
        let Some(next) = current.field("superseded_by") else {
            break;
        };
        let id = if next.starts_with("proposal/") {
            next
        } else {
            format!("proposal/{next}")
        };
        let Some(successor) = query::record(&session.conn, &id)? else {
            break;
        };
        chain.push(current.id.clone());
        current = successor;
    }
    Ok((current, chain))
}

pub fn why(session: &Session<'_>, text: &str, sections: &[String]) -> Result<Answer> {
    let mut answer = Answer::default();
    let mut answered = BTreeSet::new();
    let mut others = Vec::new();
    let mut found: Vec<(Rec, Vec<String>)> = Vec::new();
    for rec in candidates(session, text)? {
        let (rec, chain) = follow(session, rec)?;
        if live(rec.status.as_deref()) && answered.insert(rec.id.clone()) {
            found.push((rec, chain));
        } else if !live(rec.status.as_deref()) && !others.iter().any(|o: &Rec| o.id == rec.id) {
            others.push(rec);
        }
    }
    found.sort_by(|(a, _), (b, _)| {
        (
            rank(a.status.as_deref()),
            std::cmp::Reverse(a.updated.clone()),
            &a.id,
        )
            .cmp(&(
                rank(b.status.as_deref()),
                std::cmp::Reverse(b.updated.clone()),
                &b.id,
            ))
    });
    for (rec, chain) in &found {
        answer.rows.push(decision(session, rec, chain, sections)?);
    }
    let flagged = answer
        .rows
        .iter()
        .flat_map(|row| &row.more)
        .filter(|more| more.label == "cite" && more.flags.iter().any(|f| f != "unchecked"))
        .count();
    if flagged > 0 {
        answer.counts.push(("stale", flagged));
    }
    if found.is_empty() {
        if others.is_empty() {
            answer = unknown(session, "no-accepted-decision", text, &[])?;
        } else {
            answer.unknown = Some("no-accepted-decision");
            answer.notes.push(BOUNDARY.to_owned());
        }
        for rec in &others {
            let mut row = record_row(session, rec);
            let label = match rank(rec.status.as_deref()) {
                2 | 3 => "draft".to_owned(),
                _ => rec.status.clone().unwrap_or_else(|| "no-status".into()),
            };
            if !row.flags.iter().any(|f| f.starts_with("superseded")) {
                row.flags.push(label);
            }
            answer.rows.push(row);
        }
    }
    let topic = found
        .first()
        .and_then(|(rec, _)| rec.title.clone())
        .unwrap_or_else(|| text.to_owned());
    secondary(session, &topic, &mut answer)?;
    Ok(answer)
}

fn decision(
    session: &Session<'_>,
    rec: &Rec,
    chain: &[String],
    sections: &[String],
) -> Result<Row> {
    let mut row = record_row(session, rec);
    let decided = rec
        .field("decided_by")
        .unwrap_or_else(|| "unrecorded".into());
    let when = rec
        .updated
        .clone()
        .or(rec.created.clone())
        .unwrap_or_else(|| "undated".into());
    let line = crate::parse::field_line(&session.text(&rec.path).unwrap_or_default(), "decided_by");
    row.more.push(More {
        label: "decided",
        text: format!("by {decided} on {when}"),
        path: Some(rec.path.clone()),
        line: Some(line),
        flags: Vec::new(),
    });
    for old in chain {
        row.more.push(More {
            label: "via",
            text: format!("supersedes {old}"),
            ..More::default()
        });
    }
    row.more.extend(rationale(session, rec, sections)?);
    let mut statement = session.conn.prepare_cached(
        "SELECT c.line, c.raw, k.verdict, k.repo_head, k.new_first, k.new_last, k.new_path FROM citations c \
         LEFT JOIN cite_checks k ON k.path = c.path AND k.ord = c.ord \
         WHERE c.path = ?1 AND c.illustrative = 0 ORDER BY c.ord LIMIT 5",
    )?;
    let cites = statement
        .query_map([&rec.path], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                (r.get(4)?, r.get(5)?, r.get(6)?),
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (line, raw, verdict, head, moved) in cites {
        let verdict = verdict.unwrap_or_else(|| "unchecked".into());
        let shown = verdict_row(&rec.path, line, &raw, &verdict, head.as_deref(), moved);
        let flags = if verdict == "fresh" {
            Vec::new()
        } else {
            shown.flags
        };
        row.more.push(More {
            label: "cite",
            text: format!("{} [{verdict}]", shown.id),
            path: Some(rec.path.clone()),
            line: Some(line),
            flags,
        });
    }
    Ok(row)
}

/// `why:` lines: the first configured rationale section present, its first
/// paragraph (up to three lines); else the summary.
pub(crate) fn rationale(session: &Session<'_>, rec: &Rec, names: &[String]) -> Result<Vec<More>> {
    let mut statement = session.conn.prepare_cached(
        "SELECT name, line_first, line_last FROM sections WHERE path = ?1 ORDER BY ord",
    )?;
    let found = statement
        .query_map([&rec.path], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, u32>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let defaults: Vec<String> = RATIONALE.iter().map(|s| (*s).to_owned()).collect();
    let names = if names.is_empty() { &defaults } else { names };
    let section = names
        .iter()
        .find_map(|name| found.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(name)));
    let mut out = Vec::new();
    if let Some((_, first, last)) = section {
        let mut started = false;
        for (offset, text) in session
            .lines(&rec.path, first + 1, *last)
            .into_iter()
            .enumerate()
        {
            let text = text.trim().to_owned();
            if text.is_empty() {
                if started { break } else { continue }
            }
            started = true;
            let line = first + 1 + u32::try_from(offset).unwrap_or(0);
            out.push(More {
                label: "why",
                text,
                path: Some(rec.path.clone()),
                line: Some(line),
                flags: Vec::new(),
            });
            if out.len() == 3 {
                break;
            }
        }
    }
    if out.is_empty()
        && let Some(summary) = &rec.summary
    {
        out.push(More {
            label: "why",
            text: summary.clone(),
            path: Some(rec.path.clone()),
            line: None,
            flags: Vec::new(),
        });
    }
    Ok(out)
}

/// Concluded investigations and finished ticket records on the same text.
fn secondary(session: &Session<'_>, text: &str, answer: &mut Answer) -> Result<()> {
    for (id, _, _) in query::nearest(&session.conn, text, 6, &["investigation", "ticket"])? {
        let Some(rec) = query::record(&session.conn, &id)? else {
            continue;
        };
        let done = matches!(rec.status.as_deref(), Some("concluded" | "done"));
        if !done {
            continue;
        }
        let mut row = record_row(session, &rec);
        row.section = Some("secondary");
        if let Some(outcome) = outcome(session, &rec)? {
            row.more.push(outcome);
        }
        answer.rows.push(row);
    }
    Ok(())
}

fn outcome(session: &Session<'_>, rec: &Rec) -> Result<Option<More>> {
    let found: Option<(u32, u32)> = rusqlite::OptionalExtension::optional(session.conn.query_row(
        "SELECT line_first, line_last FROM sections WHERE path = ?1 AND name IN ('Outcome', 'Conclusion') ORDER BY ord LIMIT 1",
        [&rec.path],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    let Some((first, last)) = found else {
        return Ok(None);
    };
    let lines = session.lines(&rec.path, first + 1, last);
    Ok(lines
        .iter()
        .enumerate()
        .find(|(_, l)| !l.trim().is_empty())
        .map(|(offset, text)| More {
            label: "outcome",
            text: text.trim().to_owned(),
            path: Some(rec.path.clone()),
            line: Some(first + 1 + u32::try_from(offset).unwrap_or(0)),
            flags: Vec::new(),
        }))
}
