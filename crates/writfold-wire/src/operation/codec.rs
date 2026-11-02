use super::*;
use crate::{OperationVariant, RawJson, canonical, canonical::canonical_prefixed, semantic::*};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum BoundaryError {
    #[error(transparent)]
    Wire(#[from] crate::Error),
    #[error(transparent)]
    Semantic(#[from] InvalidRequest),
    #[error("invalid nominal identity")]
    Identity,
    #[error("invalid exact-length hexadecimal encoding")]
    Hex,
    #[error("authority binding refused: {0}")]
    Binding(&'static str),
    #[error("invalid unencrypted Ed25519 PKCS8 PEM key")]
    PrivateKey,
    #[error("invalid Ed25519 public key")]
    PublicKey,
    #[error("invalid detached signature")]
    Signature,
    #[error("weak or non-strict signature cannot establish current authority")]
    WeakAuthority,
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    result
}
pub(crate) fn unhex<const N: usize>(text: &str) -> Result<[u8; N], BoundaryError> {
    if text.len() != N * 2 {
        return Err(BoundaryError::Hex);
    }
    let mut bytes = [0; N];
    for (out, pair) in bytes.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let digit = |b: u8| match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err(BoundaryError::Hex),
        };
        *out = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Ok(bytes)
}
pub(crate) fn encode(value: Value, max_bytes: usize) -> Result<Vec<u8>, BoundaryError> {
    Ok(canonical(&RawJson { value }, max_bytes)?)
}

pub(super) fn decode(bytes: &[u8], limits: Limits) -> Result<Operation, BoundaryError> {
    let mut raw = RawJson::parse(bytes, limits)?;
    let variant = OperationVariant::select(&raw)?;
    let v = object(raw.value())?;
    let header = OperationHeader {
        id: identity(field(v, "id")?)?,
        policy: identity(field(v, "policy")?)?,
        toolchain: identity(field(v, "toolchain")?)?,
    };
    let plan = field(v, "plan")?;
    if field(object(plan)?, "id")?.as_str() != Some(header.id.as_str()) {
        return Err(BoundaryError::Binding("inner and outer IDs differ"));
    }
    if identity_material(&mut raw, limits.bytes())? != header.id.as_str() {
        return Err(BoundaryError::Binding(
            "operation canonical identity differs",
        ));
    }
    let v = object(raw.value())?;
    let plan = field(v, "plan")?;
    let base = field(v, "base")?;
    let body = match variant {
        OperationVariant::LegacyAdmissionPlan => {
            OperationBody::LegacyAdmission(Box::new(LegacyAdmissionPlan {
                header,
                base: nullable_release(base)?,
                captures: items(field(v, "captures")?, identity)?,
                context: dec_context(field(v, "context")?)?,
                semantic: dec_plan(plan)?,
            }))
        }
        OperationVariant::LegacyReleaseSelectionPlan => {
            OperationBody::LegacySelection(Box::new(LegacyReleaseSelectionPlan {
                header,
                base: identity(base)?,
                retained: identity(field(v, "retained")?)?,
                context: dec_context(field(v, "context")?)?,
                semantic: dec_selection_plan(plan)?,
            }))
        }
        OperationVariant::AdmissionPlan => {
            OperationBody::PortfolioAdmission(PortfolioAdmissionPlan {
                header,
                base: nullable_release(base)?,
                captures: items(field(v, "captures")?, identity)?,
                target_use: text(field(v, "target_use")?)?,
                semantic: dec_portfolio_plan(plan)?,
            })
        }
        OperationVariant::ReleaseSelectionPlan => {
            OperationBody::PortfolioSelection(PortfolioReleaseSelectionPlan {
                header,
                base: identity(base)?,
                retained: identity(field(v, "retained")?)?,
                target_use: text(field(v, "target_use")?)?,
                semantic: dec_portfolio_selection_plan(plan)?,
            })
        }
    };
    let prefix = variant.domain().prefix();
    Ok(Operation {
        body,
        message: canonical_prefixed(&raw, prefix, limits.bytes().saturating_add(prefix.len()))?,
    })
}

fn nullable_release(value: &Value) -> Result<Option<ReleaseId>, BoundaryError> {
    if value.is_null() {
        Ok(None)
    } else {
        Ok(Some(identity(value)?))
    }
}

pub(super) fn identity_material(
    raw: &mut RawJson,
    max_bytes: usize,
) -> Result<String, BoundaryError> {
    // Temporarily detach only the two identity fields. Cloning Value here would
    // recurse through untrusted input before semantic depth admission.
    let outer = raw.value.as_object_mut().ok_or(InvalidRequest)?;
    let plan = outer
        .get_mut("plan")
        .and_then(Value::as_object_mut)
        .ok_or(InvalidRequest)?;
    let plan_id = plan.remove("id");
    let outer_id = outer.remove("id");
    let encoded = canonical(raw, max_bytes);
    let outer = raw.value.as_object_mut().ok_or(InvalidRequest)?;
    if let Some(id) = outer_id {
        outer.insert("id".into(), id);
    }
    if let Some(id) = plan_id {
        outer
            .get_mut("plan")
            .and_then(Value::as_object_mut)
            .ok_or(InvalidRequest)?
            .insert("id".into(), id);
    }
    Ok(hash(&encoded?))
}
