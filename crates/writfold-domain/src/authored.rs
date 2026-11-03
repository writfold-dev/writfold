//! Authored evidence and lifecycle inputs; these are not release approvals.
use crate::{Integer, ids::*, records::record};

::vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HypothesisStatus {
    Supported,
    Refuted,
    Open,
    Unknown,
}
}
record!(
    Hypothesis,
    HypothesisView {
        text: String,
        status: HypothesisStatus
    }
);
record!(Investigation, InvestigationView {
    checked_against: String, scope_and_limits: String, hypotheses: Vec<Hypothesis>,
});

::vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthoredStatus {
    Draft,
    Proposed,
    Accepted,
    Rejected,
    Landing,
    Applied,
    Verified,
    Regressed,
    Deferred,
    Superseded,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthoredProfile {
    Modern,
    LegacyLimited,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateWhen {
    Land,
    Deploy,
    Use,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateOutcome {
    Pass,
    Fail,
    Unknown,
}
}

record!(AuthoredGateDefinition, AuthoredGateDefinitionView {
    id: GateId, claim: String, check: String, when: GateWhen,
    by: Option<ActorId>, checker: Option<CheckerId>,
});
record!(AuthoredGateResult, AuthoredGateResultView {
    definition: AuthoredGateDefinition, target_revision: Option<TargetRevision>,
    checker: Option<CheckerId>, result: GateOutcome, observed_at: Integer,
    expires_at: Integer, proposal_revision: Option<ProposalMaterialRevision>,
    environment: String, scope: String,
    evidence_refs: Vec<String> => crate::equality::strings_equal, actor: Option<ActorId>,
});
record!(AuthoredReplacement, AuthoredReplacementView { id: ProposalId, superseded_by: Option<ProposalId> });
record!(
    /// Exact source-adapter resolution; its presence does not authenticate approval.
    AuthoredLanded, AuthoredLandedView { reference: String, target_revision: TargetRevision }
);
record!(AuthoredProposal, AuthoredProposalView {
    status: AuthoredStatus, profile: AuthoredProfile, research_present: bool,
    alternatives_count: Integer, target_revision: Option<TargetRevision>,
    gate_definitions: Vec<AuthoredGateDefinition>, gate_results: Vec<AuthoredGateResult>,
    actor: Option<ActorId>, acceptor: Option<ActorId>, executor: Option<ActorId>,
    verifier: Option<ActorId>, landed_refs: Vec<String> => crate::equality::strings_equal,
    regression_note: String,
    id: ProposalId, superseded_by: Option<ProposalId>, proposals: Vec<AuthoredReplacement>,
    accepted_revision: Option<ProposalMaterialRevision>,
    proposal_revision: Option<ProposalMaterialRevision>, environment: String,
    scope: String, resolved_landed: Vec<AuthoredLanded>,
});

crate::records::scalar_views!(
    HypothesisStatus,
    AuthoredStatus,
    AuthoredProfile,
    GateWhen,
    GateOutcome
);
