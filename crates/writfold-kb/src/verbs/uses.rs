//! `uses <target>`: who relies on this. The target is a record (asset ID,
//! `ev:<home>/<stem>`, `sig:<id>`, `inv:<id>`, `proposal/<id>`, a ticket)
//! or a code locator (`<repo>@<commit>:<path>[:a-b]`, `code:<repo>:<path>`).
//! Referrers come from the edges table, so the ID-boundary rule of mentions
//! applies; the record's own home files are marked `self`.
use super::{BOUNDARY, ambiguous, link, node_row, unknown};
use crate::emit::{Answer, More};
use crate::error::Result;
use crate::index::query::{self, Edge, Rec, Resolved};
use crate::parse::cite::locator;
use crate::session::Session;
use std::collections::BTreeMap;

/// Order of referrer groups.
fn rank(kind: &str) -> u8 {
    match kind {
        "decision" => 0,
        "dossier" => 1,
        "signature" => 2,
        "investigation" => 3,
        "ticket" => 4,
        "runbook" => 5,
        "relationship" | "asset" => 6,
        "run" => 7,
        "project" | "evidence" => 8,
        _ => 9,
    }
}

/// A code target: its node and an optional line range.
fn code_target(target: &str) -> Option<(String, Option<(u32, u32)>)> {
    if let Some(cite) = locator(target, 1) {
        let node = format!("code:{}:{}", cite.repo?, cite.path?);
        return Some((node, cite.first.zip(cite.last)));
    }
    let rest = target.strip_prefix("code:")?;
    let (repo, path) = rest.split_once(':')?;
    match path.rsplit_once(':') {
        Some((file, range)) => {
            let (a, b) = range.split_once('-')?;
            let range = (a.parse().ok()?, b.parse().ok()?);
            Some((format!("code:{repo}:{file}"), Some(range)))
        }
        None => Some((format!("code:{repo}:{path}"), None)),
    }
}

/// Whether the citation behind a code edge overlaps `range`.
fn overlaps(session: &Session<'_>, edge: &Edge, range: (u32, u32)) -> Result<bool> {
    let lines: Option<(Option<u32>, Option<u32>)> =
        rusqlite::OptionalExtension::optional(session.conn.query_row(
            "SELECT l_first, l_last FROM citations WHERE path = ?1 AND ord = ?2",
            (&edge.path, &edge.cite),
            |r| Ok((r.get(0)?, r.get(1)?)),
        ))?;
    Ok(match lines {
        Some((Some(a), Some(b))) => a <= range.1 && range.0 <= b,
        _ => true,
    })
}

/// Whether a referrer is one of the target record's own home files.
fn home(target: &Rec, src: &str, src_path: Option<&str>) -> bool {
    if src == format!("dossier:{}", target.id) {
        return true;
    }
    let directory = target
        .path
        .rsplit_once('/')
        .map_or("", |(d, _)| d)
        .to_owned();
    let folder = target.path.ends_with("/README.md") || target.path.ends_with("/proposal.md");
    if folder && src_path.is_some_and(|p| p.starts_with(&format!("{directory}/"))) {
        return true;
    }
    match target
        .id
        .strip_prefix("ev:")
        .and_then(|r| r.split_once('/'))
    {
        Some((home, _)) => {
            src == format!("project:{home}") || src_path == Some(&format!("{home}/README.md"))
        }
        None => false,
    }
}

pub fn uses(session: &Session<'_>, target: &str) -> Result<Answer> {
    let (node, range, rec) = match code_target(target) {
        Some((node, range)) => (node, range, None),
        None => match query::resolve(&session.conn, target)? {
            Resolved::One(rec) => (rec.id.clone(), None, Some(*rec)),
            Resolved::Many(candidates) => return Ok(ambiguous(session, target, &candidates)),
            Resolved::None => return unknown(session, "unresolved-id", target, &[]),
        },
    };
    let mut groups: BTreeMap<(u8, String), Vec<More>> = BTreeMap::new();
    for edge in query::edges(&session.conn, &node, false)? {
        if let Some(range) = range
            && !overlaps(session, &edge, range)?
        {
            continue;
        }
        let row = node_row(session, &edge.src)?;
        let mut more = link(&edge);
        if edge.origin == "mention" || edge.status == "inferred" {
            more.flags.push("inferred".into());
        }
        groups
            .entry((rank(row.kind), edge.src.clone()))
            .or_default()
            .push(more);
    }
    let mut answer = Answer::default();
    for ((_, src), links) in groups {
        let mut row = node_row(session, &src)?;
        if rec
            .as_ref()
            .is_some_and(|r| home(r, &src, row.path.as_deref()))
        {
            row.flags.push("self".into());
        }
        row.section = Some(row.kind);
        row.more.extend(links.into_iter().take(3));
        answer.rows.push(row);
    }
    answer.notes.push(format!(
        "referrers of {node}{}; reliance that no record cites or mentions is not traced",
        range
            .map(|(a, b)| format!(" lines {a}-{b}"))
            .unwrap_or_default()
    ));
    if answer.rows.is_empty() {
        answer.notes.push(BOUNDARY.to_owned());
    }
    Ok(answer)
}
