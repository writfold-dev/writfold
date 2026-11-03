use super::*;
use crate::{operation, storage::encode_pin};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Append already-canonical children without cloning or recursively re-encoding
/// the workflow operation. The fixed-width ID slot is filled after hashing the
/// two slices that omit its entire field, so only one envelope buffer is needed.
pub(super) fn message(
    operation: &Operation,
    base: Option<&StorageBase>,
    target: &StorageTarget,
    mapping: Option<&PlanDigest>,
    max_bytes: usize,
) -> Result<(PlanDigest, Vec<u8>)> {
    let mut writer = Writer {
        bytes: Vec::new(),
        max_bytes,
    };
    writer.append(STORAGE_OPERATION_DOMAIN)?;
    writer.append(b"{\"base\":")?;
    writer.base(base)?;
    let before_id = writer.bytes.len();
    writer.append(b",\"id\":\"")?;
    let id_start = writer.bytes.len();
    writer.append(b"0000000000000000000000000000000000000000000000000000000000000000\"")?;
    let after_id = writer.bytes.len();
    writer.append(b",\"mapping\":")?;
    writer.value(json!(mapping.map(PlanDigest::as_str)))?;
    writer.append(b",\"namespace\":")?;
    writer.value(Value::from(STORAGE_OPERATION_NAMESPACE))?;
    writer.append(b",\"operation\":")?;
    writer.append(operation.canonical_bytes())?;
    writer.append(b",\"schema\":")?;
    writer.value(Value::from(STORAGE_OPERATION_SCHEMA))?;
    writer.append(b",\"target\":")?;
    writer.value(target_value(target))?;
    writer.append(b"}")?;
    let mut hash = Sha256::new();
    hash.update(STORAGE_PLAN_DOMAIN);
    hash.update(&writer.bytes[STORAGE_OPERATION_DOMAIN.len()..before_id]);
    hash.update(&writer.bytes[after_id..]);
    let digest = operation::hex(&hash.finalize());
    writer.bytes[id_start..id_start + digest.len()].copy_from_slice(digest.as_bytes());
    let id = PlanDigest::try_from(digest).map_err(|_| StorageOperationError::Identity)?;
    Ok((id, writer.bytes))
}

fn target_value(target: &StorageTarget) -> Value {
    json!({
        "backend": STORAGE_BACKEND,
        "kb": target.kb().as_str(),
        "graph": target.graph().as_str(),
        "epoch": target.epoch().id().as_str(),
        "schema_digest": target.epoch().schema().as_str(),
        "query_digest": target.epoch().query().as_str(),
        "storage_profile": target.profile().as_str(),
        "native_format": target.native_format().get(),
        "producer_generation": target.producer().as_str(),
    })
}

struct Writer {
    bytes: Vec<u8>,
    max_bytes: usize,
}
impl Writer {
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > self.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(Error::Limit {
                budget: Budget::Bytes,
                offset: self.bytes.len(),
            }
            .into());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn remaining(&self) -> usize {
        self.max_bytes.saturating_sub(self.bytes.len())
    }
    fn value(&mut self, value: Value) -> Result<()> {
        self.append(&operation::encode(value, self.remaining())?)
    }
    fn base(&mut self, base: Option<&StorageBase>) -> Result<()> {
        let Some(base) = base else {
            return self.append(b"null");
        };
        self.append(b"{\"pin\":")?;
        self.append(&encode_pin(base.pin(), self.remaining())?)?;
        self.append(b",\"release\":")?;
        self.value(Value::from(base.release().as_str()))?;
        self.append(b"}")
    }
}
