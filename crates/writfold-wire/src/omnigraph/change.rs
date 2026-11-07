use super::{NativeError, Result, equal_text, natural, object, parse, take_commit, text};
use crate::Limits;
use serde_json::Value;
use writfold_domain::GraphCommitId;

/// Expected inputs of an ordinary conditional mutation on the main branch.
/// These are supplied bindings, not live policy or native request authority.
#[derive(Clone, Copy, Debug)]
pub struct MutationBinding<'a> {
    pub query: &'a str,
    pub actor: &'a str,
    pub predecessor: &'a GraphCommitId,
}

#[derive(Debug)]
pub struct ConditionalChange {
    commit: GraphCommitId,
    affected_nodes: u64,
    affected_edges: u64,
    manifest_version: u64,
    created_at_microseconds: i64,
}

impl ConditionalChange {
    pub fn commit(&self) -> &GraphCommitId {
        &self.commit
    }
    pub fn affected_nodes(&self) -> u64 {
        self.affected_nodes
    }
    pub fn affected_edges(&self) -> u64 {
        self.affected_edges
    }
    pub fn manifest_version(&self) -> u64 {
        self.manifest_version
    }
    pub fn created_at_microseconds(&self) -> i64 {
        self.created_at_microseconds
    }
}

/// Checks a supplied successful HTTP/0.14 ordinary-main mutation payload. The
/// caller must additionally bind the exact owned native exchange, its original
/// request and the query-specific expected affected entities.
/// Any failed decode after issuance preserves uncertainty; it is not rejection.
pub fn decode_change(
    status: u16,
    bytes: &[u8],
    limits: Limits,
    binding: MutationBinding<'_>,
) -> Result<ConditionalChange> {
    if status != 200 {
        return Err(NativeError::Binding("mutation status"));
    }
    let mut raw = parse(bytes, limits)?;
    let map = object(
        raw.value(),
        &[
            "branch",
            "query_name",
            "affected_nodes",
            "affected_edges",
            "actor_id",
            "commit",
        ],
        &["outcome", "executed"],
    )?;
    equal_text(map, "branch", "main")?;
    equal_text(map, "query_name", binding.query)?;
    equal_text(map, "actor_id", binding.actor)?;
    let affected_nodes = natural(map, "affected_nodes")?;
    let affected_edges = natural(map, "affected_edges")?;
    let commit = object(
        &map["commit"],
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
    equal_text(commit, "parent_commit_id", binding.predecessor.as_str())?;
    equal_text(commit, "actor_id", binding.actor)?;
    equal_text(commit, "kind", "published")?;
    if commit["published"].as_bool() != Some(true)
        || !commit["graph_branch"].is_null()
        || !commit["merged_parent_commit_id"].is_null()
        || !commit["root_commit_id"].is_null()
        || text(commit, "graph_commit_id")? == binding.predecessor.as_str()
    {
        return Err(NativeError::Binding("ordinary published successor"));
    }
    let manifest_version = natural(commit, "graph_manifest_version")?;
    if manifest_version == 0 {
        return Err(NativeError::Shape("manifest version"));
    }
    let created_at_microseconds = commit["created_at"]
        .as_i64()
        .ok_or(NativeError::Shape("commit time"))?;
    let commit = take_commit(&mut raw.value["commit"]["graph_commit_id"])?;
    Ok(ConditionalChange {
        commit,
        affected_nodes,
        affected_edges,
        manifest_version,
        created_at_microseconds,
    })
}

/// A supplied exact precondition-failure payload. This cannot establish no-effect
/// provenance on its own and must not cause an automatic retry on the new head.
#[derive(Debug)]
pub struct ConditionalRefusal {
    actual: Option<GraphCommitId>,
}

impl ConditionalRefusal {
    pub fn actual(&self) -> Option<&GraphCommitId> {
        self.actual.as_ref()
    }
}

/// Accept only the exercised 412/precondition_failed/none/refresh combination,
/// bound to the original exact conditional header. Other errors and unknown
/// dispositions are deliberately not converted into this refusal class.
pub fn decode_precondition_refusal(
    bytes: &[u8],
    limits: Limits,
    status: u16,
    expected: &GraphCommitId,
) -> Result<ConditionalRefusal> {
    if status != 412 {
        return Err(NativeError::Binding("precondition status"));
    }
    let mut raw = parse(bytes, limits)?;
    let map = object(
        raw.value(),
        &["error", "code", "disposition", "precondition_failure"],
        &[],
    )?;
    text(map, "error")?;
    equal_text(map, "code", "precondition_failed")?;
    let disposition = object(&map["disposition"], &["effects", "action"], &[])?;
    equal_text(disposition, "effects", "none")?;
    equal_text(disposition, "action", "refresh")?;
    let fields: &[&str] = if map["precondition_failure"].get("actual").is_some() {
        &["expected", "actual"]
    } else {
        &["expected"]
    };
    let precondition = object(&map["precondition_failure"], fields, &[])?;
    equal_text(precondition, "expected", expected.as_str())?;
    let actual = match precondition.get("actual") {
        None | Some(Value::Null) => None,
        Some(Value::String(actual)) if actual != expected.as_str() => Some(take_commit(
            &mut raw.value["precondition_failure"]["actual"],
        )?),
        _ => return Err(NativeError::Binding("actual precondition")),
    };
    Ok(ConditionalRefusal { actual })
}
