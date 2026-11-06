use super::{
    Error, Result, boolean, identity, native_rows, shape, strings, take_map, take_text, text,
};
use crate::{Limits, RawJson};
use serde_json::Value;
use writfold_domain::{
    RowLimit,
    dolt::{GraphEdge, GraphNode},
};

const NODE_FIELDS: &str = "id kind label home_path home_is_null";
const NODE_STRINGS: &str = "id kind label home_path";
const EDGE_FIELDS: &str = "id source target kind";

/// Validates the whole shallow tree while RawJson owns it, including rows that
/// will not be converted if an earlier nominal identity is rejected. Thus every
/// malformed deep tree still receives RawJson's iterative error cleanup.
fn collection(bytes: &[u8], limits: Limits, max_rows: RowLimit, nodes: bool) -> Result<Vec<Value>> {
    let mut raw = RawJson::parse(bytes, limits)?;
    if raw.value().as_object().is_some_and(|map| map.is_empty()) {
        return Ok(Vec::new());
    }
    let rows = native_rows(&raw)?;
    if u64::try_from(rows.len()).map_err(|_| Error::RowLimit)? > max_rows.get() {
        return Err(Error::RowLimit);
    }
    let mut previous: Option<&str> = None;
    for (row, value) in rows.iter().enumerate() {
        let map = shape(value, if nodes { NODE_FIELDS } else { EDGE_FIELDS })?;
        strings(map, if nodes { NODE_STRINGS } else { EDGE_FIELDS })?;
        if nodes && boolean(map, "home_is_null")? && !text(map, "home_path")?.is_empty() {
            return Err(Error::NullPayload);
        }
        let id = text(map, "id")?;
        if previous.is_some_and(|prior| prior.as_bytes() >= id.as_bytes()) {
            return Err(Error::RowOrder { row });
        }
        previous = Some(id);
    }
    let Some(Value::Array(rows)) = raw.value.get_mut("rows").map(std::mem::take) else {
        return Err(Error::InvalidShape);
    };
    Ok(rows)
}

pub fn decode_nodes(bytes: &[u8], limits: Limits, max_rows: RowLimit) -> Result<Vec<GraphNode>> {
    collection(bytes, limits, max_rows, true)?
        .into_iter()
        .map(|mut row| {
            let mut map = take_map(&mut row)?;
            let id = identity(&mut map, "id")?;
            let kind = identity(&mut map, "kind")?;
            let label = take_text(&mut map, "label")?;
            let is_null = boolean(&map, "home_is_null")?;
            let home = take_text(&mut map, "home_path")?;
            Ok(GraphNode::new(
                id,
                kind,
                label,
                if is_null { None } else { Some(home) },
            ))
        })
        .collect()
}

pub fn decode_edges(bytes: &[u8], limits: Limits, max_rows: RowLimit) -> Result<Vec<GraphEdge>> {
    collection(bytes, limits, max_rows, false)?
        .into_iter()
        .map(|mut row| {
            let mut map = take_map(&mut row)?;
            Ok(GraphEdge::new(
                identity(&mut map, "id")?,
                identity(&mut map, "source")?,
                identity(&mut map, "target")?,
                identity(&mut map, "kind")?,
            ))
        })
        .collect()
}
