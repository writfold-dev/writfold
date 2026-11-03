//! Workflow schema 3 retains ordered portfolios and isolated named contexts.
use crate::{
    Integer, admission::CommitOutcome, authored::AuthoredProposal, evidence::Context, ids::*,
    knowledge::KB, language::*, records::record,
};

pub const WORKFLOW_SCHEMA_VERSION: u32 = 3;

record!(NamedContext, NamedContextView {
    id: ContextId,
    context: Context
});
record!(ClaimAssignment, ClaimAssignmentView {
    claim: ClaimRef,
    context: ContextId,
    witness: Witness
});
record!(PortfolioState, PortfolioStateView { kb: KB, selected: Vec<ClaimRef>, contexts: Vec<NamedContext> });
record!(PortfolioPlan, PortfolioPlanView {
    id: OperationId, expected_snapshot: SnapshotId, candidate: KB, actor: ActorId,
    contexts: Vec<NamedContext>, assignments: Vec<ClaimAssignment>, retired_contexts: Vec<ContextId>,
});
record!(
    /// Reference `VerifiedPortfolioApproval` input; not authenticated production authority.
    ModelPortfolioApproval, ModelPortfolioApprovalView { plan: PortfolioPlan }
);
record!(PortfolioJournalEntry, PortfolioJournalEntryView {
    plan: PortfolioPlan
});
record!(PortfolioResult, PortfolioResultView {
    state: PortfolioState, journal: Vec<PortfolioJournalEntry>, outcome: CommitOutcome,
});
record!(PortfolioSelectionPlan, PortfolioSelectionPlanView {
    id: OperationId, expected_snapshot: SnapshotId, retained_snapshot: SnapshotId,
    new_snapshot: SnapshotId, actor: ActorId, contexts: Vec<NamedContext>, selected: Vec<ClaimRef>,
});
record!(
    /// Reference `VerifiedPortfolioSelectionApproval` input; not a publication capability.
    ModelPortfolioSelectionApproval, ModelPortfolioSelectionApprovalView { plan: PortfolioSelectionPlan }
);
record!(PortfolioSelectionJournalEntry, PortfolioSelectionJournalEntryView {
    plan: PortfolioSelectionPlan
});
record!(PortfolioSelectionResult, PortfolioSelectionResultView {
    state: PortfolioState, journal: Vec<PortfolioSelectionJournalEntry>, outcome: CommitOutcome,
});
record!(ImplementationBinding, ImplementationBindingView {
    design_revision: DesignRevision,
    source: SourceId,
    target_revision: TargetRevision,
    actor: ActorId,
    observed_at: Integer,
});
record!(DesignProposal, DesignProposalView {
    authored: AuthoredProposal, target_source: SourceId, binding: Option<ImplementationBinding>,
});
record!(DesignBindingCommand, DesignBindingCommandView {
    expected: DesignProposal,
    binding: ImplementationBinding
});
record!(DesignReopenCommand, DesignReopenCommandView {
    expected: DesignProposal,
    actor: ActorId,
    reason: String
});
