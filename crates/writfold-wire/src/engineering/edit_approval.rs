//! Closed canonical bytes for prepared-edit approvals and their signed envelope.
//!
//! Decoding establishes structure only. Signature, policy, expiry and current
//! resource checks belong to `signatures::edit`.
use super::cbor::{self, Decoder, Value as C};
use crate::Limits;
use writfold_domain::engineering::approval::{EditApproval, Signer, Validity};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest, ProcedureRevision};
use writfold_domain::worktree::{PreparedEditDigest, Scope, WorktreeDigest};
use writfold_domain::{EpochSeconds, ResourceGeneration};

pub const PAYLOAD_NAMESPACE: &str = "writfold-prepared-edit-approval";
pub const ENVELOPE_NAMESPACE: &str = "writfold-signed-edit-approval";
pub const SCHEMA_VERSION: u64 = 1;
/// Ed25519 signs `SIGNING_DOMAIN || payload bytes`; no other domain shares it.
pub const SIGNING_DOMAIN: &[u8] = b"writfold-prepared-edit-approval/1\0";
const PAYLOAD_FIELDS: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Decode(#[from] cbor::DecodeError),
    #[error(transparent)]
    Encode(#[from] cbor::Error),
    #[error(transparent)]
    Identity(#[from] writfold_domain::IdentityError),
    #[error(transparent)]
    Quantity(#[from] writfold_domain::QuantityError),
    #[error(transparent)]
    Worktree(#[from] writfold_domain::worktree::InvalidWorktree),
    #[error(transparent)]
    Approval(#[from] writfold_domain::engineering::approval::InvalidApproval),
    #[error("invalid prepared-edit approval field: {0}")]
    Field(&'static str),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Encode the closed payload array in its fixed field order.
pub fn encode_payload(approval: &EditApproval, limits: Limits) -> Result<Vec<u8>> {
    let scope = approval.scope();
    let validity = approval.validity();
    let prepared = approval.prepared();
    let report = approval.report();
    let procedure = approval.procedure();
    let fields = [
        C::Text(PAYLOAD_NAMESPACE),
        C::Unsigned(SCHEMA_VERSION),
        C::Text(scope.realm().as_str()),
        C::Text(scope.run().as_str()),
        C::Text(scope.repo().as_str()),
        C::Text(scope.worktree().as_str()),
        C::Text(scope.binding().as_str()),
        C::Bytes(prepared.as_digest()),
        C::Bytes(report.as_digest()),
        C::Bytes(procedure.as_digest()),
        C::Text(approval.resource().as_str()),
        C::Unsigned(approval.generation().get()),
        C::Unsigned(validity.valid_from().get()),
        C::Unsigned(validity.expires().get()),
        C::Text(approval.signer().key().as_str()),
        C::Text(approval.signer().policy().as_str()),
    ];
    Ok(cbor::encode(&C::Array(&fields), limits)?.into_bytes())
}

/// Require the exact namespace, version, field count, canonical form and full
/// input consumption before returning a structurally typed approval.
pub fn decode_payload(bytes: &[u8], limits: Limits) -> Result<EditApproval> {
    let mut d = Decoder::new(bytes, limits)?;
    if d.array()? != PAYLOAD_FIELDS {
        return Err(Error::Field("payload field count"));
    }
    expect_text(&mut d, PAYLOAD_NAMESPACE, "payload namespace")?;
    if d.unsigned()? != SCHEMA_VERSION {
        return Err(Error::Field("schema version"));
    }
    let scope = Scope::new(
        d.text()?.parse()?,
        d.text()?.parse()?,
        d.text()?.parse()?,
        d.text()?.parse()?,
        d.text()?.parse()?,
    )?;
    let prepared = PreparedEditDigest::from_digest(digest(&mut d)?);
    let report = BlobDigest::from_digest(digest(&mut d)?);
    let procedure = ProcedureRevision::from_digest(digest(&mut d)?);
    let resource = d.text()?.parse()?;
    let generation = ResourceGeneration::new(d.unsigned()?)?;
    let validity = Validity::new(
        EpochSeconds::new(d.unsigned()?)?,
        EpochSeconds::new(d.unsigned()?)?,
    )?;
    let signer = Signer::new(d.text()?.parse()?, d.text()?.parse()?);
    d.end_array()?;
    d.finish()?;
    Ok(EditApproval::new(
        scope, prepared, report, procedure, resource, generation, validity, signer,
    ))
}

/// Signed envelope `[namespace, version, payload bytes, signature]`.
pub fn encode_envelope(payload: &[u8], signature: &[u8; 64], limits: Limits) -> Result<Vec<u8>> {
    let fields = [
        C::Text(ENVELOPE_NAMESPACE),
        C::Unsigned(SCHEMA_VERSION),
        C::Bytes(payload),
        C::Bytes(signature),
    ];
    Ok(cbor::encode(&C::Array(&fields), limits)?.into_bytes())
}

/// Returns the exact payload bytes (borrowed from the input) and signature.
pub fn decode_envelope(bytes: &[u8], limits: Limits) -> Result<(&[u8], [u8; 64])> {
    let mut d = Decoder::new(bytes, limits)?;
    if d.array()? != 4 {
        return Err(Error::Field("envelope field count"));
    }
    expect_text(&mut d, ENVELOPE_NAMESPACE, "envelope namespace")?;
    if d.unsigned()? != SCHEMA_VERSION {
        return Err(Error::Field("envelope version"));
    }
    let payload = d.bytes()?;
    let signature = d
        .bytes()?
        .try_into()
        .map_err(|_| Error::Field("signature length"))?;
    d.end_array()?;
    d.finish()?;
    Ok((payload, signature))
}

fn expect_text(d: &mut Decoder<'_>, expected: &str, field: &'static str) -> Result<()> {
    if d.text()? != expected {
        return Err(Error::Field(field));
    }
    Ok(())
}

fn digest(d: &mut Decoder<'_>) -> Result<[u8; 32]> {
    d.bytes()?
        .try_into()
        .map_err(|_| Error::Field("digest length"))
}
