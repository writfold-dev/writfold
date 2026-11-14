//! `stale [<project>|<path>] [--offline|--cached]`: which pins have moved.
use super::record_row;
use crate::emit::{Answer, Row};
use crate::error::Result;
use crate::index::query;
use crate::layout::project_name;
use crate::parse::cite::tokens;
use crate::session::Session;
use crate::stale::{self, Checked, Cite, Verdict};
use crate::time;

/// Worst first: lower sorts earlier.
pub fn severity(verdict: &str) -> u8 {
    match verdict {
        "gone" => 0,
        "changed" => 1,
        "unverifiable" => 2,
        "moved" => 3,
        "expired" => 4,
        "marked" => 5,
        "due" => 6,
        _ => 7,
    }
}

/// A stable name for what a citation points at.
pub(crate) fn target(raw: &str) -> String {
    let Some(cite) = tokens(raw)
        .into_iter()
        .next()
        .filter(|c| c.malformed.is_none())
    else {
        let compact: String = raw.chars().take(60).collect();
        return compact;
    };
    let kind = cite.kind.clone().unwrap_or_else(|| "?".into());
    if kind == "code" || cite.repo.is_some() {
        let range = match (cite.first, cite.last) {
            (Some(a), Some(b)) => format!(":{a}-{b}"),
            _ => String::new(),
        };
        return format!(
            "code:{}:{}{range}",
            cite.repo.unwrap_or_default(),
            cite.path.unwrap_or_default()
        );
    }
    let fields: serde_json::Value = serde_json::from_str(&cite.fields).unwrap_or_default();
    let key = ["id", "path", "ref", "url", "q", "rev"]
        .iter()
        .find_map(|k| fields.get(*k).and_then(|v| v.as_str()).map(str::to_owned));
    format!("{kind}:{}", key.unwrap_or_default())
}

/// The flag a non-fresh verdict prints: `moved→a-b` (with the new path
/// when the file moved), `stale` for an authored mark, else its name.
pub fn flag(verdict: &str, moved: (Option<u32>, Option<u32>, Option<String>)) -> String {
    match (verdict, moved) {
        ("moved", (Some(a), Some(b), Some(p))) => format!("moved→{p}:{a}-{b}"),
        ("moved", (Some(a), Some(b), None)) => format!("moved→{a}-{b}"),
        ("marked", _) => "stale".to_owned(),
        (other, _) => other.to_owned(),
    }
}

/// The row for one non-fresh verdict (also used for cached verdicts).
pub fn verdict_row(
    path: &str,
    line: u32,
    raw: &str,
    verdict: &str,
    head: Option<&str>,
    moved: (Option<u32>, Option<u32>, Option<String>),
) -> Row {
    let flag = flag(verdict, moved);
    let at = head.map(|h| match h.split_once(':') {
        Some((commit, named)) if commit.len() >= 12 => format!("at {} ({named})", &commit[..12]),
        _ => format!("at {h}"),
    });
    Row {
        kind: "cite",
        id: target(raw),
        status: Some(verdict.to_owned()),
        text: at.unwrap_or_default(),
        path: Some(path.to_owned()),
        line: Some(line),
        flags: vec![flag],
        ..Row::default()
    }
}

