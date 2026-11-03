//! Ordered portfolio judgments over the existing checker; no authority is pooled.
#![cfg(verus_keep_ghost)]
use crate::checker::spec::*;
use crate::wellformed::spec::{context_wf_spec, well_formed_spec};
use crate::workflow::admission_model::*;
use crate::workflow::result::WorkflowResultView;
use vstd::prelude::*;
use writfold_domain::*;

verus! {
#[verifier::opaque]
pub open spec fn first_context(contexts: Seq<NamedContextView>, id: Seq<char>) -> Option<NamedContextView> {
    if exists|i: int| 0 <= i < contexts.len() && #[trigger] contexts[i].id == id {
        let i = choose|i: int| 0 <= i < contexts.len() && #[trigger] contexts[i].id == id
            && forall|j: int| 0 <= j < i ==> #[trigger] contexts[j].id != id;
        Some(contexts[i])
    } else { None }
}
pub open spec fn context_family_valid_spec(kb: KBView, contexts: Seq<NamedContextView>) -> bool {
    (forall|i: int, j: int| 0 <= j < i < contexts.len() ==> contexts[j].id != contexts[i].id)
        && forall|i: int| 0 <= i < contexts.len() ==> context_matches_spec(kb, contexts[i].context)
}
pub open spec fn portfolio_state_valid_spec(state: PortfolioStateView) -> bool {
    well_formed_spec(state.kb) && state.selected.no_duplicates()
        && selected_present(state.kb, state.selected) && context_family_valid_spec(state.kb, state.contexts)
        && (state.contexts.len() > 0 || state.kb.claims.len() == 0)
}
pub open spec fn retired_context_ids_spec(before: Seq<NamedContextView>, after: Seq<NamedContextView>)
    -> Seq<Seq<char>>
    decreases before.len(),
{
    if before.len() == 0 { seq![] } else {
        retired_context_ids_spec(before.drop_last(), after)
            + if first_context(after, before.last().id).is_none() { seq![before.last().id] } else { seq![] }
    }
}
pub open spec fn retained_context_valid(old: NamedContextView, after: Seq<NamedContextView>) -> bool {
    match first_context(after, old.id) {
        Some(current) => current.context.target == old.context.target
            && current.context.intended_use == old.context.intended_use,
        None => true,
    }
}
pub open spec fn context_evolution_valid_spec(before: Seq<NamedContextView>, after: Seq<NamedContextView>,
    retired: Seq<Seq<char>>) -> bool {
    (forall|i: int, j: int| 0 <= j < i < retired.len() ==> retired[j] != retired[i])
        && retired == retired_context_ids_spec(before, after)
        && forall|i: int| 0 <= i < before.len() ==> #[trigger] retained_context_valid(before[i], after)
}
pub open spec fn assignment_valid_spec(kb: KBView, contexts: Seq<NamedContextView>, a: ClaimAssignmentView) -> bool {
    match first_context(contexts, a.context) {
        None => false,
        Some(named) => (exists|claim: ClaimView| #[trigger] first_claim_match(kb.claims, a.claim, Some(claim))
            && check_matches_spec(kb, named.context, a.witness, claim.statement))
            && (exists|i: int| 0 <= i < kb.derivations.len()
                && #[trigger] kb.derivations[i].claim == a.claim && kb.derivations[i].witness == a.witness)
            && usable_for_spec(kb, named.context, a.claim),
    }
}
pub open spec fn added_refs_spec(state: PortfolioStateView, plan: PortfolioPlanView) -> Seq<ClaimRefView> {
    plan.candidate.claims.skip(if state.kb.claims.len() < plan.candidate.claims.len() {
        state.kb.claims.len() as int
    } else { plan.candidate.claims.len() as int }).map_values(
        |claim: ClaimView| ClaimRefView { id: claim.id, rev: claim.revision })
}
pub open spec fn admitted_portfolio_spec(state: PortfolioStateView, plan: PortfolioPlanView) -> PortfolioStateView {
    PortfolioStateView { kb: plan.candidate, selected: state.selected + added_refs_spec(state, plan), contexts: plan.contexts }
}
pub open spec fn actors_authorized(contexts: Seq<NamedContextView>, actor: Seq<char>) -> bool {
    forall|i: int| 0 <= i < contexts.len() ==> contexts[i].context.policy.authorized_actors.contains(actor)
}
pub open spec fn assignments_exact(state: PortfolioStateView, plan: PortfolioPlanView) -> bool {
    plan.assignments.len() == plan.candidate.claims.len() - state.kb.claims.len()
        && forall|i: int| 0 <= i < plan.assignments.len() ==>
            claim_matches_spec(plan.candidate.claims[state.kb.claims.len() + i], plan.assignments[i].claim)
            && assignment_valid_spec(plan.candidate, plan.contexts, plan.assignments[i])
}
pub open spec fn portfolio_plan_valid_spec(state: PortfolioStateView, plan: PortfolioPlanView) -> bool {
    portfolio_state_valid_spec(state) && plan.candidate.snapshot != state.kb.snapshot
        && preserves_prefix_spec(state.kb, plan.candidate)
        && writfold_domain::text::nonblank_spec(plan.actor) && plan.contexts.len() > 0
        && actors_authorized(plan.contexts, plan.actor)
        && context_evolution_valid_spec(state.contexts, plan.contexts, plan.retired_contexts)
        && assignments_exact(state, plan) && portfolio_state_valid_spec(admitted_portfolio_spec(state, plan))
}
pub open spec fn context_fixed_fields_equal_spec(left: ContextView, right: ContextView) -> bool {
    left.semantic_version == right.semantic_version && left.ruleset == right.ruleset
        && left.checker == right.checker && left.policy == right.policy
        && left.intended_use == right.intended_use && left.sources == right.sources
}
pub open spec fn query_context_allowed_spec(state: PortfolioStateView, id: Seq<char>, query: ContextView) -> bool {
    context_family_valid_spec(state.kb, state.contexts) && context_matches_spec(state.kb, query)
        && match first_context(state.contexts, id) {
            Some(approved) => query.snapshot == approved.context.snapshot
                && context_fixed_fields_equal_spec(query, approved.context)
                && narrower_spec(query.target, approved.context.target) && approved.context.now <= query.now,
            None => false,
        }
}
pub open spec fn selected_portfolio_spec(state: PortfolioStateView, plan: PortfolioSelectionPlanView) -> PortfolioStateView {
    PortfolioStateView { kb: kb_at_spec(state.kb, plan.new_snapshot), selected: plan.selected, contexts: plan.contexts }
}
pub open spec fn selection_context_valid(before: Seq<NamedContextView>, current: NamedContextView,
    snapshot: Seq<char>) -> bool {
    match first_context(before, current.id) {
        Some(old) => current.context.snapshot == snapshot
            && context_fixed_fields_equal_spec(current.context, old.context)
            && current.context.target == old.context.target && old.context.now <= current.context.now,
        None => false,
    }
}
pub open spec fn selection_contexts_valid_spec(before: Seq<NamedContextView>, after: Seq<NamedContextView>,
    snapshot: Seq<char>) -> bool {
    before.len() == after.len() && (forall|i: int| 0 <= i < before.len() ==> before[i].id == after[i].id)
        && forall|i: int| 0 <= i < after.len() ==> #[trigger] selection_context_valid(before, after[i], snapshot)
}
pub open spec fn retained_valid_spec(retained: Seq<PortfolioStateView>, snapshot: Seq<char>) -> bool {
    (forall|i: int| 0 <= i < retained.len() ==> retained[i].kb.snapshot != snapshot)
        && forall|i: int, j: int| 0 <= j < i < retained.len() ==> retained[j].kb.snapshot != retained[i].kb.snapshot
}
pub open spec fn selection_target_valid(state: PortfolioStateView, plan: PortfolioSelectionPlanView,
    target: PortfolioStateView) -> bool {
    portfolio_state_valid_spec(target) && preserves_prefix_spec(target.kb, state.kb) && plan.selected == target.selected
}
pub open spec fn portfolio_selection_valid_spec(state: PortfolioStateView, plan: PortfolioSelectionPlanView,
    retained: Seq<PortfolioStateView>) -> bool {
    portfolio_state_valid_spec(state) && writfold_domain::text::nonblank_spec(plan.actor)
        && plan.new_snapshot != state.kb.snapshot && retained_valid_spec(retained, plan.new_snapshot)
        && plan.contexts.len() > 0 && selection_contexts_valid_spec(state.contexts, plan.contexts, plan.new_snapshot)
        && actors_authorized(plan.contexts, plan.actor)
        && portfolio_state_valid_spec(selected_portfolio_spec(state, plan))
        && exists|i: int| 0 <= i < retained.len() && #[trigger] retained[i].kb.snapshot == plan.retained_snapshot
            && selection_target_valid(state, plan, retained[i])
}
pub open spec fn first_plan(journal: Seq<PortfolioJournalEntryView>, plan: PortfolioPlanView) -> Option<PortfolioPlanView> {
    if exists|i: int| 0 <= i < journal.len() && #[trigger] journal[i].plan.id == plan.id {
        let i = choose|i: int| 0 <= i < journal.len() && #[trigger] journal[i].plan.id == plan.id
            && forall|j: int| 0 <= j < i ==> #[trigger] journal[j].plan.id != plan.id;
        Some(journal[i].plan)
    } else { None }
}
pub open spec fn first_selection_plan(journal: Seq<PortfolioSelectionJournalEntryView>,
    plan: PortfolioSelectionPlanView) -> Option<PortfolioSelectionPlanView> {
    if exists|i: int| 0 <= i < journal.len() && #[trigger] journal[i].plan.id == plan.id {
        let i = choose|i: int| 0 <= i < journal.len() && #[trigger] journal[i].plan.id == plan.id
            && forall|j: int| 0 <= j < i ==> #[trigger] journal[j].plan.id != plan.id;
        Some(journal[i].plan)
    } else { None }
}
pub open spec fn apply_portfolio_spec(state: PortfolioStateView, journal: Seq<PortfolioJournalEntryView>,
    plan: PortfolioPlanView, approvals: Seq<ModelPortfolioApprovalView>)
    -> WorkflowResultView<PortfolioStateView, PortfolioJournalEntryView> {
    let outcome = match first_plan(journal, plan) {
        Some(recorded) => replay_spec(recorded, plan),
        None => if plan.expected_snapshot != state.kb.snapshot { CommitOutcome::PredecessorConflict }
            else if !(exists|i: int| 0 <= i < approvals.len() && #[trigger] approvals[i].plan == plan)
                || !portfolio_plan_valid_spec(state, plan) { CommitOutcome::Rejected }
            else { CommitOutcome::Applied },
    };
    if outcome == CommitOutcome::Applied {
        WorkflowResultView { state: admitted_portfolio_spec(state, plan),
            journal: seq![PortfolioJournalEntryView { plan }] + journal, outcome,
            state_borrowed: false, journal_borrowed: false }
    } else { WorkflowResultView { state, journal, outcome, state_borrowed: true, journal_borrowed: true } }
}
pub open spec fn apply_portfolio_selection_spec(state: PortfolioStateView,
    journal: Seq<PortfolioSelectionJournalEntryView>, plan: PortfolioSelectionPlanView,
    approvals: Seq<ModelPortfolioSelectionApprovalView>, retained: Seq<PortfolioStateView>)
    -> WorkflowResultView<PortfolioStateView, PortfolioSelectionJournalEntryView> {
    let outcome = match first_selection_plan(journal, plan) {
        Some(recorded) => replay_spec(recorded, plan),
        None => if plan.expected_snapshot != state.kb.snapshot { CommitOutcome::PredecessorConflict }
            else if !(exists|i: int| 0 <= i < approvals.len() && #[trigger] approvals[i].plan == plan)
                || !portfolio_selection_valid_spec(state, plan, retained) { CommitOutcome::Rejected }
            else { CommitOutcome::Applied },
    };
    if outcome == CommitOutcome::Applied {
        WorkflowResultView { state: selected_portfolio_spec(state, plan),
            journal: seq![PortfolioSelectionJournalEntryView { plan }] + journal, outcome,
            state_borrowed: false, journal_borrowed: false }
    } else { WorkflowResultView { state, journal, outcome, state_borrowed: true, journal_borrowed: true } }
}
} // verus!
