//! `brief [<project>|--all]`: where do we stand. Sections in fixed priority,
//! each with a share of the budget; rows beyond a share move after every
//! section, so the cursor still resumes them in order.
use super::{cached_verdicts, record_row, why};
use crate::emit::{Answer, More, Row, render_row};
use crate::error::Result;
use crate::index::query::{self, Rec};
use crate::session::Session;
use crate::stale::repos;
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_runtime::git::{KbRepository, kb_config, kb_log};

/// Sections and their budget shares, in percent.
const SHARES: [(&str, usize); 6] = [
    ("header", 10),
    ("checkpoint", 25),
    ("open", 25),
    ("decisions", 20),
    ("commits", 10),
    ("stale", 10),
];

/// Which project a brief is for.
pub enum Home {
    Project(String),
    All,
}

fn last_two(url: &str) -> Option<String> {
    let url = url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_lowercase();
    let mut parts = url.rsplit(['/', ':']);
    let (name, org) = (parts.next()?, parts.next()?);
    (!name.is_empty() && !org.is_empty()).then(|| format!("{org}/{name}"))
}

/// The project for `cwd`: a project directory of the KB containing it, else
/// the single project of the assets whose locator matches the enclosing
/// repository's `origin`.
pub fn infer(session: &Session<'_>, cwd: &Path) -> Result<Option<String>> {
    if let Ok(inside) = cwd.strip_prefix(&session.ws.root)
        && let Some(top) = inside
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
        && query::record(&session.conn, &format!("project:{top}"))?.is_some()
    {
        return Ok(Some(top.to_owned()));
    }
    let (Some(git), Some(dir)) = (&session.git, cwd.ancestors().find_map(repos::git_dir)) else {
        return Ok(None);
    };
    let reads = KbRepository {
        git_dir: &dir,
        work_tree: None,
    };
    let Some(origin) = kb_config(git, reads, "remote.origin.url", budgets()?, &CANCELLED)?
        .and_then(|u| last_two(&u))
    else {
        return Ok(None);
    };
    let assets = query::records(&session.conn, "kind = 'asset' ORDER BY id", &[])?;
    let mut projects: Vec<String> = assets
        .iter()
        .filter(|a| {
            a.field("locator").and_then(|l| last_two(&l)).as_deref() == Some(origin.as_str())
        })
        .flat_map(|a| a.list("projects"))
        .collect();
    projects.sort();
    projects.dedup();
    Ok((projects.len() == 1).then(|| projects.remove(0)))
}

pub fn brief(session: &Session<'_>, home: &Home, budget: usize) -> Result<Answer> {
    let mut answer = Answer::default();
    let project = match home {
        Home::Project(name) => match query::record(&session.conn, &format!("project:{name}"))? {
            Some(rec) => Some((name.clone(), rec)),
            None => {
                answer.unknown = Some("no-home");
                answer.notes.push(format!(
                    "no project {name:?} in this KB; showing a KB-wide brief"
                ));
                None
            }
        },
        Home::All => None,
    };
    let scope = project.as_ref().map(|(name, _)| name.as_str());
    let mut sections: Vec<(&'static str, Vec<Row>)> = Vec::new();
    sections.push((
        "header",
        header(session, project.as_ref().map(|(_, rec)| rec))?,
    ));
    let (checkpoint, since) = checkpoint(session, scope)?;
    sections.push(("checkpoint", checkpoint));
    sections.push(("open", open(session, scope)?));
    sections.push(("decisions", decisions(session, scope)?));
    sections.push(("commits", commits(session, scope, since.as_deref())?));
    let (stale, unchecked) = stale(session, scope)?;
    if !stale.is_empty() {
        answer.counts.push(("stale", stale.len()));
    }
    sections.push(("stale", stale));
    if unchecked > 0 {
        answer.notes.push(format!(
            "{unchecked} citation(s) have no cached verdict; run `writfold kb stale{}`",
            scope.map(|s| format!(" {s}")).unwrap_or_default()
        ));
    }
    answer.rows = share(sections, budget);
    Ok(answer)
}

/// Fit each section into its share (unused share carries forward); the rows
/// that do not fit follow every section, in section order.
fn share(sections: Vec<(&'static str, Vec<Row>)>, budget: usize) -> Vec<Row> {
    let (mut taken, mut overflow) = (Vec::new(), Vec::new());
    let (mut carry, mut used) = (0usize, 0usize);
    for (name, rows) in sections {
        let percent = SHARES
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(0, |(_, p)| *p);
        let mut allowance = budget * percent / 100 + carry;
        let mut fitting = true;
        for (index, mut row) in rows.into_iter().enumerate() {
            row.section = Some(name);
            let cost: usize = render_row(&row)
                .split('\n')
                .map(|l| (l.len() + 1).div_ceil(4))
                .sum();
            // A section's first row may exceed its share while the whole
            // budget still has room, so no section is starved entirely.
            let fits = cost <= allowance || (index == 0 && used + cost <= budget);
            if fitting && fits {
                allowance = allowance.saturating_sub(cost);
                used += cost;
                taken.push(row);
            } else {
                fitting = false;
                overflow.push(row);
            }
        }
        carry = allowance;
    }
    taken.extend(overflow);
    taken
}

fn header(session: &Session<'_>, project: Option<&Rec>) -> Result<Vec<Row>> {
    if let Some(rec) = project {
        let mut row = record_row(session, rec);
        if let Ok(fields) =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&rec.extra)
        {
            for (key, value) in fields
                .iter()
                .filter(|(k, _)| !matches!(k.as_str(), "phase" | "title"))
                .take(6)
            {
                let value = value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned);
                row.more.push(More {
                    label: "field",
                    text: format!("{key}={value}"),
                    ..More::default()
                });
            }
        }
        return Ok(vec![row]);
    }
    let projects = query::records(&session.conn, "kind = 'project' ORDER BY id", &[])?;
    Ok(projects
        .iter()
        .map(|rec| record_row(session, rec))
        .collect())
}

