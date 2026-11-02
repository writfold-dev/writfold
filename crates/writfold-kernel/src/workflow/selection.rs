use super::result::replay_outcome;
use super::{SelectionResult, actor_present, kb_at, preserves_prefix, unique};
use crate::{checker::context_matches, wellformed::well_formed};
use writfold_domain::*;

pub fn selection_plan_valid(
    state: &SelectionState,
    context: &Context,
    plan: &SelectionPlan,
    retained: &[KB],
) -> bool {
    if !context_matches(state.kb(), context)
        || !well_formed(state.kb())
        || !unique(state.selected())
        || state
            .selected()
            .iter()
            .any(|reference| find_claim(state.kb(), reference).is_none())
        || plan.new_snapshot() == state.kb().snapshot()
        || retained.iter().enumerate().any(|(index, kb)| {
            retained[..index]
                .iter()
                .any(|prior| prior.snapshot() == kb.snapshot())
        })
        || retained
            .iter()
            .any(|kb| kb.snapshot() == plan.new_snapshot())
        || !actor_present(plan.actor())
        || !plan
            .actor()
            .as_ref()
            .is_some_and(|actor| context.policy().authorized_actors().contains(actor))
        || plan.context() != context
        || plan.policy() != context.policy().id()
        || plan.intended_use() != context.intended_use()
    {
        return false;
    }
    let Some(target) = retained
        .iter()
        .find(|kb| kb.snapshot() == plan.target_snapshot())
    else {
        return false;
    };
    well_formed(target)
        && preserves_prefix(target, state.kb())
        && plan.selected().len() == target.claims().len()
        && plan
            .selected()
            .iter()
            .zip(target.claims())
            .all(|(reference, claim)| claim.matches_ref(reference))
        && plan
            .selected()
            .iter()
            .all(|reference| find_claim(state.kb(), reference).is_some())
}

pub fn apply_selection<'a>(
    state: &'a SelectionState,
    context: &Context,
    journal: &'a [SelectionJournalEntry],
    plan: &SelectionPlan,
    trusted_approvals: &[ModelSelectionApproval],
    retained: &[KB],
) -> SelectionResult<'a> {
    if let Some(entry) = journal.iter().find(|entry| entry.plan().id() == plan.id()) {
        return SelectionResult::unchanged(state, journal, replay_outcome(entry.plan(), plan));
    }
    if plan.expected_snapshot() != state.kb().snapshot() {
        return SelectionResult::unchanged(state, journal, CommitOutcome::PredecessorConflict);
    }
    if !trusted_approvals
        .iter()
        .any(|approval| approval.plan() == plan)
        || !selection_plan_valid(state, context, plan, retained)
    {
        return SelectionResult::unchanged(state, journal, CommitOutcome::Rejected);
    }
    let selected = SelectionState::new(
        kb_at(state.kb(), plan.new_snapshot()),
        plan.selected().clone(),
    );
    let mut updated = Vec::with_capacity(journal.len() + 1);
    updated.push(SelectionJournalEntry::new(plan.clone()));
    updated.extend_from_slice(journal);
    SelectionResult::applied(selected, updated)
}
