//! Immutable operator grants and revocations.
//!
//! A grant names the run scope, resources and capabilities an operator allows
//! for a validity interval. Construction sorts and deduplicates the sets and
//! refuses empty ones; it is not a signature, an installation or current
//! authority. Installation and the current-grant check live in the kernel;
//! signature verification lives at the wire boundary.
use super::BlobDigest;
use super::approval::{Signer, Validity};
use super::execution::Capability;
use crate::worktree::Scope;
use crate::{EpochSeconds, ExecutionCost, GrantId, RealmId, ResourceId};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGrant;
impl fmt::Display for InvalidGrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid operator grant")
    }
}
impl Error for InvalidGrant {}

/// Closed wire order of capabilities; also their canonical sort order.
pub fn capability_code(capability: Capability) -> u64 {
    match capability {
        Capability::Read => 0,
        Capability::Check => 1,
        Capability::Write => 2,
        Capability::Publish => 3,
    }
}

pub fn capability_from_code(code: u64) -> Option<Capability> {
    match code {
        0 => Some(Capability::Read),
        1 => Some(Capability::Check),
        2 => Some(Capability::Write),
        3 => Some(Capability::Publish),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grant {
    id: GrantId,
    scope: Scope,
    resources: Vec<ResourceId>,
    capabilities: Vec<Capability>,
    budget: ExecutionCost,
    validity: Validity,
    signer: Signer,
}
impl Grant {
    #[allow(
        clippy::too_many_arguments,
        reason = "a grant binds seven distinct exact values"
    )]
    pub fn new(
        id: GrantId,
        scope: Scope,
        mut resources: Vec<ResourceId>,
        mut capabilities: Vec<Capability>,
        budget: ExecutionCost,
        validity: Validity,
        signer: Signer,
    ) -> Result<Self, InvalidGrant> {
        resources.sort();
        resources.dedup();
        capabilities.sort_by_key(|c| capability_code(*c));
        capabilities.dedup();
        if resources.is_empty() || capabilities.is_empty() {
            return Err(InvalidGrant);
        }
        Ok(Self {
            id,
            scope,
            resources,
            capabilities,
            budget,
            validity,
            signer,
        })
    }
    pub fn id(&self) -> &GrantId {
        &self.id
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    /// Sorted, deduplicated, nonempty.
    pub fn resources(&self) -> &[ResourceId] {
        &self.resources
    }
    /// Sorted by `capability_code`, deduplicated, nonempty.
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
    pub fn budget(&self) -> ExecutionCost {
        self.budget
    }
    pub fn validity(&self) -> Validity {
        self.validity
    }
    pub fn signer(&self) -> &Signer {
        &self.signer
    }
}

/// Withdraws one grant by id. `grant_material` names the exact retained grant
/// envelope the operator saw; revocation still applies by id, so a mismatch
/// can never keep a grant alive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revocation {
    realm: RealmId,
    grant: GrantId,
    grant_material: BlobDigest,
    revoked_at: EpochSeconds,
    signer: Signer,
}
impl Revocation {
    pub fn new(
        realm: RealmId,
        grant: GrantId,
        grant_material: BlobDigest,
        revoked_at: EpochSeconds,
        signer: Signer,
    ) -> Self {
        Self {
            realm,
            grant,
            grant_material,
            revoked_at,
            signer,
        }
    }
    pub fn realm(&self) -> &RealmId {
        &self.realm
    }
    pub fn grant(&self) -> &GrantId {
        &self.grant
    }
    pub fn grant_material(&self) -> BlobDigest {
        self.grant_material
    }
    pub fn revoked_at(&self) -> EpochSeconds {
        self.revoked_at
    }
    pub fn signer(&self) -> &Signer {
        &self.signer
    }
}
