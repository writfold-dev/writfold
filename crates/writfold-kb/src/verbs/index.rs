//! `index status|rebuild|verify`: what the index holds, every unreadable
//! file, and whether the index equals a rebuild.
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::{
    self, build,
    verify::{self, Report},
};
use crate::session::Session;
use rusqlite::{Connection, OptionalExtension};

pub fn status(session: &Session<'_>) -> Result<Answer> {
    let mut answer = Answer::default();
    let location = session.file.as_ref().map_or_else(
        || "in memory (KB not under Git)".to_owned(),
        |f| f.display().to_string(),
    );
    let mut head = Row {
        kind: "index",
        id: session.pin.label(),
        status: Some(session.index.label().to_owned()),
        text: location,
        ..Row::default()
    };
    let mut statement = session
        .conn
        .prepare("SELECT kind, COUNT(*) FROM records GROUP BY kind ORDER BY kind")?;
    let kinds = statement
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, usize>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (kind, count) in kinds {
        head.more.push(More {
            label: "records",
            text: format!("{kind}={count}"),
            ..More::default()
        });
    }
    for (label, sql) in [
        ("files", "SELECT COUNT(*) FROM files"),
        (
            "citations",
            "SELECT COUNT(*) FROM citations WHERE illustrative = 0",
        ),
        (
            "edges",
            "SELECT COUNT(*) FROM edges WHERE origin <> 'mention'",
        ),
        (
            "mentions",
            "SELECT COUNT(*) FROM edges WHERE origin = 'mention'",
        ),
        (
            "unresolved",
            "SELECT COUNT(*) FROM refs WHERE origin <> 'mention' AND NOT EXISTS (SELECT 1 FROM edges e WHERE e.prov_path = refs.path AND e.prov_line = refs.line AND e.src = refs.src AND e.rel = refs.rel)",
        ),
        ("cached-verdicts", "SELECT COUNT(*) FROM cite_checks"),
    ] {
        let count: usize = session.conn.query_row(sql, [], |r| r.get(0))?;
        head.more.push(More {
            label: "count",
            text: format!("{label}={count}"),
            ..More::default()
        });
    }
    // The graph work of the last in-place refresh (#59), when there was one
    // since the last rebuild.
    let last: Option<String> = session
        .conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'last_refresh'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(text) = last {
        head.more.push(More {
            label: "last-refresh",
            text,
            ..More::default()
        });
    }
    answer.rows.push(head);
    let mut statement = session.conn.prepare(
        "SELECT f.path, f.kind, coalesce(f.parse_error, 'duplicate id ' || d.id, '') FROM files f \
         LEFT JOIN dups d ON d.path = f.path WHERE f.parse_status <> 'ok' OR d.path IS NOT NULL ORDER BY f.path",
    )?;
    let unreadable = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (path, kind, reason) in unreadable {
        answer.rows.push(Row {
            kind: "unreadable",
            id: kind,
            text: reason,
            path: Some(path),
            line: None,
            flags: vec!["unreadable".into()],
            ..Row::default()
        });
    }
    Ok(answer)
}

/// Rebuild the same snapshot and compare it with the index, table by
/// table; a difference replaces the index with the rebuild and fails.
pub fn verify(session: &Session<'_>, after_build: &dyn Fn()) -> Result<Answer> {
    let report = match (&session.ws.repo, &session.git, &session.file) {
        (Some(repo), Some(git), Some(file)) => {
            let snapshot = index::observe(session.ws, repo, git)?;
            verify::against_rebuild(session.ws, repo, git, file, &snapshot, after_build)?
        }
        _ => outside_git(session)?,
    };
    Ok(render(&report))
}

/// A KB outside Git: rebuild into a temporary file (never into memory, so
/// a large KB needs disk, not RAM) and compare; nothing is replaced, since
/// such an index is rebuilt on every read anyway.
fn outside_git(session: &Session<'_>) -> Result<Report> {
    let root = &session.ws.root;
    let dir = std::env::temp_dir().join(format!("writfold-verify-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("rebuild.sqlite");
    let built = (|| -> Result<Report> {
        let mut rebuilt = Connection::open(&file)?;
        rebuilt.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
        build::build(&mut rebuilt, index::inputs(root, index::walk(root)?), &[])?;
        verify::compare(&session.conn, &rebuilt)
    })();
    #[expect(
        clippy::let_underscore_must_use,
        reason = "best-effort cleanup of a temporary directory"
    )]
    let _ = std::fs::remove_dir_all(&dir);
    built
}

fn render(report: &Report) -> Answer {
    let mut answer = Answer::default();
    for table in &report.tables {
        let equal = table.difference.is_none() && table.digest == table.rebuilt_digest;
        let mut row = Row {
            kind: "table",
            id: table.name.to_owned(),
            status: Some(if equal { "equal" } else { "differs" }.to_owned()),
            text: format!("rows={} sha256={}", table.rows, &table.digest[..12]),
            ..Row::default()
        };
        if let Some(key) = &table.difference {
            row.more.push(More {
                label: "first-difference",
                text: key.clone(),
                ..More::default()
            });
            row.more.push(More {
                label: "rebuilt",
                text: format!("sha256={}", &table.rebuilt_digest[..12]),
                ..More::default()
            });
        }
        answer.rows.push(row);
    }
    let integrity = Row {
        kind: "check",
        id: "fts-integrity".into(),
        status: Some(
            if report.integrity.is_empty() {
                "ok"
            } else {
                "failed"
            }
            .to_owned(),
        ),
        text: report.integrity.join("; "),
        ..Row::default()
    };
    answer.rows.push(integrity);
    if report.moved {
        answer.partial = true;
        answer.notes.push(
            "the KB or the index moved while it was compared: nothing was replaced and nothing is claimed about the index; run it again".into(),
        );
        return answer;
    }
    if let Some(difference) = report.first_difference() {
        answer.failed = true;
        answer.notes.push(match report.replaced {
            true => format!(
                "the index differed from a rebuild at {difference}; the rebuild replaced it"
            ),
            false => format!("the index differs from a rebuild at {difference}"),
        });
    }
    answer
}
