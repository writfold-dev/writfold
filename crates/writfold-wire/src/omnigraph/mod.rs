//! Exact supplied Omnigraph HTTP/0.14 payloads, not authenticated observations.
//!
//! Callers must bind these bytes to the owned native process, request, HTTP
//! contract, graph, immutable query/schema profile and retained closure. A decoded
//! native commit is not completed Writfold publication or effect authority.

mod change;
mod history;
mod rows;

pub use change::{
    ConditionalChange, ConditionalRefusal, MutationBinding, decode_change,
    decode_precondition_refusal,
};
pub use history::{decode_complete_history, decode_retention};
pub use rows::{Cell, Column, Projection, QueryRows, QueryTarget, ScalarKind, decode_query};

use crate::{Limits, RawJson};
use serde_json::{Map, Value};
use thiserror::Error;
use writfold_domain::GraphCommitId;

#[derive(Debug, Error)]
pub enum NativeError {
    #[error(transparent)]
    Json(#[from] crate::Error),
    #[error("invalid native response shape: {0}")]
    Shape(&'static str),
    #[error("native response binding mismatch: {0}")]
    Binding(&'static str),
    #[error("invalid native graph commit identity")]
    CommitIdentity,
    #[error("native response row or allocation budget exceeded")]
    Budget,
}

type Result<T, E = NativeError> = std::result::Result<T, E>;

fn object<'a>(
    value: &'a Value,
    required: &[&str],
    nullable: &[&str],
) -> Result<&'a Map<String, Value>> {
    let map = value.as_object().ok_or(NativeError::Shape("object"))?;
    if required.iter().any(|key| !map.contains_key(*key))
        || map
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !nullable.contains(&key.as_str()))
        || nullable
            .iter()
            .any(|key| map.get(*key).is_some_and(|value| !value.is_null()))
    {
        return Err(NativeError::Shape("fields"));
    }
    Ok(map)
}

fn text<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a str> {
    map.get(key)
        .and_then(Value::as_str)
        .ok_or(NativeError::Shape("string"))
}

fn natural(map: &Map<String, Value>, key: &str) -> Result<u64> {
    map.get(key)
        .and_then(Value::as_u64)
        .ok_or(NativeError::Shape("nonnegative integer"))
}

fn equal_text(map: &Map<String, Value>, key: &str, expected: &str) -> Result<()> {
    if text(map, key)? == expected {
        Ok(())
    } else {
        Err(NativeError::Binding("text"))
    }
}

fn parse(bytes: &[u8], limits: Limits) -> Result<RawJson> {
    Ok(RawJson::parse(bytes, limits)?)
}

fn take_string(value: &mut Value) -> Result<String> {
    if !value.is_string() {
        return Err(NativeError::Shape("string"));
    }
    match std::mem::take(value) {
        Value::String(value) => Ok(value),
        _ => Err(NativeError::Shape("string")),
    }
}

fn take_commit(value: &mut Value) -> Result<GraphCommitId> {
    GraphCommitId::try_from(take_string(value)?).map_err(|_| NativeError::CommitIdentity)
}
