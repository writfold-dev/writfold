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
use crate::stale::repos;
use crate::stale::target::{self as resolver, Asked as Ask};
use crate::store::{self, Freshness, Store};
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
        extra: Vec::new(),
        status: None,
        label,
        text,
        path,
        line: None,
        flags: Vec::new(),
    }
}

/// The status and continuation lines of a repository found at `dir`,
/// with its target chosen as `stale` chooses it (#179).
fn found(
    session: &Session<'_>,
    dir: PathBuf,
    via: &'static str,
    ask: &Ask<'_>,
) -> (&'static str, Vec<More>) {
    let resolved = session
        .git
        .as_ref()
        .and_then(|git| resolver::resolve(git, &dir, ask).ok());
    let ambiguous = resolved
        .as_ref()
        .is_some_and(|r| r.commit.is_none() && r.why.is_some());
    let status = if ambiguous {
        "ambiguous"
    } else if repos::partial(&dir) {
        "partial"
    } else {
        "local"
    };
    let target = resolved.map_or_else(
        || "none".to_owned(),
        |r| {
            let short: String = r
                .commit
                .map(|c| c.as_str().chars().take(12).collect())
                .unwrap_or_default();
            let head = match short.is_empty() {
                true => r.rule,
                false => format!("{} {short}", r.rule),
            };
            match r.why {
                Some(why) => format!("{head} ({why})"),
                None => head,
            }
        },
    );
    let fetched = repos::fetched(&dir).map_or_else(|| "none".to_owned(), time::stamp);
    let mut lines = vec![
        more("resolved", via.to_owned(), Some(dir.display().to_string())),
        more("target", target, None),
        more("fetched", fetched, None),
    ];
    // Shallow history: blobs at the target are all here, cited commits may
    // not be (#90). The status stays `local`.
    if repos::shallow(&dir) {
        lines.push(more(HISTORY, "shallow".to_owned(), None));
    }
    (status, lines)
}

const HISTORY: &str = "history";

/// A repository that is not a store: its clone, found as `stale` finds it,
/// and its target, or `missing`.
fn local(
    session: &Session<'_>,
    (name, url): (&str, Option<&str>),
    all: &[crate::declared::Declared],
) -> (&'static str, Vec<More>) {
    let declared = crate::declared::find(all, name);
    let shown = declared.map(|d| more("declared", d.url.clone(), Some(d.path.clone())));
    let (status, mut lines) = match repos::resolve_via(Some(session.ws), name, url) {
        Some((dir, via)) => {
            let pin = repos::mapped_ref(Some(session.ws), name);
            let ask = Ask {
                repo: name,
                url,
                pin: pin.as_deref(),
                named: matches!(via, repos::Via::Map | repos::Via::Path),
                declared,
            };
            found(session, dir, via.name(), &ask)
        }
        None => ("missing", Vec::new()),
    };
    lines.splice(0..0, shown);
    (status, lines)
}

pub fn repos(session: &Session<'_>, missing_only: bool) -> Result<Answer> {
    let mut answer = Answer::default();
    let (mut total, mut missing, mut partial, mut shallow) = (0, 0, 0, 0);
    let mut ambiguous = 0;
    let stores = store::read(&session.ws.root)?;
    let reads =
        store::material::read_all(session.git.as_ref(), session.ws, &session.conn, &stores)?;
    let mut all = cited(session)?;
    for declared in stores.iter() {
        all.entry(declared.name.clone()).or_default();
    }
    // A declared repository has a row even when nothing cites it yet (#197).
    let declared = crate::declared::read(&session.ws.root)?;
    for d in &declared {
        all.entry(d.name.clone()).or_default();
    }
    for (name, cited) in all {
        total += 1;
        let mut flags = Vec::new();
        let (status, lines) = match stores.get(&name) {
            Some(declared) => {
                let read = reads.iter().find(|r| r.name == name);
                store_row(session, declared, read, &mut flags)?
            }
            None => local(session, (&name, cited.url.as_deref()), &declared),
        };
        missing += usize::from(matches!(status, "missing" | "mismatch"));
        partial += usize::from(status == "partial");
        ambiguous += usize::from(status == "ambiguous");
        shallow += usize::from(lines.iter().any(|m| m.label == HISTORY));
        if missing_only && !matches!(status, "missing" | "mismatch") {
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
            flags,
            more: row_more,
            ..Row::default()
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
    if ambiguous > 0 {
        answer.notes.push(
            "an ambiguous clone's citations are unverifiable (target-ambiguous): add or fetch the remote that names the repository, or name the ref with `ref=REF` on its line in <git dir>/writfold/repos".into(),
        );
    }
    if shallow > 0 {
        answer.notes.push(
            "a shallow clone decides code citations from the bytes at its target (blob=, span=); a citation that needs the cited commit is unverifiable (history-not-local) until `git fetch --unshallow`".into(),
        );
    }
    answer.counts = vec![
        ("repos", total),
        ("missing", missing),
        ("partial", partial),
        ("shallow", shallow),
        ("ambiguous", ambiguous),
    ];
    Ok(answer)
}

/// A declared store's row: resolved by the proved store rule
/// ([`store::resolve`]), flagged `store` and with its freshness.
fn store_row(
    session: &Session<'_>,
    declared: &Store,
    read: Option<&store::material::Read>,
    flags: &mut Vec<String>,
) -> Result<(&'static str, Vec<More>)> {
    flags.push("store".into());
    flags.push(match declared.freshness {
        Freshness::Pinned => "pinned".into(),
        Freshness::Head => "head".into(),
    });
    let file = format!("{}/{}.toml", store::DIR, declared.name);
    let mut lines = vec![more("store", declared.url.clone(), Some(file))];
    let resolved = store::resolve(Some(session.ws), declared);
    if let Some(other) = &resolved.other_origin {
        flags.push("origin-differs".into());
        lines.push(more("origin", other.clone(), None));
    }
    let status = match resolved.git_dir {
        Some(dir) => {
            let pin = repos::mapped_ref(Some(session.ws), &declared.name);
            let ask = Ask {
                repo: &declared.name,
                url: Some(&declared.url),
                pin: pin.as_deref(),
                named: true,
                declared: None,
            };
            let (status, found) = found(session, dir, resolved.how, &ask);
            lines.extend(found);
            status
        }
        None => {
            lines.push(more("clone", format!("git clone {}", declared.url), None));
            resolved.how
        }
    };
    if let Some(read) = read.filter(|r| r.attached()) {
        let (listed, ambiguous, unknown, named) = super::material::tally(&session.conn, read)?;
        lines.push(more(
            "material",
            format!("material={listed} ambiguous={ambiguous} unknown={unknown}"),
            None,
        ));
        lines.extend(named.into_iter().map(|n| more("claim", n, None)));
    }
    Ok((status, lines))
}
