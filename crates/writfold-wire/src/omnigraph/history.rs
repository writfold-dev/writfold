use super::{NativeError, Result, equal_text, natural, object, parse, take_commit, text};
use crate::Limits;
use serde_json::Value;
use std::collections::BTreeSet;
use writfold_domain::{GraphCommitId, RowLimit};

/// Check the supplied exact-main retention acknowledgment. URI text is only a
/// diagnostic: the caller must bind the actual owned graph/root/request. This
/// result does not create a Writfold retention capability or durable receipt.
pub fn decode_retention(
    status: u16,
    bytes: &[u8],
    limits: Limits,
    name: &str,
    actor: &str,
    expected: &GraphCommitId,
) -> Result<GraphCommitId> {
    if status != 200 {
        return Err(NativeError::Binding("retention status"));
    }
    let mut raw = parse(bytes, limits)?;
    let map = object(
        raw.value(),
        &["uri", "from", "name", "actor_id", "from_commit_id"],
        &[],
    )?;
    if text(map, "uri")?.is_empty() {
        return Err(NativeError::Shape("retention URI"));
    }
    equal_text(map, "from", "main")?;
    equal_text(map, "name", name)?;
    equal_text(map, "actor_id", actor)?;
    equal_text(map, "from_commit_id", expected.as_str())?;
    take_commit(&mut raw.value["from_commit_id"])
}

/// Validate a complete bounded ordinary-main first-parent history, newest first.
/// Any continuation, missing link, duplicate identity or non-published/merged
/// entry refuses this profile. The supplied head must come from the associated
/// owned read cut; list membership alone cannot reconcile an original effect.
pub fn decode_complete_history(
    status: u16,
    bytes: &[u8],
    limits: Limits,
    expected_head: &GraphCommitId,
    maximum: RowLimit,
) -> Result<Vec<GraphCommitId>> {
    if status != 200 {
        return Err(NativeError::Binding("history status"));
    }
    let mut raw = parse(bytes, limits)?;
    let map = object(
        raw.value(),
        &["commits", "head_commit_id"],
        &["next_page_token"],
    )?;
    equal_text(map, "head_commit_id", expected_head.as_str())?;
    let rows = map["commits"]
        .as_array()
        .ok_or(NativeError::Shape("commits"))?;
    if u64::try_from(rows.len()).map_err(|_| NativeError::Budget)? > maximum.get() {
        return Err(NativeError::Budget);
    }
    validate_chain(rows, expected_head.as_str())?;
    let mut commits = Vec::new();
    commits
        .try_reserve_exact(rows.len())
        .map_err(|_| NativeError::Budget)?;
    let Value::Array(rows) = std::mem::take(&mut raw.value["commits"]) else {
        return Err(NativeError::Shape("commits"));
    };
    for mut row in rows {
        commits.push(take_commit(&mut row["graph_commit_id"])?);
    }
    Ok(commits)
}

fn validate_chain(rows: &[Value], head: &str) -> Result<()> {
    if rows.is_empty() {
        return Err(NativeError::Binding("missing history head"));
    }
    let mut expected = Some(head);
    let mut identities = BTreeSet::new();
    for row in rows {
        let record = history_record(row)?;
        let id = text(record, "graph_commit_id")?;
        if expected != Some(id) || !identities.insert(id) {
            return Err(NativeError::Binding(
                "history linkage or duplicate identity",
            ));
        }
        expected = record["parent_commit_id"].as_str();
    }
    if expected.is_some() {
        return Err(NativeError::Binding("incomplete history"));
    }
    Ok(())
}

fn history_record(value: &Value) -> Result<&serde_json::Map<String, Value>> {
    let record = object(
        value,
        &[
            "graph_commit_id",
            "graph_branch",
            "graph_manifest_version",
            "parent_commit_id",
            "merged_parent_commit_id",
            "actor_id",
            "created_at",
            "published",
            "kind",
            "root_commit_id",
        ],
        &[],
    )?;
    text(record, "graph_commit_id")?;
    equal_text(record, "kind", "published")?;
    if record["published"].as_bool() != Some(true)
        || !record["graph_branch"].is_null()
        || !record["merged_parent_commit_id"].is_null()
        || !record["root_commit_id"].is_null()
        || !optional_text(&record["parent_commit_id"])
        || !optional_text(&record["actor_id"])
        || record["created_at"].as_i64().is_none()
        || natural(record, "graph_manifest_version")? == 0
    {
        return Err(NativeError::Shape("ordinary main history record"));
    }
    Ok(record)
}

fn optional_text(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(|text| !text.is_empty())
}
