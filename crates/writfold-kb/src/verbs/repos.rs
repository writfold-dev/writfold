//! `repos [--missing]`: every repository the KB's code and gate citations
//! name, and whether `stale` can check them here. One row per distinct
//! `repo=`: `local`, `partial` (a partial clone: its code citations read as
//! unverifiable) or `missing` (no local clone: unverifiable as
//! `repo-not-local`), with the token count, a `url=` one of its tokens
//! gives, the clone found and the rule that found it, the commit `stale`
//! checks against and when the clone last fetched. A missing repository
//! spawns no Git.
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::session::Session;
use crate::stale::repos::{self, Via};
use crate::time;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The citations of one repository name.
#[derive(Default)]
struct Cited {
    tokens: usize,
    url: Option<String>,
    /// The first citing position.
    at: Option<(String, u32)>,
}

/// Every distinct repository name of the indexed code and gate citations.
fn cited(session: &Session<'_>) -> Result<BTreeMap<String, Cited>> {
    let mut statement = session.conn.prepare(
        "SELECT path, line, kind, repo, rev, fields FROM citations \
         WHERE illustrative = 0 AND malformed IS NULL AND kind IN ('code', 'gate') \
         ORDER BY path, ord",
    )?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, u32>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    let mut out: BTreeMap<String, Cited> = BTreeMap::new();
    for row in rows {
        let (path, line, kind, repo, rev, fields) = row?;
        let name = match kind.as_deref() {
            Some("gate") => rev.and_then(|r| r.split_once('@').map(|(n, _)| n.to_owned())),
            _ => repo,
        };
        let Some(name) = name else { continue };
        let entry = out.entry(name).or_default();
        entry.tokens += 1;
        entry.at.get_or_insert((path, line));
        if entry.url.is_none() {
            let fields: serde_json::Value = serde_json::from_str(&fields).unwrap_or_default();
            entry.url = fields
                .get("url")
                .and_then(|u| u.as_str())
                .map(str::to_owned);
        }
    }
    Ok(out)
}

fn more(label: &'static str, text: String, path: Option<String>) -> More {
    More {
        status: None,
        label,
        text,
        path,
        line: None,
        flags: Vec::new(),
    }
}

/// The status and continuation lines of a repository found at `dir`.
fn found(session: &Session<'_>, dir: PathBuf, via: Via) -> (&'static str, Vec<More>) {
    let status = if repos::partial(&dir) {
        "partial"
    } else {
        "local"
    };
    let target = session
        .git
        .as_ref()
        .and_then(|git| repos::target(git, &dir).ok().flatten())
        .map_or_else(
            || "none".to_owned(),
            |(commit, named)| {
                let short: String = commit.as_str().chars().take(12).collect();
                format!("{named} {short}")
            },
        );
    let fetched = repos::fetched(&dir).map_or_else(|| "none".to_owned(), time::stamp);
    let lines = vec![
        more(
            "resolved",
            via.name().to_owned(),
            Some(dir.display().to_string()),
        ),
        more("target", target, None),
        more("fetched", fetched, None),
    ];
    (status, lines)
}

pub fn repos(session: &Session<'_>, missing_only: bool) -> Result<Answer> {
    let mut answer = Answer::default();
    let (mut total, mut missing, mut partial) = (0, 0, 0);
    for (name, cited) in cited(session)? {
        total += 1;
        let (status, lines) =
            match repos::resolve_via(Some(session.ws), &name, cited.url.as_deref()) {
                Some((dir, via)) => found(session, dir, via),
                None => ("missing", Vec::new()),
            };
        missing += usize::from(status == "missing");
        partial += usize::from(status == "partial");
        if missing_only && status != "missing" {
            continue;
        }
        let mut row_more: Vec<More> = cited
            .url
            .iter()
            .map(|url| more("url", url.clone(), None))
            .collect();
        row_more.extend(lines);
        let (path, line) = cited.at.unzip();
        answer.rows.push(Row {
            kind: "repo",
            id: name,
            status: Some(status.to_owned()),
            text: format!("tokens={}", cited.tokens),
            path,
            line,
            flags: Vec::new(),
            more: row_more,
            section: None,
        });
    }
    if missing > 0 {
        answer.notes.push(
            "a missing repository's citations are unverifiable: name its clone in <git dir>/writfold/repos or clone it under a $WRITFOLD_KB_REPOS root".into(),
        );
    }
    if partial > 0 {
        answer.notes.push(
            "a partial clone's code citations are unverifiable (its blobs would need a fetch): use a full clone".into(),
        );
    }
    answer.counts = vec![("repos", total), ("missing", missing), ("partial", partial)];
    Ok(answer)
}
