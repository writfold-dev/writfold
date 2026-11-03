//! Pure reference operation inputs, not production authentication capabilities.
use crate::{evidence::Context, ids::*, knowledge::KB, language::*, records::record};

record!(Authority, AuthorityView {
    actor: ActorId, target: ClaimRef, intended_use: String, policy: PolicyId,
    statement: Statement, approved: KB, approvals: Vec<Claim>, context: Context,
});
record!(
    Plan,
    PlanView {
        id: OperationId,
        expected_snapshot: SnapshotId,
        candidate: KB,
        authority: Authority,
        claim: ClaimRef,
        witness: Witness,
    }
);
record!(
    /// Reference `VerifiedApproval` input. This value does not authenticate an actor or authorize IO.
    ModelApproval, ModelApprovalView { plan: Plan }
);
record!(JournalEntry, JournalEntryView { plan: Plan });

::vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitOutcome {
    Applied,
    AlreadyApplied,
    InputConflict,
    PredecessorConflict,
    Rejected,
}
}

record!(Application, ApplicationView { kb: KB, journal: Vec<JournalEntry>, outcome: CommitOutcome });

::vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Draft,
    Proposed,
    Approved,
    Applied,
    Verified,
    Rejected,
}
}

record!(Proposal, ProposalView {
    id: ProposalId, revision: ClaimRevision, target: ClaimRef,
    status: ProposalStatus, actor: Option<ActorId>, executor: Option<ActorId>,
    acceptor: Option<ActorId>, gates: Vec<ReceiptId>,
});
record!(SelectionState, SelectionStateView { kb: KB, selected: Vec<ClaimRef> });
record!(SelectionPlan, SelectionPlanView {
    id: OperationId, expected_snapshot: SnapshotId, target_snapshot: SnapshotId,
    new_snapshot: SnapshotId, selected: Vec<ClaimRef>, actor: Option<ActorId>,
    policy: PolicyId, intended_use: String, context: Context,
});
record!(
    /// Reference `VerifiedSelectionApproval` input; never a publication capability.
    ModelSelectionApproval, ModelSelectionApprovalView { plan: SelectionPlan }
);
record!(
    SelectionJournalEntry,
    SelectionJournalEntryView {
        plan: SelectionPlan
    }
);
record!(SelectionResult, SelectionResultView {
    state: SelectionState, journal: Vec<SelectionJournalEntry>, outcome: CommitOutcome,
});

crate::records::scalar_views!(CommitOutcome, ProposalStatus);
