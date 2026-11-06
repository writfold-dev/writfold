use super::*;
use crate::{
    canonical,
    dolt::decode_pin,
    semantic::{InvalidRequest, field, identity, obj},
};
use serde_json::Value;
use writfold_domain::SchemaEpoch;

pub(super) fn decode(bytes: &[u8], limits: Limits) -> Result<DoltOperation> {
    let mut raw = RawJson::parse(bytes, envelope_limits(limits)?)?;
    let v = obj(
        raw.value(),
        "namespace schema id operation base target mapping",
    )?;
    if field(v, "namespace")?.as_str() != Some(DOLT_OPERATION_NAMESPACE) {
        return Err(DoltOperationError::UnsupportedNamespace);
    }
    if unsigned(field(v, "schema")?)? != DOLT_OPERATION_SCHEMA {
        return Err(DoltOperationError::UnsupportedVersion);
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
        return Err(DoltOperationError::Identity);
    }
    Ok(DoltOperation {
        id,
        operation,
        base,
        target,
        mapping,
        message,
    })
}

fn target(value: &Value) -> Result<DoltTarget> {
    let v = obj(
        value,
        "backend realm kb database epoch schema_digest query_digest storage_profile producer",
    )?;
    if field(v, "backend")?.as_str() != Some(DOLT_BACKEND) {
        return Err(DoltOperationError::UnsupportedBackend);
    }
    let epoch = SchemaEpoch::new(
        identity(field(v, "epoch")?)?,
        identity(field(v, "schema_digest")?)?,
        identity(field(v, "query_digest")?)?,
    );
    Ok(DoltTarget::new(
        identity(field(v, "realm")?)?,
        identity(field(v, "kb")?)?,
        identity(field(v, "database")?)?,
        epoch,
        identity(field(v, "storage_profile")?)?,
        identity(field(v, "producer")?)?,
    ))
}

fn base(mut raw: RawJson, limits: Limits) -> Result<Option<DoltBase>> {
    if raw.value().is_null() {
        return Ok(None);
    }
    let v = obj(raw.value(), "release pin")?;
    let release = identity(field(v, "release")?)?;
    let pin_raw = detach(&mut raw, "pin")?;
    let pin = decode_pin(&canonical(&pin_raw, limits.bytes())?, limits)?;
    Ok(Some(DoltBase::new(release, pin)))
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
