//! `impact … --manifest FILE` (#155, RFC 2026-11-25-kb-research-contracts):
//! the walk's answer kept as a file a scope gate can retain. It names the
//! seed, direction and depth, says `coverage` `complete` or `partial` (the
//! same word, and the same exit status, as the answer), and lists every
//! record reached, sorted by id, with its path and the digest of its bytes,
//! and, when the seed is a record, its own path and digest (`seed_path`,
//! `seed_digest`, #166).
//! Only KB records are members: a node that is not one (a code locator) and
//! a run's own files (receipts and evidence, which record the work and may
//! mention the seed) are counted in `others` and never listed, so running
//! the scope gate never grows its own scope.
//!
//! With `--manifest` the walk is over records: an edge declared in a run's
//! own files (a worker's report, a receipt, an invocation record) is not
//! walked, listed or counted, so the reviews a scope leads to never make
//! the scope itself partial. "Complete" is complete for this seed,
//! direction and depth over typed links; it says nothing of records
//! related in meaning and not linked.
//! Writing the file is tested; the completeness decision is the proved
//! `walk::complete_within`.
use crate::error::Result;
use crate::index::query::{self, Resolved};
use crate::session::Session;
use serde_json::{Value, json};
use std::path::Path;

pub const SCHEMA: &str = "writfold-manifest/1";

/// One member, when `node` is a KB record outside `runs/`: its id, path and
/// the digest of its bytes (`unreadable` when they do not read).
fn member(session: &Session<'_>, node: &str) -> Result<Option<Value>> {
    let rec = match query::resolve(&session.conn, node)? {
        Resolved::One(rec) if !rec.path.starts_with("runs/") => rec,
        _ => return Ok(None),
    };
    let digest = std::fs::read(session.ws.root.join(&rec.path)).map_or_else(
        |_| "unreadable".to_owned(),
        |b| crate::deliver::candidate::sha(&b),
    );
    Ok(Some(
        json!({"id": node, "path": rec.path, "digest": digest}),
    ))
}

/// Write the manifest of a walk from `seed` that reached `nodes`.
pub fn write(
    session: &Session<'_>,
    file: &Path,
    (seed, direction, depth): (&str, &str, usize),
    nodes: &[String],
    complete: bool,
) -> Result<()> {
    let mut ids: Vec<&String> = nodes.iter().filter(|n| n.as_str() != seed).collect();
    ids.sort();
    ids.dedup();
    let found = ids
        .iter()
        .map(|id| member(session, id))
        .collect::<Result<Vec<Option<Value>>>>()?;
    let others = found.iter().filter(|m| m.is_none()).count();
    let members: Vec<Value> = found.into_iter().flatten().collect();
    let seed_file = member(session, seed)?;
    let mut value = json!({
        "schema": SCHEMA,
        "seed": seed,
        "direction": direction,
        "depth": depth,
        "coverage": if complete { "complete" } else { "partial" },
        "members": members,
        "others": others,
    });
    // The seed's own path and bytes, when it is a record (#166): a unit is
    // about the seed as well as its two members.
    if let (Some(f), Some(map)) = (seed_file, value.as_object_mut()) {
        for (from, to) in [("path", "seed_path"), ("digest", "seed_digest")] {
            if let Some(v) = f.get(from) {
                map.insert(to.into(), v.clone());
            }
        }
    }
    let mut bytes = serde_json::to_vec_pretty(&value).unwrap_or_default();
    bytes.push(b'\n');
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, bytes)?;
    Ok(())
}
