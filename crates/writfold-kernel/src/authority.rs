//! Installed operator grants and the current-grant check: the typed adapter
//! over `writfold_core::authority`, whose Aeneas translation is proved in
//! `proofs/Writfold/Authority.lean` (`install_check_spec`, `current_spec`,
//! `covers_spec`, `Authority.install_refused`, `Authority.advance_spec`,
//! `Authority.revoke_spec`).
//!
//! An `Authorization` is a qualified input: the caller must already have
//! verified the operator's signature over the exact grant and retained its
//! envelope. Every decision is made by the proved core; this module converts
//! nominal domain values to bytes and keeps the typed grants beside the core's
//! (same order). Historical grants, including revoked ones, are never removed.
use std::collections::BTreeSet;
use writfold_core::authority as core;
use writfold_domain::engineering::execution::Capability;
use writfold_domain::engineering::grant::{Grant, capability_code};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
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

fn refusal(r: core::Refusal) -> Refusal {
    match r {
        core::Refusal::ForeignRealm => Refusal::ForeignRealm,
        core::Refusal::ForeignBindingRealm => Refusal::ForeignBindingRealm,
        core::Refusal::StalePolicy => Refusal::StalePolicy,
        core::Refusal::UnretainedEnvelope => Refusal::UnretainedEnvelope,
        core::Refusal::DuplicateGrant => Refusal::DuplicateGrant,
        core::Refusal::NotYetValid => Refusal::NotYetValid,
        core::Refusal::Expired => Refusal::Expired,
        core::Refusal::ClockRewind => Refusal::ClockRewind,
    }
}

/// Canonical scope bytes: the five identity fields, NUL-separated. No field
/// can contain NUL, so equal bytes mean an equal scope.
fn scope_bytes(scope: &Scope) -> Vec<u8> {
    [
        scope.realm().as_str(),
        scope.run().as_str(),
        scope.repo().as_str(),
        scope.worktree().as_str(),
        scope.binding().as_str(),
    ]
    .join("\0")
    .into_bytes()
}

fn capability(c: Capability) -> u8 {
    u8::try_from(capability_code(c)).unwrap_or(u8::MAX)
}

fn core_grant(grant: &Grant) -> core::Grant {
    core::Grant {
        id: grant.id().as_str().as_bytes().to_vec(),
        scope: scope_bytes(grant.scope()),
        scope_realm: grant.scope().realm().as_str().as_bytes().to_vec(),
        resources: grant
            .resources()
            .iter()
            .map(|r| r.as_str().as_bytes().to_vec())
            .collect(),
        capabilities: grant
            .capabilities()
            .iter()
            .map(|c| capability(*c))
            .collect(),
        valid_from: grant.validity().valid_from().get(),
        expires: grant.validity().expires().get(),
    }
}

/// Grants for one realm under one policy.
pub struct Authority {
    inner: core::Authority,
    /// The typed grants, in the core's order (oldest first).
    typed: Vec<Grant>,
    /// The typed clock; always equal to `inner.now`.
    now: EpochSeconds,
}

impl Authority {
    pub fn new(realm: RealmId, policy: PolicyDigest, now: EpochSeconds) -> Self {
        Self {
            inner: core::Authority {
                realm: realm.as_str().as_bytes().to_vec(),
                policy: policy.as_str().as_bytes().to_vec(),
                now: now.get(),
                grants: Vec::new(),
            },
            typed: Vec::new(),
            now,
        }
    }
    pub fn now(&self) -> EpochSeconds {
        self.now
    }
    /// Newest first (Lean's order), including revoked grants.
    pub fn grants(&self) -> Vec<Installed> {
        self.typed
            .iter()
            .zip(&self.inner.grants)
            .rev()
            .map(|(grant, installed)| Installed {
                grant: grant.clone(),
                envelope: BlobDigest::from_digest(installed.envelope),
                revoked: installed.revoked,
            })
            .collect()
    }

    /// Lean `advanceTime`: time never moves backwards.
    pub fn advance(&mut self, time: EpochSeconds) -> Result<(), Refusal> {
        self.inner.advance(time.get()).map_err(refusal)?;
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
        let retained: Vec<[u8; 32]> = retained.iter().map(|d| *d.as_digest()).collect();
        let core_authorization = core::Authorization {
            realm: authorization.realm.as_str().as_bytes().to_vec(),
            policy: authorization.policy.as_str().as_bytes().to_vec(),
            grant: core_grant(&authorization.grant),
            envelope: *authorization.envelope.as_digest(),
        };
        self.inner
            .install(core_authorization, &retained)
            .map_err(refusal)?;
        self.typed.push(authorization.grant);
        Ok(())
    }

    /// Lean `revokeGrant`: marks every grant with this id revoked; an unknown
    /// id changes nothing. Revocation is never refused.
    pub fn revoke(&mut self, id: &GrantId) {
        self.inner.revoke(id.as_str().as_bytes());
    }

    /// Lean `currentGrant`: the newest unrevoked grant for exactly this run
    /// scope that is valid now; a newer grant shadows older ones.
    pub fn current(&self, scope: &Scope) -> Option<&Grant> {
        core::current(&self.inner.grants, &scope_bytes(scope), self.inner.now)
            .and_then(|i| self.typed.get(i))
    }

    /// The grant conjunct of Lean `intentReady`. An empty request is covered;
    /// callers that act on resources must refuse empty sets themselves.
    pub fn covers(&self, scope: &Scope, capability_: Capability, resources: &[ResourceId]) -> bool {
        let resources: Vec<Vec<u8>> = resources
            .iter()
            .map(|r| r.as_str().as_bytes().to_vec())
            .collect();
        core::covers(
            &self.inner.grants,
            &scope_bytes(scope),
            self.inner.now,
            capability(capability_),
            &resources,
        )
    }
}
