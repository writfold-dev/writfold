//! Installed operator grants and the current-grant check, refining Lean
//! `environment` `installGrant`/`revokeGrant`/`advanceTime` and `currentGrant`
//! (`Engineering/Environment.lean`, `Engineering/Control.lean`).
//!
//! Pure and IO-free. An `Authorization` is a qualified input: the caller must
//! already have verified the operator's signature over the exact grant and
//! retained its envelope. This module checks realm, policy, retention, fresh
//! identity and time. Historical grants, including revoked ones, are never
//! removed or replaced. Tested Rust, not a proved refinement.
use std::collections::BTreeSet;
use writfold_domain::engineering::BlobDigest;
use writfold_domain::engineering::execution::Capability;
use writfold_domain::engineering::grant::Grant;
use writfold_domain::worktree::Scope;
use writfold_domain::{EpochSeconds, GrantId, PolicyDigest, RealmId, ResourceId};

/// Lean `GrantAuthorization`: the envelope's realm and policy, the grant, and
/// the blob digest of its exact retained signed envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authorization {
    pub realm: RealmId,
    pub policy: PolicyDigest,
    pub grant: Grant,
    pub envelope: BlobDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Installed {
    grant: Grant,
    envelope: BlobDigest,
    revoked: bool,
}
impl Installed {
    pub fn grant(&self) -> &Grant {
        &self.grant
    }
    pub fn envelope(&self) -> BlobDigest {
        self.envelope
    }
    pub fn revoked(&self) -> bool {
        self.revoked
    }
}

/// Why an installation or clock step was refused; each names one Lean guard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    ForeignRealm,
    ForeignBindingRealm,
    StalePolicy,
    UnretainedEnvelope,
    DuplicateGrant,
    NotYetValid,
    Expired,
    ClockRewind,
}

/// Grants for one realm under one policy, newest first as in Lean.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authority {
    realm: RealmId,
    policy: PolicyDigest,
    now: EpochSeconds,
    grants: Vec<Installed>,
}

impl Authority {
    pub fn new(realm: RealmId, policy: PolicyDigest, now: EpochSeconds) -> Self {
        Self {
            realm,
            policy,
            now,
            grants: Vec::new(),
        }
    }
    pub fn now(&self) -> EpochSeconds {
        self.now
    }
    /// Newest first, including revoked grants.
    pub fn grants(&self) -> &[Installed] {
        &self.grants
    }

    /// Lean `advanceTime`: time never moves backwards.
    pub fn advance(&mut self, time: EpochSeconds) -> Result<(), Refusal> {
        if time < self.now {
            return Err(Refusal::ClockRewind);
        }
        self.now = time;
        Ok(())
    }

    /// Lean `installGrant`; on refusal nothing changes. A revoked id can never
    /// be installed again: renewal needs a fresh id.
    pub fn install(
        &mut self,
        authorization: Authorization,
        retained: &BTreeSet<BlobDigest>,
    ) -> Result<(), Refusal> {
        let grant = &authorization.grant;
        let validity = grant.validity();
        let rules = [
            (authorization.realm == self.realm, Refusal::ForeignRealm),
            (
                grant.scope().realm() == &self.realm,
                Refusal::ForeignBindingRealm,
            ),
            (authorization.policy == self.policy, Refusal::StalePolicy),
            (
                retained.contains(&authorization.envelope),
                Refusal::UnretainedEnvelope,
            ),
            (
                !self.grants.iter().any(|g| g.grant.id() == grant.id()),
                Refusal::DuplicateGrant,
            ),
            (validity.valid_from() <= self.now, Refusal::NotYetValid),
            (self.now < validity.expires(), Refusal::Expired),
        ];
        if let Some((_, refusal)) = rules.into_iter().find(|(ok, _)| !ok) {
            return Err(refusal);
        }
        self.grants.insert(
            0,
            Installed {
                grant: authorization.grant,
                envelope: authorization.envelope,
                revoked: false,
            },
        );
        Ok(())
    }

    /// Lean `revokeGrant`: marks every grant with this id revoked; an unknown
    /// id changes nothing. Revocation is never refused.
    pub fn revoke(&mut self, id: &GrantId) {
        for installed in self.grants.iter_mut().filter(|g| g.grant.id() == id) {
            installed.revoked = true;
        }
    }

    /// Lean `currentGrant`: the newest unrevoked grant for exactly this run
    /// scope that is valid now. A newer grant shadows older ones even when it
    /// covers less; there is no fallback search.
    pub fn current(&self, scope: &Scope) -> Option<&Grant> {
        self.grants
            .iter()
            .find(|g| {
                g.grant.scope() == scope && !g.revoked && g.grant.validity().contains(self.now)
            })
            .map(|g| &g.grant)
    }

    /// The grant conjunct of Lean `intentReady`: the current grant carries the
    /// capability and every requested resource. An empty request is covered;
    /// callers that act on resources must refuse empty sets themselves.
    pub fn covers(&self, scope: &Scope, capability: Capability, resources: &[ResourceId]) -> bool {
        self.current(scope).is_some_and(|g| {
            g.capabilities().contains(&capability)
                && resources.iter().all(|r| g.resources().contains(r))
        })
    }
}
