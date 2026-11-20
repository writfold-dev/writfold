//! Session start's "named here" part (#26): records that the session's
//! surroundings already name, looked up before the first prompt. The names
//! are the ticket-shaped words of the working repository's branch and of
//! the current run's task, and the repository's own directory name. Each
//! is looked up in the `exact` tier only ([`query::resolve`]: an ID, a
//! ticket, a typed ID, an alias or an asset locator), so a name that is not
//! a record adds nothing and no ranking is involved. Reads the HEAD file
//! and the derived index (which `brief` has just refreshed); spawns
//! nothing.
use crate::index::query::{self, Rec, Resolved};
use crate::parse::ticket;
use crate::workspace::Workspace;
use std::path::Path;

/// At most this many names are looked up, and this many rows shown.
const MAX_NAMES: usize = 6;
const MAX_ROWS: usize = 5;

/// The ticket-shaped words of `text`: `feature/hbr-101-flush` gives
/// `HBR-101`; `HBR101` and `hbr 101` too.
pub fn tickets(text: &str) -> Vec<String> {
    let words: Vec<&str> = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut found = Vec::new();
    for (i, word) in words.iter().enumerate() {
        let next = words.get(i.saturating_add(1)).copied().unwrap_or_default();
        let pair = (word.bytes().all(|b| b.is_ascii_alphabetic())
            && !next.is_empty()
            && next.bytes().all(|b| b.is_ascii_digit()))
        .then(|| ticket(&format!("{word}-{next}")))
        .flatten();
        for t in pair.into_iter().chain(ticket(word)) {
            if !found.contains(&t) {
                found.push(t);
            }
        }
    }
    found
}

/// The branch the working repository's HEAD names, without `refs/heads/`.
fn branch(top: &Path) -> Option<String> {
    let dot = top.join(".git");
    let git_dir = if dot.is_file() {
        let text = std::fs::read_to_string(&dot).ok()?;
        top.join(text.strip_prefix("gitdir:")?.trim())
    } else {
        dot
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let name = head.strip_prefix("ref: refs/heads/")?.trim();
    Some(name.to_owned())
}

/// The names to look up, each with where it came from.
fn names(ws: &Workspace, cwd: &Path, task: Option<&str>) -> Vec<(String, String)> {
    let mut names: Vec<(String, String)> = Vec::new();
    let top = cwd.ancestors().find(|d| d.join(".git").exists());
    if let Some(name) = top.and_then(branch) {
        for t in tickets(&name) {
            names.push((t, format!("branch {name}")));
        }
    }
    if let Some(task) = task {
        for t in tickets(task) {
            names.push((t, "the run's task".into()));
        }
    }
    let repo_name = top
        .filter(|t| std::fs::canonicalize(t).ok() != std::fs::canonicalize(&ws.root).ok())
        .and_then(|t| t.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    if let Some(name) = repo_name {
        names.push((name, "the repository's name".into()));
    }
    let mut seen = Vec::new();
    names.retain(|(n, _)| {
        let new = !seen.contains(n);
        seen.push(n.clone());
        new
    });
    names.truncate(MAX_NAMES);
    names
}

fn row(rec: &Rec, from: &str) -> String {
    let status = rec
        .status
        .as_deref()
        .map(|s| format!(" [{s}]"))
        .unwrap_or_default();
    let title = rec.title.as_deref().unwrap_or_default();
    format!(
        "{} {}{status} {title} @{}:{} (named by {from})\n",
        rec.kind, rec.id, rec.path, rec.line
    )
}

/// The part, or `None` when no name is a record (or there is no index).
pub fn block(ws: &Workspace, cwd: &Path, run: Option<&str>) -> Option<String> {
    let file = crate::index::index_file(ws.repo.as_ref()?).ok()?;
    if !file.is_file() {
        return None;
    }
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
    let conn = rusqlite::Connection::open_with_flags(&file, flags).ok()?;
    let task = run
        .and_then(|r| query::record(&conn, r).ok().flatten())
        .and_then(|rec| rec.title);
    let mut rows: Vec<String> = Vec::new();
    for (name, from) in names(ws, cwd, task.as_deref()) {
        let found = match query::resolve(&conn, &name).ok()? {
            Resolved::One(rec) => vec![*rec],
            Resolved::Many(recs) => recs,
            Resolved::None => Vec::new(),
        };
        for rec in found {
            let line = row(&rec, &from);
            if !rows.contains(&line) && rows.len() < MAX_ROWS {
                rows.push(line);
            }
        }
    }
    (!rows.is_empty()).then(|| {
        format!(
            "\nNamed here (exact matches for the branch, the run's task and the repository; `writfold kb find NAME` for more):\n{}",
            rows.concat()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::tickets;

    #[test]
    fn ticket_shaped_words() {
        assert_eq!(tickets("feature/hbr-101-flush"), ["HBR-101"]);
        assert_eq!(tickets("HBR101 and lan 20"), ["HBR-101", "LAN-20"]);
        assert!(tickets("main").is_empty());
        // Only a candidate: looked up exactly, a miss adds nothing.
        assert_eq!(tickets("release-2026"), ["RELEASE-2026"]);
    }
}
