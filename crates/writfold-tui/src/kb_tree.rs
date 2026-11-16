//! The walk as a tree, read from each `impact` row's structured `edge`: a
//! depth-1 row hangs under the root, a depth-2 row under the entry its edge
//! names. A row without `edge`, or whose parent is not in the answer, is
//! drawn flat under a `depth N` header. Every row appears once; no text is
//! split.
use crate::kb_entry::{Entry, from_row};
use crate::kb_source::TRACKER;
use crate::source::{Reply, field};
use serde_json::Value;

struct Walked<'a> {
    row: &'a Value,
    id: &'a str,
    /// The edge's other end: where the walk came from.
    parent: Option<&'a str>,
    depth: Option<u64>,
}

fn walked(row: &Value) -> Walked<'_> {
    let id = field(row, "id");
    let edge = row.get("edge");
    let end = |key: &str| edge.and_then(|e| e.get(key)).and_then(Value::as_str);
    let parent = match (end("from"), end("to")) {
        (Some(from), Some(to)) if from == id => Some(to),
        (Some(from), Some(to)) if to == id => Some(from),
        _ => None,
    };
    Walked {
        row,
        id,
        parent,
        depth: edge.and_then(|e| e.get("depth")).and_then(Value::as_u64),
    }
}

fn entry(walked: &Walked<'_>, depth: usize) -> Entry {
    let row = walked.row;
    let mut entry = from_row(row, field(row, "title"));
    let edge = |key: &str| {
        row.get("edge")
            .and_then(|e| e.get(key))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    entry.word = edge("rel");
    entry.origin = edge("origin");
    entry.from = edge("from") == walked.id;
    entry.depth = depth;
    entry
}

/// Rows that are not placed under a parent, flat under `depth N` headers
/// (`depth ?` for a row without an edge).
fn flat(rows: &[Walked<'_>], placed: &[bool], keep: &dyn Fn(&Walked<'_>) -> bool) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut depths: Vec<Option<u64>> = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        if !placed[at] && keep(row) && !depths.contains(&row.depth) {
            depths.push(row.depth);
        }
    }
    for depth in depths {
        let name = depth.map_or_else(|| "?".to_owned(), |d| d.to_string());
        out.push(Entry::header(format!("depth {name}")));
        for (at, row) in rows.iter().enumerate() {
            if !placed[at] && keep(row) && row.depth == depth {
                out.push(entry(row, 0));
            }
        }
    }
    out
}

/// The unfiltered tree: the root, its depth-1 entries in verb order, each
/// followed by its depth-2 children in verb order.
pub fn tree(reply: &Reply, root: &str) -> Vec<Entry> {
    let rows: Vec<Walked<'_>> = reply.rows.iter().map(walked).collect();
    let mut placed = vec![false; rows.len()];
    let mut out = vec![Entry::header(root)];
    for (at, row) in rows.iter().enumerate() {
        if row.depth != Some(1) || row.parent != Some(root) {
            continue;
        }
        placed[at] = true;
        out.push(entry(row, 0));
        for (child, below) in rows.iter().enumerate() {
            if !placed[child] && below.depth == Some(2) && below.parent == Some(row.id) {
                placed[child] = true;
                out.push(entry(below, 1));
            }
        }
    }
    out.extend(flat(&rows, &placed, &|_| true));
    out
}

/// The tracker: tickets, investigations and signatures within two links,
/// grouped under the record their edge came from.
pub fn tracker(reply: &Reply, root: &str) -> Vec<Entry> {
    let rows: Vec<Walked<'_>> = reply.rows.iter().map(walked).collect();
    let kept = |row: &Walked<'_>| TRACKER.contains(&field(row.row, "kind"));
    let mut placed = vec![false; rows.len()];
    let mut out = Vec::new();
    let mut parents: Vec<&str> = vec![root];
    parents.extend(
        rows.iter()
            .filter(|r| r.depth == Some(1) && r.parent == Some(root))
            .map(|r| r.id),
    );
    for (level, parent) in parents.into_iter().enumerate() {
        let depth = if level == 0 { 1 } else { 2 };
        let mut group = Vec::new();
        for (at, row) in rows.iter().enumerate() {
            if !placed[at] && kept(row) && row.depth == Some(depth) && row.parent == Some(parent) {
                placed[at] = true;
                group.push(entry(row, 0));
            }
        }
        if !group.is_empty() {
            out.push(Entry::header(parent));
            out.extend(group);
        }
    }
    out.extend(flat(&rows, &placed, &kept));
    out
}

/// The walk's trailer in words: reached, what was not followed and the
/// frontier; `true` when the verb says its coverage is partial.
pub fn trailer(reply: &Reply) -> (String, bool) {
    let shown = reply
        .trailer
        .get("shown")
        .and_then(Value::as_u64)
        .unwrap_or(reply.rows.len() as u64);
    let mut text = format!("{shown} reached");
    let skipped: Vec<String> = ["retired", "disputed", "inferred", "other"]
        .into_iter()
        .filter_map(|reason| {
            reply
                .count(&format!("skipped_{reason}"))
                .filter(|n| *n > 0)
                .map(|n| format!("{n} {reason}"))
        })
        .collect();
    if !skipped.is_empty() {
        text.push_str(&format!("   not followed: {}", skipped.join(", ")));
    }
    if let Some(frontier) = reply.count("frontier").filter(|n| *n > 0) {
        text.push_str(&format!("   frontier {frontier}"));
    }
    let partial = reply.trailer_text("coverage").as_deref() == Some("partial");
    (text, partial)
}

#[cfg(test)]
#[path = "kb_tree_tests.rs"]
mod tests;