/// The newest run's last checkpoint entry, and its date (for `commits`).
fn checkpoint(session: &Session<'_>, scope: Option<&str>) -> Result<(Vec<Row>, Option<String>)> {
    let runs = query::records(
        &session.conn,
        "kind = 'run' AND (?1 IS NULL OR project = ?1) ORDER BY coalesce(created, '') DESC, id DESC LIMIT 1",
        &[&scope],
    )?;
    let Some(run) = runs.into_iter().next() else {
        return Ok((Vec::new(), None));
    };
    let directory = run.path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
    let path = format!("{directory}/checkpoint.md");
    let text = session.text(&path).unwrap_or_default();
    let entries = crate::parse::sections(&text);
    let mut row = record_row(session, &run);
    let mut since = run.created.clone();
    if let Some(entry) = entries.last() {
        row.text = format!("{} (checkpoint {})", run.label(), entry.name);
        row.path = Some(path.clone());
        row.line = Some(entry.first);
        since = Some(entry.name.clone());
        for (offset, line) in session
            .lines(&path, entry.first + 1, entry.last)
            .iter()
            .enumerate()
        {
            if !line.trim().is_empty() && row.more.len() < 12 {
                let at = entry.first + 1 + u32::try_from(offset).unwrap_or(0);
                row.more.push(More {
                    status: None,
                    label: "note",
                    text: line.trim().to_owned(),
                    path: Some(path.clone()),
                    line: Some(at),
                    flags: Vec::new(),
                });
            }
        }
    }
    Ok((vec![row], since))
}

/// Open ticket records and investigations, and decisions not yet decided or
/// still landing. Accepted decisions are listed once, under `decisions`.
fn open(session: &Session<'_>, scope: Option<&str>) -> Result<Vec<Row>> {
    let in_scope = "(?1 IS NULL OR project = ?1 OR EXISTS (SELECT 1 FROM edges e JOIN edges p ON p.src = e.dst \
                    WHERE e.src = records.id AND p.rel = 'project' AND p.dst = 'project:' || ?1))";
    let clause = format!(
        "((kind = 'ticket' AND coalesce(status, '') NOT IN ('done', 'n/a')) \
          OR (kind = 'investigation' AND coalesce(status, '') NOT IN ('concluded', 'abandoned')) \
          OR (kind = 'decision' AND status IN ('draft', 'proposed', 'landing'))) \
         AND {in_scope} ORDER BY coalesce(updated, created, '') DESC, id"
    );
    let found = query::records(&session.conn, &clause, &[&scope])?;
    Ok(found.iter().map(|rec| record_row(session, rec)).collect())
}

fn decisions(session: &Session<'_>, scope: Option<&str>) -> Result<Vec<Row>> {
    let found = query::records(
        &session.conn,
        "kind = 'decision' AND status IN ('accepted', 'applied', 'verified') AND (?1 IS NULL OR project = ?1) \
         ORDER BY coalesce(updated, created, '') DESC, id LIMIT 20",
        &[&scope],
    )?;
    let mut rows = Vec::new();
    for rec in found {
        let mut row = record_row(session, &rec);
        row.more
            .extend(why::rationale(session, &rec, &[])?.into_iter().take(1));
        rows.push(row);
    }
    Ok(rows)
}

fn commits(session: &Session<'_>, scope: Option<&str>, since: Option<&str>) -> Result<Vec<Row>> {
    let (Some(git), Some(repo), Some(_)) = (&session.git, &session.ws.repo, &session.pin.head)
    else {
        return Ok(Vec::new());
    };
    let pathspec = match scope {
        Some(name) => format!("{}{name}", repo.prefix),
        None => repo.pathspec().to_owned(),
    };
    let since = since.map(|s| s.get(..10).unwrap_or(s));
    let found = kb_log(
        git,
        repo.reads(),
        &pathspec,
        10,
        since,
        budgets()?,
        &CANCELLED,
    )?;
    Ok(found
        .into_iter()
        .map(|commit| Row {
            kind: "commit",
            id: commit.id.chars().take(12).collect(),
            text: format!("{} {}", commit.date, commit.subject),
            ..Row::default()
        })
        .collect())
}

fn stale(session: &Session<'_>, scope: Option<&str>) -> Result<(Vec<Row>, usize)> {
    let prefix = scope.map(|s| format!("{s}/")).unwrap_or_default();
    let mut statement = session.conn.prepare_cached(
        "SELECT DISTINCT path FROM citations WHERE substr(path, 1, length(?1)) = ?1 ORDER BY path",
    )?;
    let paths = statement
        .query_map([prefix], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    cached_verdicts(session, &paths, 5)
}