fn load(session: &Session<'_>, scope: Option<&str>) -> Result<Vec<Cite>> {
    let prefix = match scope {
        Some(s) if !s.contains('/') && project_name(s).is_some() => format!("{s}/"),
        Some(s) => s.to_owned(),
        None => String::new(),
    };
    let mut statement = session.conn.prepare(
        "SELECT path, ord, line, kind, repo, rev, cpath, l_first, l_last, blob, span, seen, ttl, \
         state, marked, malformed, fields, raw FROM citations \
         WHERE illustrative = 0 AND substr(path, 1, length(?1)) = ?1 ORDER BY path, ord",
    )?;
    let found = statement
        .query_map([prefix], |r| {
            Ok(Cite {
                path: r.get(0)?,
                ord: r.get(1)?,
                line: r.get(2)?,
                kind: r.get(3)?,
                repo: r.get(4)?,
                rev: r.get(5)?,
                cpath: r.get(6)?,
                first: r.get(7)?,
                last: r.get(8)?,
                blob: r.get(9)?,
                span: r.get(10)?,
                seen: r.get(11)?,
                ttl: r.get(12)?,
                state: r.get(13)?,
                marked: r.get(14)?,
                malformed: r.get(15)?,
                fields: r.get(16)?,
                raw: r.get(17)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}

/// The reason or detail a verdict carries.
fn detail(verdict: &Verdict) -> Option<String> {
    match verdict {
        Verdict::Unverifiable(reason) => Some(reason.clone()),
        Verdict::Changed { detail } | Verdict::Gone { detail } => detail.clone(),
        Verdict::Marked(mark) => Some(format!("marked stale={mark}")),
        _ => None,
    }
}

fn remember(session: &Session<'_>, cite: &Cite, checked: &Checked) -> Result<()> {
    let (first, last, path) = match &checked.verdict {
        Verdict::Moved { path, first, last } => (Some(*first), Some(*last), path.clone()),
        _ => (None, None, None),
    };
    session
        .conn
        .prepare_cached(
            "INSERT OR REPLACE INTO cite_checks VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?
        .execute((
            &cite.path,
            &cite.ord,
            &checked.head,
            &checked.verdict.name(),
            &first,
            &last,
            &path,
            &detail(&checked.verdict),
            &time::stamp(session.now),
        ))?;
    Ok(())
}

pub fn stale(
    session: &Session<'_>,
    scope: Option<&str>,
    offline: bool,
    cached: bool,
) -> Result<Answer> {
    let cites = load(session, scope)?;
    let checked = if cached {
        stale::check(session, &cites, false)?
    } else {
        stale::check_asking(session, &cites, offline)?
    };
    let mut answer = Answer::default();
    let (mut flagged, mut unverifiable, mut absent) = (0, 0, 0);
    let mut rows = Vec::new();
    for (cite, checked) in cites.iter().zip(&checked) {
        if !offline && !cached {
            remember(session, cite, checked)?;
        }
        let name = checked.verdict.name();
        match &checked.verdict {
            Verdict::Fresh => continue,
            Verdict::Unverifiable(reason) => {
                unverifiable += 1;
                absent += usize::from(reason.starts_with("checker-absent"));
            }
            _ => flagged += 1,
        }
        let moved = match &checked.verdict {
            Verdict::Moved { path, first, last } => (Some(*first), Some(*last), path.clone()),
            _ => (None, None, None),
        };
        let mut row = verdict_row(
            &cite.path,
            cite.line,
            &cite.raw,
            name,
            Some(&checked.head),
            moved,
        );
        if let Some(detail) = detail(&checked.verdict) {
            row.text = if row.text.is_empty() {
                detail
            } else {
                format!("{} {detail}", row.text)
            };
        }
        rows.push(row);
    }
    let ttl = past_ttl(session, scope)?;
    flagged += ttl.len();
    rows.extend(ttl);
    let regressions = regressions(session)?;
    flagged += regressions.len();
    rows.extend(regressions);
    answer.rows = rows;
    answer.counts = vec![
        ("checked", cites.len()),
        ("stale", flagged),
        ("unverifiable", unverifiable),
    ];
    if absent > 0 {
        answer.counts.push(("checker-absent", absent));
        answer.notes.push(
            "a checked kind's checker is not on PATH; `writfold kb checker list` names it".into(),
        );
    }
    answer.notes.extend(never_note(session, &cites)?);
    if cached {
        answer.notes.push(
            "cached: no checker was started; a checked kind shows the verdict the last kb stale cached".into(),
        );
    }
    if offline {
        answer
            .notes
            .push("offline: only authored stale marks, due TTLs and expired checker verdicts are reported".into());
    }
    Ok(answer)
}

/// The note naming undeclared kinds whose tokens without `ttl=` never
/// expire, with their counts. Verdicts are unchanged.
fn never_note(session: &Session<'_>, cites: &[Cite]) -> Result<Option<String>> {
    let never = stale::kinds::never_expiring(&stale::kinds::read(&session.ws.root)?, cites);
    if never.is_empty() {
        return Ok(None);
    }
    let listed: Vec<String> = never.iter().map(|(k, n)| format!("{k} ({n})")).collect();
    Ok(Some(format!(
        "undeclared kinds whose tokens lack ttl= never expire: {}; a {}/<kind>.toml with default_ttl ages them",
        listed.join(", "),
        stale::kinds::DIR
    )))
}

/// Assets whose `verified_at + ttl` has passed.
fn past_ttl(session: &Session<'_>, scope: Option<&str>) -> Result<Vec<Row>> {
    let assets = query::records(&session.conn, "kind = 'asset' ORDER BY id", &[])?;
    let project = scope.filter(|s| !s.contains('/'));
    Ok(assets
        .iter()
        .filter(|rec| project.is_none_or(|p| rec.list("projects").iter().any(|q| q == p)))
        .filter(|rec| {
            rec.field("verified_at")
                .and_then(|v| time::expired(&v, rec.field("ttl").as_deref(), session.now))
                == Some(true)
        })
        .map(|rec| {
            let mut row = record_row(session, rec);
            row.text = format!(
                "verified_at {} ttl {}",
                rec.field("verified_at").unwrap_or_default(),
                rec.field("ttl").unwrap_or_default()
            );
            row
        })
        .collect())
}

/// Investigations opened after a signature they cite was marked fixed.
fn regressions(session: &Session<'_>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    let signatures = query::records(
        &session.conn,
        "kind = 'signature' AND status = 'fixed' ORDER BY id",
        &[],
    )?;
    for signature in signatures {
        let Some(fixed) = signature.field("fixed_at").and_then(|f| time::parse(&f)) else {
            continue;
        };
        for edge in query::edges(&session.conn, &signature.id, false)? {
            if edge.origin != "field" {
                continue;
            }
            let Some(inv) =
                query::record(&session.conn, &edge.src)?.filter(|r| r.kind == "investigation")
            else {
                continue;
            };
            if inv
                .created
                .as_deref()
                .and_then(time::parse)
                .is_some_and(|opened| opened > fixed)
            {
                let mut row = record_row(session, &inv);
                row.text = format!("opened after {} was marked fixed", signature.id);
                row.flags.push("regression".into());
                rows.push(row);
            }
        }
    }
    Ok(rows)
}
