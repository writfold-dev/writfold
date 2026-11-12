//! `context <id|alias|locator|ticket>`: everything around one thing.
use super::{ambiguous, cached_verdicts, record_row, unknown, via};
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec, Resolved};
use crate::session::Session;
use std::collections::{BTreeMap, BTreeSet};

/// Open-ended ticket and investigation states; others are closed.
fn closed(rec: &Rec) -> bool {
    matches!(
        (rec.kind.as_str(), rec.status.as_deref()),
        ("ticket", Some("done" | "n/a")) | ("investigation", Some("concluded" | "abandoned"))
    )
}

/// Order of incoming referrer kinds.
fn rank(kind: &str) -> u8 {
    match kind {
        "ticket" => 0,
        "investigation" => 1,
        "signature" => 2,
        "runbook" => 3,
        "decision" => 4,
        "dossier" => 5,
        _ => 6,
    }
}

pub fn context(session: &Session<'_>, target: &str, depth: usize) -> Result<Answer> {
    let rec = match query::resolve(&session.conn, target)? {
        Resolved::One(rec) => *rec,
        Resolved::Many(candidates) => return Ok(ambiguous(session, target, &candidates)),
        Resolved::None => return unknown(session, "unresolved-id", target, &[]),
    };
    let mut answer = Answer::default();
    answer.rows.push(primary(session, &rec));
    let mut shown: BTreeSet<String> = BTreeSet::from([rec.id.clone()]);
    siblings(session, &rec, &mut answer, &mut shown)?;
    neighbours(session, &rec, &mut answer, &mut shown, depth)?;
    dossier(session, &rec, &mut answer)?;
    runs(session, &rec, &mut answer, &shown)?;
    let mut paths = vec![rec.path.clone()];
    if let Some(dossier) = query::record(&session.conn, &format!("dossier:{}", rec.id))? {
        paths.push(dossier.path);
    }
    let (rows, unchecked) = cached_verdicts(session, &paths, 5)?;
    if !rows.is_empty() {
        answer.counts.push(("stale", rows.len()));
    }
    answer.rows.extend(rows);
    if unchecked > 0 {
        answer.notes.push(format!(
            "{unchecked} citation(s) in the home files have no cached verdict; run `writfold kb stale {}`",
            rec.path
        ));
    }
    Ok(answer)
}

fn primary(session: &Session<'_>, rec: &Rec) -> Row {
    let mut row = record_row(session, rec);
    for key in ["kind", "owner", "locator", "projects", "verified_at", "ttl"] {
        if let Some(value) = rec.field(key).filter(|_| rec.kind == "asset") {
            row.more.push(More {
                label: "field",
                text: format!("{key}={value}"),
                ..More::default()
            });
        }
    }
    if let Some(summary) = &rec.summary {
        row.more.push(More {
            label: "summary",
            text: summary.clone(),
            ..More::default()
        });
    }
    row
}

/// Assets sharing this asset's locator.
fn siblings(
    session: &Session<'_>,
    rec: &Rec,
    answer: &mut Answer,
    shown: &mut BTreeSet<String>,
) -> Result<()> {
    let Some(locator) = rec.field("locator").filter(|_| rec.kind == "asset") else {
        return Ok(());
    };
    let found = query::records(
        &session.conn,
        "kind = 'asset' AND json_extract(extra, '$.locator') = ?1 AND id <> ?2 ORDER BY id",
        &[&locator, &rec.id],
    )?;
    for sibling in found {
        let mut row = record_row(session, &sibling);
        row.more.push(More {
            label: "via",
            text: format!("same locator {locator}"),
            ..More::default()
        });
        shown.insert(sibling.id.clone());
        answer.rows.push(row);
    }
    Ok(())
}

