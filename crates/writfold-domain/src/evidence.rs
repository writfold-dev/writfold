//! Acquisition, policy and time remain separate from logical support.
use crate::{Integer, ids::*, language::*, records::record};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtlTag {
    ExpireAfter,
    NoExpiry,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ttl {
    ExpireAfter(Integer),
    NoExpiry,
    Missing,
}

impl Ttl {
    pub fn tag(&self) -> TtlTag {
        match self {
            Self::ExpireAfter(_) => TtlTag::ExpireAfter,
            Self::NoExpiry => TtlTag::NoExpiry,
            Self::Missing => TtlTag::Missing,
        }
    }
    pub fn secs(&self) -> Option<&Integer> {
        match self {
            Self::ExpireAfter(secs) => Some(secs),
            _ => None,
        }
    }
}

pub const TTL_NO_EXPIRY: Ttl = Ttl::NoExpiry;
pub const TTL_MISSING: Ttl = Ttl::Missing;
pub fn ttl_expire_after(secs: Integer) -> Ttl {
    Ttl::ExpireAfter(secs)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObsOutcome {
    Ok,
    Timeout,
    Partial,
    Truncated,
    Denied,
    Unavailable,
}

record!(Observation {
    id: ObservationId,
    source: SourceId,
    revision: SourceRevision,
    content: ContentId,
    observed_at: Integer,
    outcome: ObsOutcome,
});
record!(Attestation {
    id: AttestationId,
    claim: ClaimRef,
    observation: ObservationId,
    statement: Statement,
    authority: ActorId,
});
record!(Enumeration {
    id: EnumerationId, source: SourceId, revision: SourceRevision,
    query: String, scope: Scope, observation: ObservationId,
    members: Vec<String>, universe: Vec<String>, method: String,
});

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrectionKind {
    Retract,
    Dispute,
    Supersede,
}

record!(Correction {
    id: CorrectionId, kind: CorrectionKind, target: ClaimRef,
    replacement: Option<ClaimRef>, recorded_at: Integer,
});
record!(Policy {
    id: PolicyId, require_fresh: bool, require_validation: bool,
    require_uncontested: bool, ttl: Ttl, authorized_actors: Vec<ActorId>,
});
record!(SourceSelection {
    source: SourceId,
    revision: SourceRevision,
    content: ContentId
});
record!(Context {
    snapshot: SnapshotId, semantic_version: String, ruleset: RuleVersion,
    checker: CheckerId, policy: Policy, now: Integer, target: Scope,
    intended_use: String, sources: Vec<SourceSelection>,
});
