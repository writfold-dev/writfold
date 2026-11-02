//! Pure reference operation inputs, not production authentication capabilities.
use crate::{evidence::Context, ids::*, knowledge::KB, language::*, records::record};

record!(Authority {
    actor: ActorId, target: ClaimRef, intended_use: String, policy: PolicyId,
    statement: Statement, approved: KB, approvals: Vec<Claim>, context: Context,
});
record!(Plan {
    id: OperationId,
    expected_snapshot: SnapshotId,
    candidate: KB,
    authority: Authority,
    claim: ClaimRef,
    witness: Witness,
});
record!(
    /// Reference `VerifiedApproval` input. This value does not authenticate an actor or authorize IO.
    ModelApproval { plan: Plan }
);
record!(JournalEntry { plan: Plan });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitOutcome {
    Applied,
    AlreadyApplied,
    InputConflict,
    PredecessorConflict,
    Rejected,
}

record!(Application { kb: KB, journal: Vec<JournalEntry>, outcome: CommitOutcome });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Draft,
    Proposed,
    Approved,
    Applied,
    Verified,
    Rejected,
}

record!(Proposal {
    id: ProposalId, revision: ClaimRevision, target: ClaimRef,
    status: ProposalStatus, actor: Option<ActorId>, executor: Option<ActorId>,
    acceptor: Option<ActorId>, gates: Vec<ReceiptId>,
});
record!(SelectionState { kb: KB, selected: Vec<ClaimRef> });
record!(SelectionPlan {
    id: OperationId, expected_snapshot: SnapshotId, target_snapshot: SnapshotId,
    new_snapshot: SnapshotId, selected: Vec<ClaimRef>, actor: Option<ActorId>,
    policy: PolicyId, intended_use: String, context: Context,
});
record!(
    /// Reference `VerifiedSelectionApproval` input; never a publication capability.
    ModelSelectionApproval { plan: SelectionPlan }
);
record!(SelectionJournalEntry {
    plan: SelectionPlan
});
record!(SelectionResult {
    state: SelectionState, journal: Vec<SelectionJournalEntry>, outcome: CommitOutcome,
});
