//! Forward selection preserves current history, ordered contexts and policy inputs.
use super::contexts::all_authorized;
use super::journal::{first_selection_entry, selection_approval_present};
use super::retained::{contexts_wellformed, retained_target, retained_valid};
use super::{
    PortfolioSelectionResult, context_fixed_fields_equal, find_context, portfolio_state_valid,
};
#[cfg(verus_keep_ghost)]
use super::{model::*, projection::selection_projection};
use crate::workflow::result::replay_outcome;
use crate::workflow::{kb_at, preserves_prefix, references_present, selected_unique};
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub fn selected_portfolio(state: &PortfolioState, plan: &PortfolioSelectionPlan) -> (result: PortfolioState)
    ensures result@ == selected_portfolio_spec(state@, plan@),
{
    proof { broadcast use vstd::std_specs::vec::group_vec_axioms; }
    PortfolioState::new(kb_at(state.kb(), plan.new_snapshot()), plan.selected().clone(), super::contexts::clone_contexts(plan.contexts()))
}
fn selected_context_matches(before: &[NamedContext], current: &NamedContext, snapshot: &SnapshotId) -> (valid: bool)
    ensures valid == selection_context_valid(before.deep_view(), current@, snapshot@),
{
    let Some(old) = find_context(before, current.id()) else { return false; };
    let context = current.context();
    let prior = old.context();
    context.snapshot() == snapshot && context_fixed_fields_equal(context, prior)
        && context.target() == prior.target() && prior.now() <= context.now()
}
pub fn selection_contexts_valid(before: &[NamedContext], after: &[NamedContext], snapshot: &SnapshotId) -> (valid: bool)
    ensures valid == selection_contexts_valid_spec(before.deep_view(), after.deep_view(), snapshot@),
{
    proof {
        assert(before.deep_view().len() == before.len());
        assert(after.deep_view().len() == after.len());
    }
    if before.len() != after.len() { return false; }
    let mut i = 0;
    while i < before.len()
        invariant i <= before.len(), before.len() == after.len(),
            before.deep_view().len() == before.len(), after.deep_view().len() == after.len(),
            forall|j: int| 0 <= j < i ==> before.deep_view()[j].id == after.deep_view()[j].id,
        decreases before.len() - i,
    {
        let old = &before[i];
        let current = &after[i];
        proof { assert(old@ == before.deep_view()[i as int]); assert(current@ == after.deep_view()[i as int]); }
        if old.id() != current.id() {
            proof { if selection_contexts_valid_spec(before.deep_view(), after.deep_view(), snapshot@) {
                assert(before.deep_view()[i as int].id == after.deep_view()[i as int].id);
            } }
            return false;
        }
        i += 1;
    }
    let mut i = 0;
    while i < after.len()
        invariant i <= after.len(), before.len() == after.len(),
            before.deep_view().len() == before.len(), after.deep_view().len() == after.len(),
            forall|j: int| 0 <= j < before.len() ==> before.deep_view()[j].id == after.deep_view()[j].id,
            forall|j: int| 0 <= j < i
                ==> #[trigger] selection_context_valid(before.deep_view(), after.deep_view()[j], snapshot@),
        decreases after.len() - i,
    {
        let current = &after[i];
        proof { assert(current@ == after.deep_view()[i as int]); }
        if !selected_context_matches(before, current, snapshot) {
            proof { if selection_contexts_valid_spec(before.deep_view(), after.deep_view(), snapshot@) {
                assert(selection_context_valid(before.deep_view(), after.deep_view()[i as int], snapshot@));
            } }
            return false;
        }
        i += 1;
    }
    true
}
pub fn portfolio_selection_valid(state: &PortfolioState, plan: &PortfolioSelectionPlan,
    retained: &[PortfolioState]) -> (valid: bool)
    ensures valid == portfolio_selection_valid_spec(state@, plan@, retained.deep_view()),
{
    proof { broadcast use writfold_domain::equality::vector_equality; }
    if !portfolio_state_valid(state) || !writfold_domain::text::nonblank(plan.actor().as_str())
        || plan.new_snapshot() == state.kb().snapshot() || !retained_valid(retained, plan.new_snapshot())
        || plan.contexts().is_empty() || !selection_contexts_valid(state.contexts(), plan.contexts(), plan.new_snapshot())
        || !all_authorized(plan.contexts(), plan.actor()) { return false; }
    // Validate the projection without constructing an owned duplicate of the current KB.
    proof { selection_projection(state@, plan@); }
    if !selected_unique(plan.selected()) || !references_present(state.kb(), plan.selected())
        || !contexts_wellformed(plan.contexts()) { return false; }
    let Some(target) = retained_target(retained, plan.retained_snapshot()) else { return false; };
    proof {
        let i = choose|i: int| 0 <= i < retained.len() && target@ == retained.deep_view()[i]
            && target@.kb.snapshot == plan@.retained_snapshot
            && forall|j: int| 0 <= j < i ==> retained.deep_view()[j].kb.snapshot != plan@.retained_snapshot;
        assert forall|j: int| 0 <= j < retained.len() && retained.deep_view()[j].kb.snapshot == plan@.retained_snapshot
            implies retained.deep_view()[j] == target@ by { assert(i == j); }
    }
    portfolio_state_valid(target) && preserves_prefix(target.kb(), state.kb()) && plan.selected() == target.selected()
}
pub fn apply_portfolio_selection<'a>(state: &'a PortfolioState, journal: &'a [PortfolioSelectionJournalEntry],
    plan: &PortfolioSelectionPlan, trusted_approvals: &[ModelPortfolioSelectionApproval], retained: &[PortfolioState])
    -> (result: PortfolioSelectionResult<'a>)
    ensures result@ == apply_portfolio_selection_spec(state@, journal.deep_view(), plan@,
        trusted_approvals.deep_view(), retained.deep_view()),
{
    if let Some(recorded) = first_selection_entry(journal, plan) {
        return PortfolioSelectionResult::unchanged(state, journal, replay_outcome(recorded, plan));
    }
    if plan.expected_snapshot() != state.kb().snapshot() {
        return PortfolioSelectionResult::unchanged(state, journal, CommitOutcome::PredecessorConflict);
    }
    if !selection_approval_present(trusted_approvals, plan) || !portfolio_selection_valid(state, plan, retained) {
        return PortfolioSelectionResult::unchanged(state, journal, CommitOutcome::Rejected);
    }
    proof { assert(apply_portfolio_selection_spec(state@, journal.deep_view(), plan@,
        trusted_approvals.deep_view(), retained.deep_view()).outcome == CommitOutcome::Applied); }
    let mut updated = Vec::with_capacity(journal.len().saturating_add(1));
    updated.push(PortfolioSelectionJournalEntry::new(plan.clone()));
    let mut i = 0;
    proof { assert(journal.deep_view().len() == journal.len()); }
    while i < journal.len()
        invariant i <= journal.len(), updated.deep_view() == seq![PortfolioSelectionJournalEntryView { plan: plan@ }]
            + journal.deep_view().take(i as int),
            journal.deep_view().len() == journal.len(),
            apply_portfolio_selection_spec(state@, journal.deep_view(), plan@,
                trusted_approvals.deep_view(), retained.deep_view()).outcome == CommitOutcome::Applied,
        decreases journal.len() - i,
    {
        let ghost prefix = updated.deep_view();
        let entry = journal[i].clone();
        proof { assert(entry@ == journal.deep_view()[i as int]); }
        updated.push(entry);
        proof {
            assert(updated.deep_view() =~= prefix.push(entry@));
            assert(seq![PortfolioSelectionJournalEntryView { plan: plan@ }] + journal.deep_view().take(i as int + 1)
                =~= prefix.push(entry@));
        }
        i += 1;
    }
    proof { assert(journal.deep_view().take(journal.len() as int) == journal.deep_view()); }
    PortfolioSelectionResult::applied(selected_portfolio(state, plan), updated)
}
} // verus!
