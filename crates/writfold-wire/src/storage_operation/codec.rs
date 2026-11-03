use super::*;
use crate::{
    canonical,
    semantic::{InvalidRequest, field, identity, obj},
    storage::decode_pin,
};
use serde_json::Value;
use writfold_domain::{NativeFormatVersion, SchemaEpoch};

pub(super) fn decode(bytes: &[u8], limits: Limits) -> Result<StorageOperation> {
    let mut raw = RawJson::parse(bytes, envelope_limits(limits)?)?;
    let v = obj(
        raw.value(),
        "namespace schema id operation base target mapping",
    )?;
    if field(v, "namespace")?.as_str() != Some(STORAGE_OPERATION_NAMESPACE) {
        return Err(StorageOperationError::UnsupportedNamespace);
    }
    if unsigned(field(v, "schema")?)? != STORAGE_OPERATION_SCHEMA {
        return Err(StorageOperationError::UnsupportedVersion);
    }
    let supplied_id: PlanDigest = identity(field(v, "id")?)?;
    let target = target(field(v, "target")?)?;
    let mapping = match field(v, "mapping")? {
        Value::Null => None,
        value => Some(identity(value)?),
    };
    let base = base(detach(&mut raw, "base")?, limits)?;
    let operation_raw = detach(&mut raw, "operation")?;
    let operation = Operation::decode(&canonical(&operation_raw, limits.bytes())?, limits)?;
    binding::check(&operation, base.as_ref(), &target, mapping.as_ref())?;
    let (id, message) = encode::message(
        &operation,
        base.as_ref(),
        &target,
        mapping.as_ref(),
        limits.bytes(),
    )?;
    if id != supplied_id {
        return Err(StorageOperationError::Identity);
    }
    Ok(StorageOperation {
        id,
        operation,
        base,
        target,
        mapping,
        message,
    })
}

fn target(value: &Value) -> Result<StorageTarget> {
    let v = obj(
        value,
        "backend kb graph epoch schema_digest query_digest storage_profile native_format producer_generation",
    )?;
    if field(v, "backend")?.as_str() != Some(STORAGE_BACKEND) {
        return Err(StorageOperationError::UnsupportedBackend);
    }
    let epoch = SchemaEpoch::new(
        identity(field(v, "epoch")?)?,
        identity(field(v, "schema_digest")?)?,
        identity(field(v, "query_digest")?)?,
    );
    let native_format = NativeFormatVersion::new(unsigned(field(v, "native_format")?)?)
        .map_err(|_| InvalidRequest)?;
    Ok(StorageTarget::new(
        identity(field(v, "kb")?)?,
        identity(field(v, "graph")?)?,
        epoch,
        identity(field(v, "storage_profile")?)?,
        native_format,
        identity(field(v, "producer_generation")?)?,
    ))
}

fn base(mut raw: RawJson, limits: Limits) -> Result<Option<StorageBase>> {
    if raw.value().is_null() {
        return Ok(None);
    }
    let v = obj(raw.value(), "release pin")?;
    let release = identity(field(v, "release")?)?;
    let pin_raw = detach(&mut raw, "pin")?;
    let pin = decode_pin(&canonical(&pin_raw, limits.bytes())?, limits)?;
    Ok(Some(StorageBase::new(release, pin)))
}

fn unsigned(value: &Value) -> Result<u64> {
    let number = value.as_number().ok_or(InvalidRequest)?;
    if number.as_str().contains(['.', 'e', 'E']) {
        return Err(InvalidRequest.into());
    }
    number.as_u64().ok_or(InvalidRequest.into())
}

/// Move each untrusted subtree directly into the iterative-drop owner, including
/// error paths. Never clone Value or hand it to serde's recursive serializer.
fn detach(raw: &mut RawJson, key: &str) -> Result<RawJson> {
    let value = raw
        .value
        .as_object_mut()
        .ok_or(InvalidRequest)?
        .remove(key)
        .ok_or(InvalidRequest)?;
    Ok(RawJson { value })
}
