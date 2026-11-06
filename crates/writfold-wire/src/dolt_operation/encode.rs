use super::*;
use crate::{canonical::Writer, dolt::encode_pin, operation};
use sha2::{Digest, Sha256};

/// Borrow canonical children and fill the fixed-width ID slot after hashing the
/// two slices excluding its field. The signed envelope owns a single buffer.
pub(super) fn message(
    operation: &Operation,
    base: Option<&DoltBase>,
    target: &DoltTarget,
    mapping: Option<&PlanDigest>,
    max_bytes: usize,
) -> Result<(PlanDigest, Vec<u8>)> {
    let mut writer = Writer {
        bytes: Vec::new(),
        max_bytes,
    };
    writer.append(DOLT_OPERATION_DOMAIN)?;
    writer.append(b"{\"base\":")?;
    if let Some(base) = base {
        writer.append(b"{\"pin\":")?;
        writer.append(&encode_pin(
            base.pin(),
            max_bytes.saturating_sub(writer.bytes.len()),
        )?)?;
        writer.append(b",\"release\":")?;
        writer.string(base.release().as_str())?;
        writer.append(b"}")?;
    } else {
        writer.append(b"null")?;
    }
    let before_id = writer.bytes.len();
    writer.append(b",\"id\":\"")?;
    let id_start = writer.bytes.len();
    writer.append(b"0000000000000000000000000000000000000000000000000000000000000000\"")?;
    let after_id = writer.bytes.len();
    writer.append(b",\"mapping\":")?;
    if let Some(mapping) = mapping {
        writer.string(mapping.as_str())?;
    } else {
        writer.append(b"null")?;
    }
    writer.append(b",\"namespace\":")?;
    writer.string(DOLT_OPERATION_NAMESPACE)?;
    writer.append(b",\"operation\":")?;
    writer.append(operation.canonical_bytes())?;
    writer.append(b",\"schema\":1,\"target\":{")?;
    for (index, (key, value)) in [
        ("backend", DOLT_BACKEND),
        ("database", target.database().as_str()),
        ("epoch", target.epoch().id().as_str()),
        ("kb", target.kb().as_str()),
        ("producer", target.producer().as_str()),
        ("query_digest", target.epoch().query().as_str()),
        ("realm", target.realm().as_str()),
        ("schema_digest", target.epoch().schema().as_str()),
        ("storage_profile", target.profile().as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        if index != 0 {
            writer.append(b",")?;
        }
        writer.string(key)?;
        writer.append(b":")?;
        writer.string(value)?;
    }
    writer.append(b"}}")?;
    let mut hash = Sha256::new();
    hash.update(DOLT_PLAN_DOMAIN);
    hash.update(&writer.bytes[DOLT_OPERATION_DOMAIN.len()..before_id]);
    hash.update(&writer.bytes[after_id..]);
    let digest = operation::hex(&hash.finalize());
    writer.bytes[id_start..id_start + digest.len()].copy_from_slice(digest.as_bytes());
    let id = PlanDigest::try_from(digest).map_err(|_| DoltOperationError::Identity)?;
    Ok((id, writer.bytes))
}
