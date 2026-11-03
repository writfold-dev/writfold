//! Source-model admission judgments; none of these values authenticates production authority.
#![cfg(verus_keep_ghost)]
use super::actor_present_spec;
use super::result::WorkflowResultView;
use crate::checker::{retracted_spec, spec::*};
use crate::wellformed::spec::{context_wf_spec, well_formed_spec};
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub open spec fn prefix<T>(base: Seq<T>, candidate: Seq<T>) -> bool {
    base.len() <= candidate.len()
        && forall|i: int| 0 <= i < base.len() ==> base[i] == candidate[i]
}
pub open spec fn preserves_prefix_spec(base: KBView, candidate: KBView) -> bool {
    prefix(base.observations, candidate.observations) && prefix(base.claims, candidate.claims)
        && prefix(base.attestations, candidate.attestations)
        && prefix(base.enumerations, candidate.enumerations)
        && prefix(base.derivations, candidate.derivations)
        && prefix(base.corrections, candidate.corrections) && prefix(base.receipts, candidate.receipts)
}
pub open spec fn kb_at_spec(kb: KBView, snapshot: Seq<char>) -> KBView {
    KBView { snapshot, ..kb }
}
pub open spec fn context_matches_spec(kb: KBView, context: ContextView) -> bool {
    kb.snapshot == context.snapshot && context_wf_spec(context)
}
pub open spec fn added_derivation_spec(kb: KBView, context: ContextView,
    claim: ClaimView, derivation: DerivationView) -> bool {
    claim_matches_spec(claim, derivation.claim)
        && !retracted_spec(kb, context, derivation.claim)
        && check_matches_snapshot_spec(kb, context, kb.snapshot, derivation.witness, claim.statement)
        && usable_for_snapshot_spec(kb, context, kb.snapshot, derivation.claim)
}
pub open spec fn added_claim_spec(kb: KBView, context: ContextView, claim: ClaimView) -> bool {
    exists|i: int| 0 <= i < kb.derivations.len()
        && added_derivation_spec(kb, context, claim, kb.derivations[i])
}
pub open spec fn plan_valid_spec(kb: KBView, context: ContextView, plan: PlanView) -> bool {
    let candidate = plan.candidate;
    let authority = plan.authority;
    context_matches_spec(kb, context) && well_formed_spec(candidate)
        && candidate.snapshot != kb.snapshot && preserves_prefix_spec(kb, candidate)
        && (exists|claim: ClaimView| first_claim_match(candidate.claims, plan.claim, Some(claim))
            && authority.statement == claim.statement
            && check_matches_snapshot_spec(candidate, context, candidate.snapshot, plan.witness, claim.statement))
        && writfold_domain::text::nonblank_spec(authority.actor)
        && authority.target == plan.claim && context.policy.authorized_actors.contains(authority.actor)
        && authority.policy == context.policy.id && authority.intended_use == context.intended_use
        && authority.context == context && authority.approved == candidate
        && authority.approvals == candidate.claims.subrange(kb.claims.len() as int, candidate.claims.len() as int)
        && !retracted_spec(candidate, context, plan.claim)
        && (exists|i: int| 0 <= i < candidate.derivations.len()
            && candidate.derivations[i].claim == plan.claim && candidate.derivations[i].witness == plan.witness)
        && usable_for_snapshot_spec(candidate, context, candidate.snapshot, plan.claim)
        && (forall|i: int| kb.claims.len() <= i < candidate.claims.len()
            ==> added_claim_spec(candidate, context, candidate.claims[i]))
}
pub open spec fn first_plan(journal: Seq<JournalEntryView>, plan: PlanView) -> Option<PlanView> {
    if exists|i: int| 0 <= i < journal.len() && journal[i].plan.id == plan.id {
        let i = choose|i: int| 0 <= i < journal.len() && journal[i].plan.id == plan.id
            && forall|j: int| 0 <= j < i ==> journal[j].plan.id != plan.id;
        Some(journal[i].plan)
    } else { None }
}
pub open spec fn replay_spec<P>(recorded: P, proposed: P) -> CommitOutcome {
    if recorded == proposed { CommitOutcome::AlreadyApplied } else { CommitOutcome::InputConflict }
}
pub open spec fn apply_plan_spec(kb: KBView, context: ContextView, journal: Seq<JournalEntryView>,
    plan: PlanView, approvals: Seq<ModelApprovalView>) -> WorkflowResultView<KBView, JournalEntryView> {
    let outcome = match first_plan(journal, plan) {
        Some(recorded) => replay_spec(recorded, plan),
        None => if plan.expected_snapshot != kb.snapshot { CommitOutcome::PredecessorConflict }
            else if !(exists|i: int| 0 <= i < approvals.len() && approvals[i].plan == plan)
                || !plan_valid_spec(kb, context, plan) { CommitOutcome::Rejected }
            else { CommitOutcome::Applied },
    };
    if outcome == CommitOutcome::Applied {
        WorkflowResultView { state: plan.candidate, journal: seq![JournalEntryView { plan }] + journal,
            outcome, state_borrowed: false, journal_borrowed: false }
    } else {
        WorkflowResultView { state: kb, journal, outcome, state_borrowed: true, journal_borrowed: true }
    }
}
pub open spec fn selected_present(kb: KBView, selected: Seq<ClaimRefView>) -> bool {
    forall|i: int| 0 <= i < selected.len() ==> exists|j: int| 0 <= j < kb.claims.len()
        && claim_matches_spec(kb.claims[j], selected[i])
}
pub open spec fn retained_unique(retained: Seq<KBView>) -> bool {
    forall|i: int, j: int| 0 <= j < i < retained.len() ==> retained[j].snapshot != retained[i].snapshot
}
pub open spec fn selection_target_valid(state: SelectionStateView, plan: SelectionPlanView, target: KBView) -> bool {
    well_formed_spec(target) && preserves_prefix_spec(target, state.kb)
        && plan.selected.len() == target.claims.len()
        && (forall|i: int| 0 <= i < plan.selected.len() ==> claim_matches_spec(target.claims[i], plan.selected[i]))
        && selected_present(state.kb, plan.selected)
}
pub open spec fn selection_plan_valid_spec(state: SelectionStateView, context: ContextView,
    plan: SelectionPlanView, retained: Seq<KBView>) -> bool {
    context_matches_spec(state.kb, context) && well_formed_spec(state.kb)
        && state.selected.no_duplicates() && selected_present(state.kb, state.selected)
        && plan.new_snapshot != state.kb.snapshot && retained_unique(retained)
        && (forall|i: int| 0 <= i < retained.len() ==> retained[i].snapshot != plan.new_snapshot)
        && actor_present_spec(plan.actor)
        && (match plan.actor { Some(actor) => context.policy.authorized_actors.contains(actor), None => false })
        && plan.context == context && plan.policy == context.policy.id && plan.intended_use == context.intended_use
        && (exists|i: int| 0 <= i < retained.len() && retained[i].snapshot == plan.target_snapshot
            && selection_target_valid(state, plan, retained[i]))
}
pub open spec fn first_selection_plan(journal: Seq<SelectionJournalEntryView>, plan: SelectionPlanView)
    -> Option<SelectionPlanView> {
    if exists|i: int| 0 <= i < journal.len() && journal[i].plan.id == plan.id {
        let i = choose|i: int| 0 <= i < journal.len() && journal[i].plan.id == plan.id
            && forall|j: int| 0 <= j < i ==> journal[j].plan.id != plan.id;
        Some(journal[i].plan)
    } else { None }
}
pub open spec fn apply_selection_spec(state: SelectionStateView, context: ContextView,
    journal: Seq<SelectionJournalEntryView>, plan: SelectionPlanView,
    approvals: Seq<ModelSelectionApprovalView>, retained: Seq<KBView>)
    -> WorkflowResultView<SelectionStateView, SelectionJournalEntryView> {
    let outcome = match first_selection_plan(journal, plan) {
        Some(recorded) => replay_spec(recorded, plan),
        None => if plan.expected_snapshot != state.kb.snapshot { CommitOutcome::PredecessorConflict }
            else if !(exists|i: int| 0 <= i < approvals.len() && approvals[i].plan == plan)
                || !selection_plan_valid_spec(state, context, plan, retained) { CommitOutcome::Rejected }
            else { CommitOutcome::Applied },
    };
    if outcome == CommitOutcome::Applied {
        WorkflowResultView { state: SelectionStateView { kb: kb_at_spec(state.kb, plan.new_snapshot), selected: plan.selected },
            journal: seq![SelectionJournalEntryView { plan }] + journal, outcome,
            state_borrowed: false, journal_borrowed: false }
    } else {
        WorkflowResultView { state, journal, outcome, state_borrowed: true, journal_borrowed: true }
    }
}
pub open spec fn transition_edge(current: ProposalStatus, target: ProposalStatus) -> bool {
    use ProposalStatus::*;
    matches!((current, target), (Draft, Proposed) | (Proposed, Approved) | (Proposed, Rejected)
        | (Approved, Applied) | (Approved, Rejected) | (Applied, Verified))
}
pub open spec fn gate_reusable(kb: KBView, context: ContextView, target: ClaimRefView, gate: Seq<char>) -> bool {
    exists|i: int| 0 <= i < kb.receipts.len() && kb.receipts[i].id == gate
        && (forall|j: int| 0 <= j < i ==> kb.receipts[j].id != gate)
        && reusable_spec(kb, context, target, kb.receipts[i])
}
pub open spec fn transition_spec(kb: KBView, context: ContextView, proposal: ProposalView, target: ProposalStatus) -> bool {
    use ProposalStatus::*;
    transition_edge(proposal.status, target) && actor_present_spec(proposal.actor)
        && usable_for_spec(kb, context, proposal.target)
        && (forall|i: int| 0 <= i < proposal.gates.len() ==> gate_reusable(kb, context, proposal.target, proposal.gates[i]))
        && (matches!(target, Approved | Applied | Verified) ==> proposal.gates.len() > 0)
        && (target == Applied ==> actor_present_spec(proposal.executor))
        && (target == Verified ==> actor_present_spec(proposal.executor)
            && actor_present_spec(proposal.acceptor) && proposal.executor != proposal.acceptor)
}
pub open spec fn conclude_spec(investigation: InvestigationView) -> bool {
    writfold_domain::text::nonblank_spec(investigation.checked_against)
        && writfold_domain::text::nonblank_spec(investigation.scope_and_limits)
        && (forall|i: int| 0 <= i < investigation.hypotheses.len()
            ==> writfold_domain::text::nonblank_spec(investigation.hypotheses[i].text)
                && investigation.hypotheses[i].status != HypothesisStatus::Open)
}
}
