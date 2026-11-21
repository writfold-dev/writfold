//! Edit receipts that `kb edit apply` wrote before it was removed
//! (21 November 2026, #54): `runs/<id>/edits/<time>-<hex>.md`. They stay
//! readable, so `resume` still lists a run's newest ones and a cover row
//! that cites one still grounds (`gate::ground`). Nothing writes them now.
use crate::emit::{More, Row};
use crate::error::Result;
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use writfold_sources::frontmatter::parse_frontmatter;

/// The directory of a run that holds its edit receipts.
pub const DIR: &str = "edits";
/// Receipts `resume` shows, newest first.
const SHOWN: usize = 3;
/// Files shown under each receipt in `resume`.
const FILES_SHOWN: usize = 5;

/// The run's edit receipts, newest first: the newest few as rows, and how
/// many there are.
pub fn rows(session: &Session<'_>, rec: &Rec) -> Result<(Vec<Row>, usize)> {
    let prefix = format!("{}/{DIR}/", runs::dir(rec));
    let width = i64::try_from(prefix.len()).unwrap_or(i64::MAX);
    let mut statement = session.conn.prepare_cached(
        "SELECT path FROM files WHERE substr(path, 1, ?1) = ?2 AND path LIKE '%.md' \
         ORDER BY path DESC",
    )?;
    let found = statement
        .query_map((width, &prefix), |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    let total = found.len();
    let out = found.iter().take(SHOWN).map(|p| row(session, p)).collect();
    Ok((out, total))
}

fn row(session: &Session<'_>, path: &str) -> Row {
    let text = session.text(path).unwrap_or_default();
    let fields: Vec<(String, String)> = parse_frontmatter(&text)
        .map(|b| {
            b.entries()
                .map(|(k, v)| (k.to_owned(), v.trim().to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let get = |key: &str| {
        fields
            .iter()
            .find(|(k, _)| k == key)
            .map_or("?", |(_, v)| v.as_str())
    };
    let head = get("head");
    let mut row = Row {
        kind: "edit",
        id: crate::layout::stem(path).to_owned(),
        text: format!(
            "splicery files={} splices={} at {}@{}",
            get("files"),
            get("splices"),
            get("repo"),
            head.get(..12).unwrap_or(head),
        ),
        path: Some(path.to_owned()),
        line: Some(1),
        ..Row::default()
    };
    let entries: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with("- `"))
        .collect();
    for (n, line) in entries.iter().take(FILES_SHOWN) {
        row.more.push(More {
            extra: Vec::new(),
            status: None,
            label: "file",
            text: summary(line),
            path: Some(path.to_owned()),
            line: u32::try_from(n + 1).ok(),
            flags: Vec::new(),
        });
    }
    if entries.len() > FILES_SHOWN {
        row.more.push(More {
            label: "file",
            text: format!("{} more files", entries.len() - FILES_SHOWN),
            path: Some(path.to_owned()),
            ..More::default()
        });
    }
    row
}

/// `- `p` before=B after=A splices=…` as `p B12→A12 splices=…`.
fn summary(line: &str) -> String {
    let body = line.trim_start_matches("- `");
    let (path, rest) = body.split_once('`').unwrap_or((body, ""));
    let mut parts = Vec::new();
    let (mut before, mut after) = ("?", "?");
    for word in rest.split_whitespace() {
        match word.split_once('=') {
            Some(("before", v)) => before = v,
            Some(("after", v)) => after = v,
            Some(("splices", _)) | Some(("recipe", _)) => parts.push(word),
            _ => {}
        }
    }
    let short = |t: &str| t.get(..12).unwrap_or(t).to_owned();
    format!(
        "{path} {}->{} {}",
        short(before),
        short(after),
        parts.join(" ")
    )
}
