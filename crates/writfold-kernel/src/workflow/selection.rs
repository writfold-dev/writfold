//! Retained selection preserves journal precedence and whole-plan replay identity.
use super::result::replay_outcome;
use super::{SelectionResult, actor_present, kb_at, preserves_prefix};
use super::{admission::actor_authorized, selection_scan::*};
#[cfg(verus_keep_ghost)]
use super::admission_model::*;
use crate::{checker::context_matches, wellformed::well_formed};
use writfold_domain::*;
use vstd::prelude::*;

verus! {
pub fn selection_plan_valid(state: &SelectionState, context: &Context, plan: &SelectionPlan,
    retained: &[KB]) -> (valid: bool)
    ensures valid == selection_plan_valid_spec(state@, context@, plan@, retained.deep_view()),
{
    if !context_matches(state.kb(), context) || !well_formed(state.kb()) || !selected_unique(state.selected())
        || !references_present(state.kb(), state.selected()) || plan.new_snapshot() == state.kb().snapshot()
        || !retained_valid(retained, plan.new_snapshot()) || !actor_present(plan.actor()) {
        return false;
    }
    let authorized = match plan.actor() {
        Some(actor) => actor_authorized(context, actor),
        None => false,
    };
    if !authorized || plan.context() != context || plan.policy() != context.policy().id()
        || plan.intended_use() != context.intended_use() { return false; }
    let Some(target) = retained_target(retained, plan.target_snapshot()) else { return false; };
    proof {
        let i = choose|i: int| 0 <= i < retained.len() && target@ == retained.deep_view()[i]
            && target@.snapshot == plan@.target_snapshot
            && forall|j: int| 0 <= j < i ==> retained.deep_view()[j].snapshot != plan@.target_snapshot;
        assert forall|j: int| 0 <= j < retained.len() && retained.deep_view()[j].snapshot == plan@.target_snapshot
            implies retained.deep_view()[j] == target@ by {
            assert(i == j);
        }
    }
    well_formed(target) && preserves_prefix(target, state.kb()) && selected_exact(target, plan.selected())
        && references_present(state.kb(), plan.selected())
}
pub fn apply_selection<'a>(state: &'a SelectionState, context: &Context,
    journal: &'a [SelectionJournalEntry], plan: &SelectionPlan,
    trusted_approvals: &[ModelSelectionApproval], retained: &[KB]) -> (result: SelectionResult<'a>)
    ensures result@ == apply_selection_spec(state@, context@, journal.deep_view(), plan@,
        trusted_approvals.deep_view(), retained.deep_view()),
{
    if let Some(recorded) = first_entry(journal, plan) {
        return SelectionResult::unchanged(state, journal, replay_outcome(recorded, plan));
    }
    if plan.expected_snapshot() != state.kb().snapshot() {
        return SelectionResult::unchanged(state, journal, CommitOutcome::PredecessorConflict);
    }
    if !approval_present(trusted_approvals, plan) || !selection_plan_valid(state, context, plan, retained) {
        return SelectionResult::unchanged(state, journal, CommitOutcome::Rejected);
    }
    proof { broadcast use vstd::std_specs::vec::group_vec_axioms; }
    let selected = SelectionState::new(kb_at(state.kb(), plan.new_snapshot()), plan.selected().clone());
    let mut updated = Vec::with_capacity(journal.len().saturating_add(1));
    updated.push(SelectionJournalEntry::new(plan.clone()));
    let mut i = 0;
    while i < journal.len()
        invariant i <= journal.len(), updated.deep_view() == seq![SelectionJournalEntryView { plan: plan@ }]
            + journal.deep_view().take(i as int),
        decreases journal.len() - i,
    {
        updated.push(journal[i].clone());
        proof { assert(updated.deep_view() =~= seq![SelectionJournalEntryView { plan: plan@ }]
            + journal.deep_view().take(i as int + 1)); }
        i += 1;
    }
    SelectionResult::applied(selected, updated)
}
}
