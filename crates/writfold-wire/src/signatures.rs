//! Detached Ed25519 approvals. Current authorization is relative to supplied
//! immutable inputs, never evidence that those inputs are the live store state.
mod binding;
use crate::{
    Limits, RawJson,
    operation::{BoundaryError, Operation, encode, hex, unhex},
    policy::AuthorityPolicy,
    semantic::*,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, pkcs8::DecodePrivateKey};
use serde_json::json;
use writfold_domain::{AuthorityKeyId, ContentDigest, PlanId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approval {
    schema_version: u8,
    plan: PlanId,
    key: AuthorityKeyId,
    signature: [u8; 64],
}
impl Approval {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        let raw = RawJson::parse(bytes, limits)?;
        let value = obj(raw.value(), "schema_version plan key signature")?;
        let schema_version = match integer(field(value, "schema_version")?)?
            .to_string()
            .as_str()
        {
            "2" => 2,
            "3" => 3,
            _ => return Err(BoundaryError::Binding("unsupported approval schema")),
        };
        Ok(Self {
            schema_version,
            plan: identity(field(value, "plan")?)?,
            key: identity(field(value, "key")?)?,
            signature: unhex(&text(field(value, "signature")?)?)?,
        })
    }
    pub fn canonical_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, BoundaryError> {
        encode(
            json!({"schema_version": self.schema_version, "plan": self.plan.as_str(),
            "key": self.key.as_str(), "signature": hex(&self.signature)}),
            max_bytes,
        )
    }
    pub fn schema_version(&self) -> u8 {
        self.schema_version
    }
    pub fn plan(&self) -> &PlanId {
        &self.plan
    }
    pub fn key(&self) -> &AuthorityKeyId {
        &self.key
    }
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

/// Private-key material is never exposed or included in Debug output. Dalek's
/// configured zeroize support clears its owned secret material on drop.
pub struct ApprovalSigningKey {
    key: SigningKey,
}
impl ApprovalSigningKey {
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self, BoundaryError> {
        SigningKey::from_pkcs8_pem(pem)
            .map(|key| Self { key })
            .map_err(|_| BoundaryError::PrivateKey)
    }
    /// Sign exactly this prepared schema-3 operation after policy/toolchain
    /// binding; there is no legacy signing or automatic plan refresh.
    pub fn sign(
        &self,
        operation: &Operation,
        policy: &AuthorityPolicy,
        key: AuthorityKeyId,
        expected_toolchain: &ContentDigest,
    ) -> Result<Approval, BoundaryError> {
        binding::current(operation, policy, expected_toolchain)?;
        if operation.actor()? != key {
            return Err(BoundaryError::Binding(
                "signer differs from designated actor",
            ));
        }
        let trusted = policy.key(&key, operation.target_use())?;
        if self.key.verifying_key().as_bytes() != trusted.public_key() {
            return Err(BoundaryError::Binding(
                "private key differs from policy key",
            ));
        }
        let approval = Approval {
            schema_version: 3,
            plan: operation.header().id().clone(),
            key,
            signature: self.key.sign(operation.approval_message()).to_bytes(),
        };
        verify_current(operation, &approval, policy, expected_toolchain)?;
        Ok(approval)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoricalEquation {
    Rejected,
    StrongStrict,
    WeakOrNonStrict,
}
/// Inspection only: neither even StrongStrict nor a model approval is an IO
/// capability. Original policy and canonical signed operation remain accessible.
#[derive(Debug)]
pub struct HistoricalInspection<'a> {
    operation: &'a Operation,
    approval: &'a Approval,
    policy: &'a AuthorityPolicy,
    equation: HistoricalEquation,
}
impl<'a> HistoricalInspection<'a> {
    pub fn operation(&self) -> &'a Operation {
        self.operation
    }
    pub fn approval(&self) -> &'a Approval {
        self.approval
    }
    pub fn policy(&self) -> &'a AuthorityPolicy {
        self.policy
    }
    pub fn equation(&self) -> HistoricalEquation {
        self.equation
    }
}

/// Preserve ordinary historical equation verification, including WF-AUTH-001's
/// identity-point acceptance, but never promote it into current authorization.
pub fn inspect_historical<'a>(
    operation: &'a Operation,
    approval: &'a Approval,
    policy: &'a AuthorityPolicy,
) -> Result<HistoricalInspection<'a>, BoundaryError> {
    binding::approval(operation, approval, policy)?;
    let key = policy
        .key(approval.key(), operation.target_use())?
        .verifying_key()?;
    let message = operation.approval_message();
    let signature = Signature::from_bytes(approval.signature());
    let equation = if key.verify(message, &signature).is_err() {
        HistoricalEquation::Rejected
    } else if key.is_weak() || key.verify_strict(message, &signature).is_err() {
        HistoricalEquation::WeakOrNonStrict
    } else {
        HistoricalEquation::StrongStrict
    };
    Ok(HistoricalInspection {
        operation,
        approval,
        policy,
        equation,
    })
}

/// Cryptographically authorized relative to these exact supplied inputs only.
/// The store facade must load/recheck live policy, predecessor and toolchain,
/// validate kernel/state/capture premises, and own any publication capability.
#[derive(Debug)]
pub struct SuppliedAuthorization<'a> {
    operation: &'a Operation,
    approval: &'a Approval,
    policy: &'a AuthorityPolicy,
    toolchain: &'a ContentDigest,
}
impl<'a> SuppliedAuthorization<'a> {
    pub fn operation(&self) -> &'a Operation {
        self.operation
    }
    pub fn approval(&self) -> &'a Approval {
        self.approval
    }
    pub fn policy(&self) -> &'a AuthorityPolicy {
        self.policy
    }
    pub fn toolchain(&self) -> &'a ContentDigest {
        self.toolchain
    }
}

pub fn verify_current<'a>(
    operation: &'a Operation,
    approval: &'a Approval,
    policy: &'a AuthorityPolicy,
    expected_toolchain: &'a ContentDigest,
) -> Result<SuppliedAuthorization<'a>, BoundaryError> {
    binding::current(operation, policy, expected_toolchain)?;
    binding::approval(operation, approval, policy)?;
    let key = policy
        .key(approval.key(), operation.target_use())?
        .verifying_key()?;
    if key.is_weak() {
        return Err(BoundaryError::WeakAuthority);
    }
    key.verify_strict(
        operation.approval_message(),
        &Signature::from_bytes(approval.signature()),
    )
    .map_err(|_| BoundaryError::Signature)?;
    Ok(SuppliedAuthorization {
        operation,
        approval,
        policy,
        toolchain: expected_toolchain,
    })
}
