//! The JSON form of one page of an answer ([`crate::emit`]): the
//! envelope, its rows, and `diagnostics` when the verb read definition
//! files.
use crate::emit::{Answer, Frame, Row};
use serde_json::{Value, json};

/// The JSON envelope of one page: `rows` are the rows on it.
pub fn body(answer: &Answer, frame: &Frame, trailer: Value, rows: &[Row]) -> String {
    let rows: Vec<Value> = rows.iter().map(row_json).collect();
    let mut body = json!({"writfold_kb": 1, "verb": frame.verb, "rows": rows,
                      "notes": answer.notes, "trailer": trailer});
    if let Some(report) = &answer.report {
        body["diagnostics"] = crate::defs::json(report);
    }
    format!("{body}\n")
}

fn row_json(row: &Row) -> Value {
    let more: Vec<Value> = row
        .more
        .iter()
        .map(|m| {
            let mut value = json!({"label": m.label, "text": m.text, "path": m.path, "line": m.line, "flags": m.flags});
            if let Some(status) = &m.status {
                value["status"] = json!(status);
            }
            value
        })
        .collect();
    let mut value = json!({"kind": row.kind, "id": row.id, "status": row.status, "text": row.text,
           "path": row.path, "line": row.line, "flags": row.flags, "more": more,
           "section": row.section});
    if let Some(edge) = &row.edge {
        value["edge"] = json!({"from": edge.from, "rel": edge.rel, "to": edge.to,
                               "origin": edge.origin, "depth": edge.depth});
    }
    if let Some(title) = &row.title {
        value["title"] = json!(title);
    }
    if let Some(tier) = row.matched_on {
        value["matched_on"] = json!(tier);
    }
    #[cfg(feature = "semantic")]
    if let Some(hit) = &row.semantic {
        value["semantic"] = json!({"score": hit.score, "path": hit.path,
                                  "first": hit.first, "last": hit.last});
    }
    value
}
