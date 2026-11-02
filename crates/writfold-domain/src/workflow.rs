//! Workflow schema 3 retains ordered portfolios and isolated named contexts.
use crate::{
    Integer, admission::CommitOutcome, authored::AuthoredProposal, evidence::Context, ids::*,
    knowledge::KB, language::*, records::record,
};

pub const WORKFLOW_SCHEMA_VERSION: u32 = 3;

record!(NamedContext {
    id: ContextId,
    context: Context
});
record!(ClaimAssignment {
    claim: ClaimRef,
    context: ContextId,
    witness: Witness
});
record!(PortfolioState { kb: KB, selected: Vec<ClaimRef>, contexts: Vec<NamedContext> });
record!(PortfolioPlan {
    id: OperationId, expected_snapshot: SnapshotId, candidate: KB, actor: ActorId,
    contexts: Vec<NamedContext>, assignments: Vec<ClaimAssignment>, retired_contexts: Vec<ContextId>,
});
record!(
    /// Reference `VerifiedPortfolioApproval` input; not authenticated production authority.
    ModelPortfolioApproval { plan: PortfolioPlan }
);
record!(PortfolioJournalEntry {
    plan: PortfolioPlan
});
record!(PortfolioResult {
    state: PortfolioState, journal: Vec<PortfolioJournalEntry>, outcome: CommitOutcome,
});
record!(PortfolioSelectionPlan {
    id: OperationId, expected_snapshot: SnapshotId, retained_snapshot: SnapshotId,
    new_snapshot: SnapshotId, actor: ActorId, contexts: Vec<NamedContext>, selected: Vec<ClaimRef>,
});
record!(
    /// Reference `VerifiedPortfolioSelectionApproval` input; not a publication capability.
    ModelPortfolioSelectionApproval { plan: PortfolioSelectionPlan }
);
record!(PortfolioSelectionJournalEntry {
    plan: PortfolioSelectionPlan
});
record!(PortfolioSelectionResult {
    state: PortfolioState, journal: Vec<PortfolioSelectionJournalEntry>, outcome: CommitOutcome,
});
record!(ImplementationBinding {
    design_revision: DesignRevision,
    source: SourceId,
    target_revision: TargetRevision,
    actor: ActorId,
    observed_at: Integer,
});
record!(DesignProposal {
    authored: AuthoredProposal, target_source: SourceId, binding: Option<ImplementationBinding>,
});
record!(DesignBindingCommand {
    expected: DesignProposal,
    binding: ImplementationBinding
});
record!(DesignReopenCommand {
    expected: DesignProposal,
    actor: ActorId,
    reason: String
});
