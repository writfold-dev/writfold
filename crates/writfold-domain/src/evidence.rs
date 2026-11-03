//! Acquisition, policy and time remain separate from logical support.
use crate::{Integer, ids::*, language::*, records::record};
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtlTag {
    ExpireAfter,
    NoExpiry,
    Missing,
}

#[derive(Debug)]
pub enum Ttl {
    ExpireAfter(Integer),
    NoExpiry,
    Missing,
}

#[cfg(verus_keep_ghost)]
pub ghost enum TtlView {
    ExpireAfter(int),
    NoExpiry,
    Missing,
}

#[cfg(verus_keep_ghost)]
impl View for Ttl {
    type V = TtlView;
    open spec fn view(&self) -> TtlView {
        match self {
            Self::ExpireAfter(secs) => TtlView::ExpireAfter(secs@),
            Self::NoExpiry => TtlView::NoExpiry,
            Self::Missing => TtlView::Missing,
        }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Ttl {
    type V = TtlView;
    open spec fn deep_view(&self) -> TtlView { self@ }
}
impl Clone for Ttl {
    fn clone(&self) -> (cloned: Self)
        ensures cloned@ == self@,
    {
        match self {
            Self::ExpireAfter(secs) => Self::ExpireAfter(secs.clone()),
            Self::NoExpiry => Self::NoExpiry,
            Self::Missing => Self::Missing,
        }
    }
}
#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Ttl {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}
impl PartialEq for Ttl {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (self@ == other@),
    {
        match (self, other) {
            (Self::ExpireAfter(left), Self::ExpireAfter(right)) => left == right,
            (Self::NoExpiry, Self::NoExpiry) | (Self::Missing, Self::Missing) => true,
            _ => false,
        }
    }
}
impl Eq for Ttl {}

impl Ttl {
    pub fn tag(&self) -> (result: TtlTag)
        ensures result == match self {
            Self::ExpireAfter(_) => TtlTag::ExpireAfter,
            Self::NoExpiry => TtlTag::NoExpiry,
            Self::Missing => TtlTag::Missing,
        },
    {
        match self {
            Self::ExpireAfter(_) => TtlTag::ExpireAfter,
            Self::NoExpiry => TtlTag::NoExpiry,
            Self::Missing => TtlTag::Missing,
        }
    }
    pub fn secs(&self) -> (result: Option<&Integer>)
        ensures match self {
            Self::ExpireAfter(secs) => result == Some(secs),
            _ => result is None,
        },
    {
        match self {
            Self::ExpireAfter(secs) => Some(secs),
            _ => None,
        }
    }
}

pub const TTL_NO_EXPIRY: Ttl = Ttl::NoExpiry;
pub const TTL_MISSING: Ttl = Ttl::Missing;
pub fn ttl_expire_after(secs: Integer) -> (result: Ttl)
    ensures result == Ttl::ExpireAfter(secs),
{
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

} // verus!

record!(Observation, ObservationView {
    id: ObservationId,
    source: SourceId,
    revision: SourceRevision,
    content: ContentId,
    observed_at: Integer,
    outcome: ObsOutcome,
});
record!(Attestation, AttestationView {
    id: AttestationId,
    claim: ClaimRef,
    observation: ObservationId,
    statement: Statement,
    authority: ActorId,
});
record!(Enumeration, EnumerationView {
    id: EnumerationId, source: SourceId, revision: SourceRevision,
    query: String, scope: Scope, observation: ObservationId,
    members: Vec<String> => crate::equality::strings_equal,
    universe: Vec<String> => crate::equality::strings_equal, method: String,
});

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrectionKind {
    Retract,
    Dispute,
    Supersede,
}
}

record!(Correction, CorrectionView {
    id: CorrectionId, kind: CorrectionKind, target: ClaimRef,
    replacement: Option<ClaimRef>, recorded_at: Integer,
});
record!(Policy, PolicyView {
    id: PolicyId, require_fresh: bool, require_validation: bool,
    require_uncontested: bool, ttl: Ttl, authorized_actors: Vec<ActorId>,
});
record!(SourceSelection, SourceSelectionView {
    source: SourceId,
    revision: SourceRevision,
    content: ContentId
});
record!(Context, ContextView {
    snapshot: SnapshotId, semantic_version: String, ruleset: RuleVersion,
    checker: CheckerId, policy: Policy, now: Integer, target: Scope,
    intended_use: String, sources: Vec<SourceSelection>,
});

crate::records::scalar_views!(TtlTag, Freshness, ObsOutcome, CorrectionKind);
