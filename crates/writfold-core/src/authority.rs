//! Installed operator grants and the current-grant check, mirroring Lean
//! `environment` `installGrant` / `revokeGrant` / `advanceTime`,
//! `currentGrant` and the grant conjunct of `intentReady`.
//!
//! Grants are kept oldest first and searched newest first, which is Lean's
//! prepend-then-`find?` order. An `Authorization` is a qualified input: the
//! caller has verified the operator's signature over the exact grant.
use crate::bytes::{contains_digest, same};
use alloc::vec::Vec;

pub struct Grant {
    pub id: Vec<u8>,
    /// Canonical bytes of the run scope (realm, run, repository, worktree, binding).
    pub scope: Vec<u8>,
    /// The realm named inside the scope.
    pub scope_realm: Vec<u8>,
    pub resources: Vec<Vec<u8>>,
    pub capabilities: Vec<u8>,
    pub valid_from: u64,
    pub expires: u64,
}

pub struct Authorization {
    pub realm: Vec<u8>,
    pub policy: Vec<u8>,
    pub grant: Grant,
    pub envelope: [u8; 32],
}

pub struct Installed {
    pub grant: Grant,
    pub envelope: [u8; 32],
    pub revoked: bool,
}

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

pub struct Authority {
    pub realm: Vec<u8>,
    pub policy: Vec<u8>,
    pub now: u64,
    pub grants: Vec<Installed>,
}

/// Whether any installed grant, revoked or not, has this id.
pub fn has_id(grants: &[Installed], id: &[u8]) -> bool {
    let mut i = 0;
    while i < grants.len() {
        if same(&grants[i].grant.id, id) {
            return true;
        }
        i += 1;
    }
    false
}

/// Lean's installation guards, in order; the first failing guard names the refusal.
pub fn install_check(
    realm: &[u8],
    policy: &[u8],
    now: u64,
    grants: &[Installed],
    authorization: &Authorization,
    retained: &[[u8; 32]],
) -> Result<(), Refusal> {
    let grant = &authorization.grant;
    if !same(&authorization.realm, realm) {
        return Err(Refusal::ForeignRealm);
    }
    if !same(&grant.scope_realm, realm) {
        return Err(Refusal::ForeignBindingRealm);
    }
    if !same(&authorization.policy, policy) {
        return Err(Refusal::StalePolicy);
    }
    if !contains_digest(retained, &authorization.envelope) {
        return Err(Refusal::UnretainedEnvelope);
    }
    if has_id(grants, &grant.id) {
        return Err(Refusal::DuplicateGrant);
    }
    if now < grant.valid_from {
        return Err(Refusal::NotYetValid);
    }
    if grant.expires <= now {
        return Err(Refusal::Expired);
    }
    Ok(())
}

/// Whether installed grant `g` is the current grant for `scope` at `now`.
pub fn live(g: &Installed, scope: &[u8], now: u64) -> bool {
    same(&g.grant.scope, scope) && !g.revoked && g.grant.valid_from <= now && now < g.grant.expires
}

/// Lean `currentGrant`: the newest live grant for exactly this scope. A newer
/// grant shadows older ones even when it covers less; there is no fallback.
pub fn current(grants: &[Installed], scope: &[u8], now: u64) -> Option<usize> {
    let mut i = grants.len();
    while i > 0 {
        if live(&grants[i - 1], scope, now) {
            return Some(i - 1);
        }
        i -= 1;
    }
    None
}

/// Whether `resources` are all among `granted`.
pub fn resources_within(granted: &[Vec<u8>], resources: &[Vec<u8>]) -> bool {
    let mut i = 0;
    while i < resources.len() {
        if !any_same(granted, &resources[i]) {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether `item` is among `items`.
pub fn any_same(items: &[Vec<u8>], item: &[u8]) -> bool {
    let mut j = 0;
    while j < items.len() {
        if same(&items[j], item) {
            return true;
        }
        j += 1;
    }
    false
}

/// Whether `capability` is among `capabilities`.
pub fn has_capability(capabilities: &[u8], capability: u8) -> bool {
    let mut i = 0;
    while i < capabilities.len() {
        if capabilities[i] == capability {
            return true;
        }
        i += 1;
    }
    false
}

/// The grant conjunct of Lean `intentReady`.
pub fn covers(
    grants: &[Installed],
    scope: &[u8],
    now: u64,
    capability: u8,
    resources: &[Vec<u8>],
) -> bool {
    match current(grants, scope, now) {
        None => false,
        Some(i) => {
            has_capability(&grants[i].grant.capabilities, capability)
                && resources_within(&grants[i].grant.resources, resources)
        }
    }
}

impl Authority {
    /// Lean `installGrant`; refusal changes nothing.
    pub fn install(
        &mut self,
        authorization: Authorization,
        retained: &[[u8; 32]],
    ) -> Result<(), Refusal> {
        install_check(
            &self.realm,
            &self.policy,
            self.now,
            &self.grants,
            &authorization,
            retained,
        )?;
        self.grants.push(Installed {
            grant: authorization.grant,
            envelope: authorization.envelope,
            revoked: false,
        });
        Ok(())
    }

    /// Lean `revokeGrant`: every grant with this id is revoked; never refused.
    pub fn revoke(&mut self, id: &[u8]) {
        let mut i = 0;
        while i < self.grants.len() {
            if same(&self.grants[i].grant.id, id) {
                self.grants[i].revoked = true;
            }
            i += 1;
        }
    }

    /// Lean `advanceTime`: the clock never moves backwards.
    pub fn advance(&mut self, time: u64) -> Result<(), Refusal> {
        if time < self.now {
            return Err(Refusal::ClockRewind);
        }
        self.now = time;
        Ok(())
    }
}
