//! `index status|rebuild`: what the index holds, and every unreadable file.
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::session::Session;

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
        ("edges", "SELECT COUNT(*) FROM edges"),
        (
            "unresolved",
            "SELECT COUNT(*) FROM refs WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.prov_path = refs.path AND e.prov_line = refs.line AND e.src = refs.src AND e.rel = refs.rel)",
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
    answer.rows.push(head);
    let mut statement = session.conn.prepare(
        "SELECT path, kind, coalesce(parse_error, '') FROM files WHERE parse_status <> 'ok' ORDER BY path",
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
