//! Closed canonical bytes for operator grants and revocations and their signed
//! envelopes. Each has its own payload namespace, envelope namespace and
//! signing domain; none is shared with prepared-edit approvals or any other
//! signed record.
//!
//! Decoding establishes structure only. Signature and policy checks belong to
//! `signatures::grant`; installation and current-grant checks to the kernel.
use super::cbor::{self, Decoder, Value as C};
use crate::Limits;
use writfold_domain::engineering::approval::{Signer, Validity};
use writfold_domain::engineering::grant::{
    Grant, Revocation, capability_code, capability_from_code,
};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_domain::worktree::Scope;
use writfold_domain::{EpochSeconds, ExecutionCost};

pub const GRANT_NAMESPACE: &str = "writfold-operator-grant";
pub const GRANT_ENVELOPE: &str = "writfold-signed-operator-grant";
pub const GRANT_DOMAIN: &[u8] = b"writfold-operator-grant/1\0";
pub const REVOCATION_NAMESPACE: &str = "writfold-grant-revocation";
pub const REVOCATION_ENVELOPE: &str = "writfold-signed-grant-revocation";
pub const REVOCATION_DOMAIN: &[u8] = b"writfold-grant-revocation/1\0";
pub const SCHEMA_VERSION: u64 = 1;
const GRANT_FIELDS: usize = 15;
const REVOCATION_FIELDS: usize = 8;

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
    #[error(transparent)]
    Grant(#[from] writfold_domain::engineering::grant::InvalidGrant),
    #[error("invalid operator grant field: {0}")]
    Field(&'static str),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// `[namespace, version, grant, realm, run, repo, worktree, binding,
/// [resource..], [capability code..], budget, valid_from, expires, key, policy]`.
pub fn encode_grant(grant: &Grant, limits: Limits) -> Result<Vec<u8>> {
    let scope = grant.scope();
    let resources: Vec<C<'_>> = grant
        .resources()
        .iter()
        .map(|r| C::Text(r.as_str()))
        .collect();
    let capabilities: Vec<C<'_>> = grant
        .capabilities()
        .iter()
        .map(|c| C::Unsigned(capability_code(*c)))
        .collect();
    let fields = [
        C::Text(GRANT_NAMESPACE),
        C::Unsigned(SCHEMA_VERSION),
        C::Text(grant.id().as_str()),
        C::Text(scope.realm().as_str()),
        C::Text(scope.run().as_str()),
        C::Text(scope.repo().as_str()),
        C::Text(scope.worktree().as_str()),
        C::Text(scope.binding().as_str()),
        C::Array(&resources),
        C::Array(&capabilities),
        C::Unsigned(grant.budget().get()),
        C::Unsigned(grant.validity().valid_from().get()),
        C::Unsigned(grant.validity().expires().get()),
        C::Text(grant.signer().key().as_str()),
        C::Text(grant.signer().policy().as_str()),
    ];
    Ok(cbor::encode(&C::Array(&fields), limits)?.into_bytes())
}

pub fn decode_grant(bytes: &[u8], limits: Limits) -> Result<Grant> {
    let mut d = Decoder::new(bytes, limits)?;
    header(&mut d, GRANT_FIELDS, GRANT_NAMESPACE)?;
    let id = d.text()?.parse()?;
    let scope = Scope::new(
        d.text()?.parse()?,
        d.text()?.parse()?,
        d.text()?.parse()?,
        d.text()?.parse()?,
        d.text()?.parse()?,
    )?;
    let mut resources = Vec::new();
    for _ in 0..d.array()? {
        resources.push(d.text()?.parse()?);
    }
    d.end_array()?;
    let mut capabilities = Vec::new();
    for _ in 0..d.array()? {
        capabilities.push(capability_from_code(d.unsigned()?).ok_or(Error::Field("capability"))?);
    }
    d.end_array()?;
    let budget = ExecutionCost::new(d.unsigned()?)?;
    let validity = validity(&mut d)?;
    let signer = Signer::new(d.text()?.parse()?, d.text()?.parse()?);
    d.end_array()?;
    d.finish()?;
    Ok(Grant::new(
        id,
        scope,
        resources,
        capabilities,
        budget,
        validity,
        signer,
    )?)
}

/// `[namespace, version, realm, grant, grant material, revoked_at, key, policy]`.
pub fn encode_revocation(revocation: &Revocation, limits: Limits) -> Result<Vec<u8>> {
    let material = revocation.grant_material();
    let fields = [
        C::Text(REVOCATION_NAMESPACE),
        C::Unsigned(SCHEMA_VERSION),
        C::Text(revocation.realm().as_str()),
        C::Text(revocation.grant().as_str()),
        C::Bytes(material.as_digest()),
        C::Unsigned(revocation.revoked_at().get()),
        C::Text(revocation.signer().key().as_str()),
        C::Text(revocation.signer().policy().as_str()),
    ];
    Ok(cbor::encode(&C::Array(&fields), limits)?.into_bytes())
}

pub fn decode_revocation(bytes: &[u8], limits: Limits) -> Result<Revocation> {
    let mut d = Decoder::new(bytes, limits)?;
    header(&mut d, REVOCATION_FIELDS, REVOCATION_NAMESPACE)?;
    let realm = d.text()?.parse()?;
    let grant = d.text()?.parse()?;
    let material = BlobDigest::from_digest(
        d.bytes()?
            .try_into()
            .map_err(|_| Error::Field("digest length"))?,
    );
    let revoked_at = EpochSeconds::new(d.unsigned()?)?;
    let signer = Signer::new(d.text()?.parse()?, d.text()?.parse()?);
    d.end_array()?;
    d.finish()?;
    Ok(Revocation::new(realm, grant, material, revoked_at, signer))
}

/// Signed envelope `[namespace, version, payload bytes, signature]`.
pub fn encode_envelope(
    namespace: &str,
    payload: &[u8],
    signature: &[u8; 64],
    limits: Limits,
) -> Result<Vec<u8>> {
    let fields = [
        C::Text(namespace),
        C::Unsigned(SCHEMA_VERSION),
        C::Bytes(payload),
        C::Bytes(signature),
    ];
    Ok(cbor::encode(&C::Array(&fields), limits)?.into_bytes())
}

/// Returns the exact payload bytes (borrowed from the input) and signature.
pub fn decode_envelope<'a>(
    namespace: &str,
    bytes: &'a [u8],
    limits: Limits,
) -> Result<(&'a [u8], [u8; 64])> {
    let mut d = Decoder::new(bytes, limits)?;
    header(&mut d, 4, namespace)?;
    let payload = d.bytes()?;
    let signature = d
        .bytes()?
        .try_into()
        .map_err(|_| Error::Field("signature length"))?;
    d.end_array()?;
    d.finish()?;
    Ok((payload, signature))
}

fn header(d: &mut Decoder<'_>, fields: usize, namespace: &str) -> Result<()> {
    if d.array()? != fields {
        return Err(Error::Field("field count"));
    }
    if d.text()? != namespace {
        return Err(Error::Field("namespace"));
    }
    if d.unsigned()? != SCHEMA_VERSION {
        return Err(Error::Field("schema version"));
    }
    Ok(())
}

fn validity(d: &mut Decoder<'_>) -> Result<Validity> {
    Ok(Validity::new(
        EpochSeconds::new(d.unsigned()?)?,
        EpochSeconds::new(d.unsigned()?)?,
    )?)
}
