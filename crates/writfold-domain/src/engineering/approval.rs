//! Immutable operator approval of one exact prepared edit.
//!
//! Construction checks only internal structure (a nonempty validity interval).
//! It is not a signature, a current grant, a live resource generation, or
//! permission to perform native IO; verification against a supplied policy and
//! current expected values lives at the wire boundary.
use super::{BlobDigest, ProcedureRevision};
use crate::worktree::{PreparedEditDigest, Scope};
use crate::{AuthorityKeyId, EpochSeconds, PolicyDigest, ResourceGeneration, ResourceId};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidApproval;
impl fmt::Display for InvalidApproval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid prepared-edit approval")
    }
}
impl Error for InvalidApproval {}

/// Half-open interval `[valid_from, expires)`; never empty, never renewed in place.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Validity {
    valid_from: EpochSeconds,
    expires: EpochSeconds,
}
impl Validity {
    pub fn new(valid_from: EpochSeconds, expires: EpochSeconds) -> Result<Self, InvalidApproval> {
        if valid_from >= expires {
            return Err(InvalidApproval);
        }
        Ok(Self {
            valid_from,
            expires,
        })
    }
    pub fn valid_from(self) -> EpochSeconds {
        self.valid_from
    }
    pub fn expires(self) -> EpochSeconds {
        self.expires
    }
    pub fn contains(self, now: EpochSeconds) -> bool {
        self.valid_from <= now && now < self.expires
    }
}

/// The policy key and exact policy identity the signer claims; verification
/// must confirm both against the supplied current policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signer {
    key: AuthorityKeyId,
    policy: PolicyDigest,
}
impl Signer {
    pub fn new(key: AuthorityKeyId, policy: PolicyDigest) -> Self {
        Self { key, policy }
    }
    pub fn key(&self) -> &AuthorityKeyId {
        &self.key
    }
    pub fn policy(&self) -> &PolicyDigest {
        &self.policy
    }
}

/// Everything an approval binds. `report` names the exact retained Weft report
/// bytes by blob digest; it says nothing about the report's truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditApproval {
    scope: Scope,
    prepared: PreparedEditDigest,
    report: BlobDigest,
    procedure: ProcedureRevision,
    resource: ResourceId,
    generation: ResourceGeneration,
    validity: Validity,
    signer: Signer,
}
impl EditApproval {
    #[allow(
        clippy::too_many_arguments,
        reason = "an approval binds eight distinct exact values"
    )]
    pub fn new(
        scope: Scope,
        prepared: PreparedEditDigest,
        report: BlobDigest,
        procedure: ProcedureRevision,
        resource: ResourceId,
        generation: ResourceGeneration,
        validity: Validity,
        signer: Signer,
    ) -> Self {
        Self {
            scope,
            prepared,
            report,
            procedure,
            resource,
            generation,
            validity,
            signer,
        }
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn prepared(&self) -> PreparedEditDigest {
        self.prepared
    }
    pub fn report(&self) -> BlobDigest {
        self.report
    }
    pub fn procedure(&self) -> ProcedureRevision {
        self.procedure
    }
    pub fn resource(&self) -> &ResourceId {
        &self.resource
    }
    pub fn generation(&self) -> ResourceGeneration {
        self.generation
    }
    pub fn validity(&self) -> Validity {
        self.validity
    }
    pub fn signer(&self) -> &Signer {
        &self.signer
    }
}
