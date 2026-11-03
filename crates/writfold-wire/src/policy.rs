//! Schema-2 authority policy; retained bytes and immutable projections are distinct.
use crate::{
    Limits, RawJson, canonical,
    operation::{BoundaryError, hash, unhex},
    semantic::*,
};
use ed25519_dalek::VerifyingKey;
use std::collections::BTreeMap;
use writfold_domain::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedKey {
    public_key: [u8; 32],
    target_uses: Vec<String>,
}
impl TrustedKey {
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }
    pub fn target_uses(&self) -> &[String] {
        &self.target_uses
    }
    pub fn permits(&self, target_use: &str) -> bool {
        self.target_uses.iter().any(|value| value == target_use)
    }
    pub(crate) fn verifying_key(&self) -> Result<VerifyingKey, BoundaryError> {
        VerifyingKey::from_bytes(&self.public_key).map_err(|_| BoundaryError::PublicKey)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyIdentityProfile {
    HistoricalOriginalBytes,
    NewBootstrapCanonical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityPolicy {
    kb_id: KbId,
    policy_version: PolicyRevision,
    trusted_keys: BTreeMap<AuthorityKeyId, TrustedKey>,
    original: Vec<u8>,
    canonical: Vec<u8>,
    original_digest: PolicyDigest,
    identity: PolicyDigest,
    profile: PolicyIdentityProfile,
}
impl AuthorityPolicy {
    /// Historical identity hashes the original bytes, including whitespace.
    /// This does not admit a key as strong or authorize an operation.
    pub fn historical(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        Self::decode(
            bytes,
            limits,
            PolicyIdentityProfile::HistoricalOriginalBytes,
        )
    }
    /// New trust anchoring validates all keys and semantic identities before
    /// hashing canonical bytes. No strings are trimmed or otherwise normalized.
    pub fn new_bootstrap(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        Self::decode(bytes, limits, PolicyIdentityProfile::NewBootstrapCanonical)
    }
    fn decode(
        bytes: &[u8],
        limits: Limits,
        profile: PolicyIdentityProfile,
    ) -> Result<Self, BoundaryError> {
        let raw = RawJson::parse(bytes, limits)?;
        let v = obj(
            raw.value(),
            "schema_version kb_id policy_version trusted_keys",
        )?;
        if integer(field(v, "schema_version")?)?.to_string() != "2" {
            return Err(BoundaryError::Binding(
                "unsupported authority policy schema",
            ));
        }
        let kb = text(field(v, "kb_id")?)?;
        let version = text(field(v, "policy_version")?)?;
        if !writfold_domain::text::nonblank(&kb) || !writfold_domain::text::nonblank(&version) {
            return Err(BoundaryError::Binding("blank policy identity"));
        }
        let mut trusted_keys = BTreeMap::new();
        for (name, value) in object(field(v, "trusted_keys")?)? {
            let id = name
                .parse::<AuthorityKeyId>()
                .map_err(|_| BoundaryError::Identity)?;
            let key = obj(value, "public_key target_uses")?;
            let public_key = unhex(&text(field(key, "public_key")?)?)?;
            let target_uses = items(field(key, "target_uses")?, text)?;
            if target_uses.is_empty()
                || !unique(&target_uses)
                || target_uses.iter().any(|s| !writfold_domain::text::nonblank(s))
            {
                return Err(BoundaryError::Binding(
                    "target uses must be distinct and nonblank",
                ));
            }
            trusted_keys.insert(
                id,
                TrustedKey {
                    public_key,
                    target_uses,
                },
            );
        }
        if profile == PolicyIdentityProfile::NewBootstrapCanonical {
            format!("bootstrap:{kb}")
                .parse::<SnapshotId>()
                .map_err(|_| BoundaryError::Identity)?;
            version
                .parse::<PolicyId>()
                .map_err(|_| BoundaryError::Identity)?;
            for (id, key) in &trusted_keys {
                id.as_str()
                    .parse::<ActorId>()
                    .map_err(|_| BoundaryError::Identity)?;
                if key.verifying_key()?.is_weak() {
                    return Err(BoundaryError::WeakAuthority);
                }
            }
        }
        let canonical = canonical(&raw, limits.bytes())?;
        let original_digest =
            PolicyDigest::try_from(hash(bytes)).map_err(|_| BoundaryError::Identity)?;
        let identity = match profile {
            PolicyIdentityProfile::HistoricalOriginalBytes => original_digest.clone(),
            PolicyIdentityProfile::NewBootstrapCanonical => {
                PolicyDigest::try_from(hash(&canonical)).map_err(|_| BoundaryError::Identity)?
            }
        };
        Ok(Self {
            kb_id: KbId::try_from(kb).map_err(|_| BoundaryError::Identity)?,
            policy_version: PolicyRevision::try_from(version)
                .map_err(|_| BoundaryError::Identity)?,
            trusted_keys,
            original: bytes.to_vec(),
            canonical,
            original_digest,
            identity,
            profile,
        })
    }
    pub fn kb_id(&self) -> &KbId {
        &self.kb_id
    }
    pub fn policy_version(&self) -> &PolicyRevision {
        &self.policy_version
    }
    pub fn trusted_keys(&self) -> &BTreeMap<AuthorityKeyId, TrustedKey> {
        &self.trusted_keys
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub fn original_digest(&self) -> &PolicyDigest {
        &self.original_digest
    }
    pub fn identity(&self) -> &PolicyDigest {
        &self.identity
    }
    pub fn profile(&self) -> PolicyIdentityProfile {
        self.profile
    }
    pub fn bootstrap_snapshot(&self) -> Result<SnapshotId, BoundaryError> {
        SnapshotId::try_from(format!("bootstrap:{}", self.kb_id))
            .map_err(|_| BoundaryError::Identity)
    }
    pub fn semantic_policy_id(&self) -> Result<PolicyId, BoundaryError> {
        self.policy_version
            .as_str()
            .parse()
            .map_err(|_| BoundaryError::Identity)
    }
    pub fn authorized_actors(&self, target_use: &str) -> Result<Vec<ActorId>, BoundaryError> {
        self.trusted_keys
            .iter()
            .filter(|(_, key)| key.permits(target_use))
            .map(|(id, _)| id.as_str().parse().map_err(|_| BoundaryError::Identity))
            .collect()
    }
    pub fn validate_context(&self, context: &Context) -> Result<(), BoundaryError> {
        if context.policy().id().as_str() != self.policy_version.as_str() {
            return Err(BoundaryError::Binding("semantic policy identity differs"));
        }
        let allowed = self.authorized_actors(context.intended_use())?;
        let actual = context.policy().authorized_actors();
        if actual.len() != allowed.len()
            || !unique(actual)
            || actual.iter().any(|actor| !allowed.contains(actor))
        {
            return Err(BoundaryError::Binding("semantic authorized actors differ"));
        }
        Ok(())
    }
    pub(crate) fn key(
        &self,
        actor: &AuthorityKeyId,
        target_use: &str,
    ) -> Result<&TrustedKey, BoundaryError> {
        self.trusted_keys
            .get(actor)
            .filter(|key| key.permits(target_use))
            .ok_or(BoundaryError::Binding("untrusted key or unauthorized use"))
    }
}
