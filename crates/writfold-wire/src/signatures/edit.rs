//! Strict Ed25519 approval of one exact prepared edit, in its own signing domain.
//!
//! Verification is relative to the supplied policy and the caller's current
//! expected values only. A verified approval cannot write, reserve, claim or
//! reconcile anything; it does not check grants or revocations, which are
//! separate owner records.
use super::ApprovalSigningKey;
use crate::engineering::edit_approval::{self as codec, SIGNING_DOMAIN};
use crate::{Limits, operation::BoundaryError, policy::AuthorityPolicy};
use ed25519_dalek::{Signature, Signer as _, VerifyingKey};
use writfold_domain::engineering::approval::EditApproval;
use writfold_domain::engineering::{BlobDigest, ProcedureRevision};
use writfold_domain::worktree::{PreparedEditDigest, Scope};
use writfold_domain::{EpochSeconds, ResourceGeneration, ResourceId};

/// The policy target use a key must carry to approve prepared edits.
pub const EDIT_APPROVAL_USE: &str = "prepared-edit-approval";

fn malformed(_: codec::Error) -> BoundaryError {
    BoundaryError::Binding("malformed prepared-edit approval")
}

/// The decoded approval together with its exact signed payload bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedEditApproval {
    approval: EditApproval,
    payload: Vec<u8>,
    signature: [u8; 64],
}
impl SignedEditApproval {
    /// Decode an envelope. The payload must be the canonical encoding of the
    /// approval it decodes to; no alternative spelling is accepted.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        let (payload, signature) = codec::decode_envelope(bytes, limits).map_err(malformed)?;
        let approval = codec::decode_payload(payload, limits).map_err(malformed)?;
        if codec::encode_payload(&approval, limits).map_err(malformed)? != payload {
            return Err(BoundaryError::Binding(
                "noncanonical prepared-edit approval",
            ));
        }
        Ok(Self {
            approval,
            payload: payload.to_vec(),
            signature,
        })
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, BoundaryError> {
        codec::encode_envelope(&self.payload, &self.signature, limits).map_err(malformed)
    }
    pub fn approval(&self) -> &EditApproval {
        &self.approval
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

fn message(payload: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(SIGNING_DOMAIN.len() + payload.len());
    message.extend_from_slice(SIGNING_DOMAIN);
    message.extend_from_slice(payload);
    message
}

/// Resolve the signer's key under the supplied policy for this use only.
fn trusted_key(
    approval: &EditApproval,
    policy: &AuthorityPolicy,
) -> Result<VerifyingKey, BoundaryError> {
    if approval.signer().policy() != policy.identity() {
        return Err(BoundaryError::Binding("approval names a different policy"));
    }
    let trusted = policy
        .trusted_keys()
        .get(approval.signer().key())
        .ok_or(BoundaryError::Binding("approval key is not in the policy"))?;
    if !trusted.permits(EDIT_APPROVAL_USE) {
        return Err(BoundaryError::Binding(
            "approval key lacks the edit-approval use",
        ));
    }
    let key = trusted.verifying_key()?;
    if key.is_weak() {
        return Err(BoundaryError::WeakAuthority);
    }
    Ok(key)
}

impl ApprovalSigningKey {
    /// Sign one exact prepared-edit approval under the supplied policy.
    pub fn sign_edit(
        &self,
        approval: EditApproval,
        policy: &AuthorityPolicy,
        limits: Limits,
    ) -> Result<SignedEditApproval, BoundaryError> {
        if self.key.verifying_key() != trusted_key(&approval, policy)? {
            return Err(BoundaryError::Binding(
                "private key differs from policy key",
            ));
        }
        let payload = codec::encode_payload(&approval, limits).map_err(malformed)?;
        let signature = self.key.sign(&message(&payload)).to_bytes();
        Ok(SignedEditApproval {
            approval,
            payload,
            signature,
        })
    }
}

/// The caller's current values the approval must match exactly.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedEdit<'a> {
    pub scope: &'a Scope,
    pub prepared: PreparedEditDigest,
    pub report: BlobDigest,
    pub procedure: ProcedureRevision,
    pub resource: &'a ResourceId,
    pub generation: ResourceGeneration,
}

/// Strict verification relative to these inputs only. Holding this value is
/// not permission to perform native IO.
#[derive(Debug)]
pub struct VerifiedEditApproval<'a> {
    signed: &'a SignedEditApproval,
    policy: &'a AuthorityPolicy,
    now: EpochSeconds,
}
impl<'a> VerifiedEditApproval<'a> {
    pub fn signed(&self) -> &'a SignedEditApproval {
        self.signed
    }
    pub fn policy(&self) -> &'a AuthorityPolicy {
        self.policy
    }
    pub fn verified_at(&self) -> EpochSeconds {
        self.now
    }
}

pub fn verify_edit<'a>(
    signed: &'a SignedEditApproval,
    policy: &'a AuthorityPolicy,
    expected: ExpectedEdit<'_>,
    now: EpochSeconds,
) -> Result<VerifiedEditApproval<'a>, BoundaryError> {
    let approval = signed.approval();
    let key = trusted_key(approval, policy)?;
    key.verify_strict(
        &message(signed.payload()),
        &Signature::from_bytes(signed.signature()),
    )
    .map_err(|_| BoundaryError::Signature)?;
    let checks = [
        (approval.scope() == expected.scope, "approval scope differs"),
        (
            approval.prepared() == expected.prepared,
            "approval names a different prepared edit",
        ),
        (
            approval.report() == expected.report,
            "approval names a different report",
        ),
        (
            approval.procedure() == expected.procedure,
            "approval names a different procedure",
        ),
        (
            approval.resource() == expected.resource,
            "approval names a different resource",
        ),
        (
            approval.generation() == expected.generation,
            "approval resource generation is stale",
        ),
    ];
    if let Some((_, reason)) = checks.iter().find(|(ok, _)| !*ok) {
        return Err(BoundaryError::Binding(reason));
    }
    if !approval.validity().contains(now) {
        return Err(BoundaryError::Binding(
            "approval is outside its validity interval",
        ));
    }
    Ok(VerifiedEditApproval {
        signed,
        policy,
        now,
    })
}