/// Depth 1 (homes, referrers, targets) then a bounded breadth-first walk;
/// the trailer counts the frontier left unvisited.
fn neighbours(
    session: &Session<'_>,
    rec: &Rec,
    answer: &mut Answer,
    shown: &mut BTreeSet<String>,
    depth: usize,
) -> Result<()> {
    let mut level = vec![rec.id.clone()];
    let mut frontier = 0usize;
    for step in 1..=depth.max(1) + 1 {
        let mut found: BTreeMap<(u8, String), (Rec, More)> = BTreeMap::new();
        let mut next = Vec::new();
        for id in &level {
            for (edge, outgoing) in query::edges(&session.conn, id, true)?
                .into_iter()
                .map(|e| (e, true))
                .chain(
                    query::edges(&session.conn, id, false)?
                        .into_iter()
                        .map(|e| (e, false)),
                )
            {
                let other = if outgoing { &edge.dst } else { &edge.src };
                if shown.contains(other)
                    || !matches!(edge.status.as_str(), "explicit" | "confirmed")
                    || !matches!(edge.origin.as_str(), "authored" | "field")
                {
                    continue;
                }
                if step > depth.max(1) {
                    frontier += 1;
                    continue;
                }
                let Some(neighbour) = query::record(&session.conn, other)? else {
                    continue;
                };
                if closed(&neighbour) || neighbour.kind == "dossier" {
                    continue;
                }
                let rel = if outgoing {
                    edge.rel.clone()
                } else {
                    format!("{} (from)", edge.rel)
                };
                let more = via(&rel, id, &edge.path, edge.line);
                found
                    .entry((rank(&neighbour.kind), neighbour.id.clone()))
                    .or_insert((neighbour, more));
            }
        }
        for ((_, id), (neighbour, more)) in found {
            let mut row = record_row(session, &neighbour);
            row.more.push(more);
            if step > 1 {
                row.flags.push(format!("depth={step}"));
            }
            answer.rows.push(row);
            shown.insert(id.clone());
            // A project is a home, not a dependency: the walk stops there.
            if neighbour.kind != "project" {
                next.push(id);
            }
        }
        level = next;
    }
    if frontier > 0 {
        answer.counts.push(("frontier", frontier));
    }
    Ok(())
}

fn dossier(session: &Session<'_>, rec: &Rec, answer: &mut Answer) -> Result<()> {
    let Some(dossier) = query::record(&session.conn, &format!("dossier:{}", rec.id))? else {
        return Ok(());
    };
    let mut statement = session
        .conn
        .prepare_cached("SELECT name, line_first FROM sections WHERE path = ?1 ORDER BY ord")?;
    let headings = statement
        .query_map([&dossier.path], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut row = record_row(session, &dossier);
    for (name, line) in headings {
        row.more.push(More {
            label: "section",
            text: name,
            path: Some(dossier.path.clone()),
            line: Some(line),
            flags: Vec::new(),
        });
    }
    answer.rows.push(row);
    Ok(())
}

/// Runs whose files mention the ID.
fn runs(
    session: &Session<'_>,
    rec: &Rec,
    answer: &mut Answer,
    shown: &BTreeSet<String>,
) -> Result<()> {
    let phrase = format!("\"{}\"", rec.id.replace('"', "\"\""));
    let mut statement = session.conn.prepare_cached(
        "SELECT DISTINCT path FROM fts WHERE fts MATCH ?1 AND path LIKE 'runs/%' ORDER BY path",
    )?;
    let paths = statement
        .query_map([phrase], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut seen = BTreeSet::new();
    for path in paths {
        let Some(directory) = path.split('/').nth(1) else {
            continue;
        };
        let manifest = format!("runs/{directory}/manifest.json");
        let found = query::records(&session.conn, "path = ?1", &[&manifest])?;
        for run in found {
            if !shown.contains(&run.id) && seen.insert(run.id.clone()) {
                let mut row = record_row(session, &run);
                row.more.push(More {
                    label: "via",
                    text: "mentions".into(),
                    path: Some(path.clone()),
                    line: None,
                    flags: Vec::new(),
                });
                answer.rows.push(row);
            }
        }
    }
    Ok(())
}
