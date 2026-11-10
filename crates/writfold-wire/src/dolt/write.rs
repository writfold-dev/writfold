//! Bounded decoders for the single Dolt writer's closed probe queries. They
//! decode native observations only; nothing here grants writer authority.
use super::{Error, Result, identity, integer, native_rows, scalar, shape, strings, take_map};
use crate::{Limits, RawJson};
use serde_json::Value;
use writfold_domain::DoltIntegrationCommitId;

const HEAD_FIELDS: &str = "head branch retained pending";
const HEAD_STRINGS: &str = "head branch";

/// One native observation of the conditional write head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltHeadProbe {
    /// The commit the write head names.
    pub head: DoltIntegrationCommitId,
    /// Whether the native session's active branch is the write head.
    pub active: bool,
    /// Whether `wf-retained/<head>` exists.
    pub retained: bool,
    /// Uncommitted working-set table changes.
    pub pending: u64,
}

/// Decode the head probe; `branch` is the expected active write head.
pub fn decode_head(bytes: &[u8], limits: Limits, branch: &str) -> Result<DoltHeadProbe> {
    let mut map = scalar(bytes, limits, HEAD_FIELDS, HEAD_STRINGS)?;
    let retained = match integer(&map, "retained")? {
        0 => false,
        1 => true,
        _ => return Err(Error::InvalidField { field: "retained" }),
    };
    let pending = integer(&map, "pending")?;
    let active = map.get("branch").and_then(Value::as_str) == Some(branch);
    Ok(DoltHeadProbe {
        head: identity(&mut map, "head")?,
        active,
        retained,
        pending,
    })
}

/// The commit carrying one write identity on the head's history, if any. More
/// than one candidate refuses: a write identity names at most one commit.
pub fn decode_applied(bytes: &[u8], limits: Limits) -> Result<Option<DoltIntegrationCommitId>> {
    let mut raw = RawJson::parse(bytes, limits)?;
    if raw.value().as_object().is_some_and(|map| map.is_empty()) {
        return Ok(None);
    }
    let rows = native_rows(&raw)?;
    match rows.len() {
        0 => return Ok(None),
        1 => {}
        _ => return Err(Error::RowCount),
    }
    let row = rows.first().ok_or(Error::RowCount)?;
    strings(shape(row, "native_commit")?, "native_commit")?;
    let Some(Value::Array(rows)) = raw.value.get_mut("rows") else {
        return Err(Error::InvalidShape);
    };
    let mut map = take_map(rows.first_mut().ok_or(Error::RowCount)?)?;
    identity(&mut map, "native_commit").map(Some)
}
