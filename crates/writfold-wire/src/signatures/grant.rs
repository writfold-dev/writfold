//! Strict Ed25519 operator grants and revocations, each in its own signing
//! domain and policy target use.
//!
//! Verification is relative to the supplied policy only. A verified grant is a
//! kernel `Authorization` candidate; it is not installed, not current, and
//! cannot perform native IO. A verified revocation withdraws a grant by id.
use super::ApprovalSigningKey;
use crate::engineering::grant::{
    self as codec, GRANT_DOMAIN, GRANT_ENVELOPE, REVOCATION_DOMAIN, REVOCATION_ENVELOPE,
};
use crate::{Limits, operation::BoundaryError, policy::AuthorityPolicy};
use ed25519_dalek::{Signature, Signer as _, VerifyingKey};
use writfold_domain::engineering::approval::Signer;
use writfold_domain::engineering::grant::{Grant, Revocation};

/// Policy target use a key must carry to sign operator grants.
pub const GRANT_USE: &str = "operator-grant";
/// Policy target use a key must carry to sign grant revocations.
pub const REVOCATION_USE: &str = "grant-revocation";

/// One closed signed record kind: its codec, envelope, domain and use.
pub trait Record: Sized + Clone + Eq {
    const ENVELOPE: &'static str;
    const DOMAIN: &'static [u8];
    const USE: &'static str;
    const NAME: &'static str;
    fn signer(&self) -> &Signer;
    fn encode(&self, limits: Limits) -> codec::Result<Vec<u8>>;
    fn decode(bytes: &[u8], limits: Limits) -> codec::Result<Self>;
}

impl Record for Grant {
    const ENVELOPE: &'static str = GRANT_ENVELOPE;
    const DOMAIN: &'static [u8] = GRANT_DOMAIN;
    const USE: &'static str = GRANT_USE;
    const NAME: &'static str = "operator grant";
    fn signer(&self) -> &Signer {
        Grant::signer(self)
    }
    fn encode(&self, limits: Limits) -> codec::Result<Vec<u8>> {
        codec::encode_grant(self, limits)
    }
    fn decode(bytes: &[u8], limits: Limits) -> codec::Result<Self> {
        codec::decode_grant(bytes, limits)
    }
}

impl Record for Revocation {
    const ENVELOPE: &'static str = REVOCATION_ENVELOPE;
    const DOMAIN: &'static [u8] = REVOCATION_DOMAIN;
    const USE: &'static str = REVOCATION_USE;
    const NAME: &'static str = "grant revocation";
    fn signer(&self) -> &Signer {
        Revocation::signer(self)
    }
    fn encode(&self, limits: Limits) -> codec::Result<Vec<u8>> {
        codec::encode_revocation(self, limits)
    }
    fn decode(bytes: &[u8], limits: Limits) -> codec::Result<Self> {
        codec::decode_revocation(bytes, limits)
    }
}

fn malformed(_: codec::Error) -> BoundaryError {
    BoundaryError::Binding("malformed operator authority record")
}

/// A decoded record together with its exact signed payload bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signed<T> {
    record: T,
    payload: Vec<u8>,
    signature: [u8; 64],
}
impl<T: Record> Signed<T> {
    /// Decode an envelope. The payload must be the canonical encoding of the
    /// record it decodes to, so unsorted or duplicated sets are refused.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        let (payload, signature) =
            codec::decode_envelope(T::ENVELOPE, bytes, limits).map_err(malformed)?;
        let record = T::decode(payload, limits).map_err(malformed)?;
        if record.encode(limits).map_err(malformed)? != payload {
            return Err(BoundaryError::Binding(
                "noncanonical operator authority record",
            ));
        }
        Ok(Self {
            record,
            payload: payload.to_vec(),
            signature,
        })
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, BoundaryError> {
        codec::encode_envelope(T::ENVELOPE, &self.payload, &self.signature, limits)
            .map_err(malformed)
    }
    pub fn record(&self) -> &T {
        &self.record
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

fn message<T: Record>(payload: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(T::DOMAIN.len() + payload.len());
    message.extend_from_slice(T::DOMAIN);
    message.extend_from_slice(payload);
    message
}

fn trusted_key<T: Record>(
    record: &T,
    policy: &AuthorityPolicy,
) -> Result<VerifyingKey, BoundaryError> {
    if record.signer().policy() != policy.identity() {
        return Err(BoundaryError::Binding("record names a different policy"));
    }
    let trusted = policy
        .trusted_keys()
        .get(record.signer().key())
        .ok_or(BoundaryError::Binding("record key is not in the policy"))?;
    if !trusted.permits(T::USE) {
        return Err(BoundaryError::Binding("record key lacks the target use"));
    }
    let key = trusted.verifying_key()?;
    if key.is_weak() {
        return Err(BoundaryError::WeakAuthority);
    }
    Ok(key)
}

impl ApprovalSigningKey {
    /// Sign one exact grant or revocation under the supplied policy.
    pub fn sign_record<T: Record>(
        &self,
        record: T,
        policy: &AuthorityPolicy,
        limits: Limits,
    ) -> Result<Signed<T>, BoundaryError> {
        if self.key.verifying_key() != trusted_key(&record, policy)? {
            return Err(BoundaryError::Binding(
                "private key differs from policy key",
            ));
        }
        let payload = record.encode(limits).map_err(malformed)?;
        let signature = self.key.sign(&message::<T>(&payload)).to_bytes();
        Ok(Signed {
            record,
            payload,
            signature,
        })
    }
}

/// Strictly verified relative to `policy` only.
#[derive(Debug)]
pub struct Verified<'a, T> {
    signed: &'a Signed<T>,
    policy: &'a AuthorityPolicy,
}
impl<'a, T> Verified<'a, T> {
    pub fn signed(&self) -> &'a Signed<T> {
        self.signed
    }
    pub fn policy(&self) -> &'a AuthorityPolicy {
        self.policy
    }
}

pub fn verify<'a, T: Record>(
    signed: &'a Signed<T>,
    policy: &'a AuthorityPolicy,
) -> Result<Verified<'a, T>, BoundaryError> {
    let key = trusted_key(signed.record(), policy)?;
    key.verify_strict(
        &message::<T>(signed.payload()),
        &Signature::from_bytes(signed.signature()),
    )
    .map_err(|_| BoundaryError::Signature)?;
    Ok(Verified { signed, policy })
}
