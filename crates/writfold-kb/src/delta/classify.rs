//! What a changed KB path is: a record, a run's artefact, configuration,
//! or another file.
use rusqlite::Connection;

/// A changed path's kind, its ID (the record's, else the path) and its
/// section: `records`, `run`, `config` or `other`.
pub struct Class {
    pub kind: String,
    pub id: String,
    pub section: &'static str,
}

const RUN: [(&str, &str); 9] = [
    ("gates", "gate-receipt"),
    ("statements", "statement"),
    ("covers", "cover"),
    ("observed", "observation"),
    ("retrievals", "retrieval"),
    ("edits", "edit-receipt"),
    ("plan", "plan-event"),
    ("deliveries", "delivery"),
    ("policy", "policy-pin"),
];

const CONFIG: [(&str, &str); 7] = [
    ("gates", "gate"),
    ("guards", "guard"),
    ("plans", "plan"),
    ("sources", "kind"),
    ("checks", "check-receipt"),
    ("proposals", "proposal"),
    ("deliveries", "delivery-mode"),
];

fn class(kind: &str, id: &str, section: &'static str) -> Class {
    Class {
        kind: kind.to_owned(),
        id: id.to_owned(),
        section,
    }
}

/// The record at `path` in the index, if one is there.
fn indexed(conn: &Connection, path: &str) -> Option<(String, String)> {
    conn.query_row(
        "SELECT id, kind FROM records WHERE path = ?1 ORDER BY id LIMIT 1",
        [path],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .ok()
}

/// Classify one KB-relative path.
pub fn classify(conn: &Connection, path: &str) -> Class {
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        ["runs", _, "checkpoint.md"] => return class("checkpoint", path, "run"),
        ["runs", run, "manifest.json" | "writfold-run.json"] => {
            let id = indexed(conn, path).map_or_else(|| (*run).to_owned(), |(id, _)| id);
            return class("run", &id, "run");
        }
        ["runs", _, dir, ..] => {
            if let Some((_, kind)) = RUN.iter().find(|(d, _)| d == dir) {
                return class(kind, path, "run");
            }
            return class("run-file", path, "run");
        }
        [".writfold", dir, ..] => {
            let kind = CONFIG
                .iter()
                .find(|(d, _)| d == dir)
                .map_or("config", |(_, k)| k);
            return class(kind, path, "config");
        }
        _ => {}
    }
    if let Some((id, kind)) = indexed(conn, path) {
        return class(&kind, &id, "records");
    }
    match crate::layout::classify(path) {
        Some(crate::layout::Kind::Doc) | None => class("doc", path, "other"),
        Some(kind) => class(kind.as_str(), path, "records"),
    }
}
