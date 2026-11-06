//! Strict Ed25519 dolt approvals, in a domain distinct from workflow approvals.
//! Authorization refers only to exact supplied immutable inputs, never live state.
use super::{ApprovalSigningKey, binding};
use crate::{
    Limits, RawJson,
    dolt_operation::DoltOperation,
    operation::{BoundaryError, encode, hex, unhex},
    policy::AuthorityPolicy,
    semantic::{field, identity, obj},
};
use ed25519_dalek::{Signature, Signer, VerifyingKey};
use serde_json::json;
use writfold_domain::{AuthorityKeyId, ContentDigest, PlanDigest, RealmId};

pub const DOLT_APPROVAL_NAMESPACE: &str = "writfold-dolt-approval";
pub const DOLT_APPROVAL_SCHEMA: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltApproval {
    plan: PlanDigest,
    key: AuthorityKeyId,
    signature: [u8; 64],
}
impl DoltApproval {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        let raw = RawJson::parse(bytes, limits)?;
        let v = obj(raw.value(), "namespace schema plan key signature")?;
        if field(v, "namespace")?.as_str() != Some(DOLT_APPROVAL_NAMESPACE) {
            return Err(BoundaryError::Binding(
                "unsupported dolt approval namespace",
            ));
        }
        let schema = field(v, "schema")?
            .as_number()
            .filter(|number| !number.as_str().contains(['.', 'e', 'E']));
        if schema.and_then(serde_json::Number::as_u64) != Some(DOLT_APPROVAL_SCHEMA) {
            return Err(BoundaryError::Binding("unsupported dolt approval schema"));
        }
        let signature = field(v, "signature")?
            .as_str()
            .ok_or(crate::semantic::InvalidRequest)?;
        Ok(Self {
            plan: identity(field(v, "plan")?)?,
            key: identity(field(v, "key")?)?,
            signature: unhex(signature)?,
        })
    }
    pub fn canonical_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, BoundaryError> {
        encode(
            json!({
                "namespace": DOLT_APPROVAL_NAMESPACE, "schema": DOLT_APPROVAL_SCHEMA,
                "plan": self.plan.as_str(), "key": self.key.as_str(), "signature": hex(&self.signature),
            }),
            max_bytes,
        )
    }
    pub fn plan(&self) -> &PlanDigest {
        &self.plan
    }
    pub fn key(&self) -> &AuthorityKeyId {
        &self.key
    }
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

impl ApprovalSigningKey {
    /// Sign only a checked dolt operation under the supplied current policy.
    /// The private key has no Debug representation, including through this API:
    /// ```compile_fail
    /// use writfold_wire::signatures::ApprovalSigningKey;
    /// fn expose(key: &ApprovalSigningKey) { println!("{key:?}"); }
    /// ```
    pub fn sign_dolt(
        &self,
        operation: &DoltOperation,
        policy: &AuthorityPolicy,
        key: AuthorityKeyId,
        expected_toolchain: &ContentDigest,
        expected_realm: &RealmId,
    ) -> Result<DoltApproval, BoundaryError> {
        let trusted = current_key(operation, policy, &key, expected_toolchain, expected_realm)?;
        if self.key.verifying_key() != trusted {
            return Err(BoundaryError::Binding(
                "private key differs from policy key",
            ));
        }
        Ok(DoltApproval {
            plan: operation.id().clone(),
            key,
            signature: self.key.sign(operation.approval_message()).to_bytes(),
        })
    }
}

/// Proof of strict verification relative to these inputs only. Loading/rechecking
/// live predecessors, policies, kernel premises and tool admission is a separate
/// obligation. This type cannot publish, write, retain, or open a dolt backend.
#[derive(Debug)]
pub struct SuppliedDoltAuthorization<'a> {
    operation: &'a DoltOperation,
    approval: &'a DoltApproval,
    policy: &'a AuthorityPolicy,
    toolchain: &'a ContentDigest,
    realm: &'a RealmId,
}
impl<'a> SuppliedDoltAuthorization<'a> {
    pub fn operation(&self) -> &'a DoltOperation {
        self.operation
    }
    pub fn approval(&self) -> &'a DoltApproval {
        self.approval
    }
    pub fn policy(&self) -> &'a AuthorityPolicy {
        self.policy
    }
    pub fn realm(&self) -> &'a RealmId {
        self.realm
    }
    pub fn toolchain(&self) -> &'a ContentDigest {
        self.toolchain
    }
}

pub fn verify_dolt_current<'a>(
    operation: &'a DoltOperation,
    approval: &'a DoltApproval,
    policy: &'a AuthorityPolicy,
    expected_toolchain: &'a ContentDigest,
    expected_realm: &'a RealmId,
) -> Result<SuppliedDoltAuthorization<'a>, BoundaryError> {
    if approval.plan() != operation.id() {
        return Err(BoundaryError::Binding("dolt approval plan differs"));
    }
    let key = current_key(
        operation,
        policy,
        approval.key(),
        expected_toolchain,
        expected_realm,
    )?;
    key.verify_strict(
        operation.approval_message(),
        &Signature::from_bytes(approval.signature()),
    )
    .map_err(|_| BoundaryError::Signature)?;
    Ok(SuppliedDoltAuthorization {
        operation,
        approval,
        policy,
        toolchain: expected_toolchain,
        realm: expected_realm,
    })
}

fn current_key(
    operation: &DoltOperation,
    policy: &AuthorityPolicy,
    key: &AuthorityKeyId,
    expected_toolchain: &ContentDigest,
    expected_realm: &RealmId,
) -> Result<VerifyingKey, BoundaryError> {
    binding::current(operation.operation(), policy, expected_toolchain)?;
    if operation.target().realm() != expected_realm {
        return Err(BoundaryError::Binding(
            "Dolt target and expected realm differ",
        ));
    }
    if operation.target().kb() != policy.kb_id() {
        return Err(BoundaryError::Binding("dolt target and policy KB differ"));
    }
    if operation.operation().actor()? != *key {
        return Err(BoundaryError::Binding(
            "signer differs from designated actor",
        ));
    }
    let key = policy
        .key(key, operation.operation().target_use())?
        .verifying_key()?;
    if key.is_weak() {
        return Err(BoundaryError::WeakAuthority);
    }
    Ok(key)
}
