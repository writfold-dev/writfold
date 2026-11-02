use super::{
    PortfolioSelectionResult, context_fixed_fields_equal, find_context, portfolio_state_valid,
};
use crate::wellformed::context_wf;
use crate::workflow::result::replay_outcome;
use crate::workflow::{kb_at, preserves_prefix, unique};
use writfold_domain::*;

pub fn selected_portfolio(state: &PortfolioState, plan: &PortfolioSelectionPlan) -> PortfolioState {
    PortfolioState::new(
        kb_at(state.kb(), plan.new_snapshot()),
        plan.selected().clone(),
        plan.contexts().clone(),
    )
}

pub fn selection_contexts_valid(
    before: &[NamedContext],
    after: &[NamedContext],
    snapshot: &SnapshotId,
) -> bool {
    before
        .iter()
        .map(|item| item.id())
        .eq(after.iter().map(|item| item.id()))
        && after.iter().all(|current| {
            find_context(before, current.id()).is_some_and(|old| {
                let context = current.context();
                let prior = old.context();
                context.snapshot() == snapshot
                    && context_fixed_fields_equal(context, prior)
                    && context.target() == prior.target()
                    && prior.now() <= context.now()
            })
        })
}

pub fn portfolio_selection_valid(
    state: &PortfolioState,
    plan: &PortfolioSelectionPlan,
    retained: &[PortfolioState],
) -> bool {
    if !portfolio_state_valid(state) || plan.actor().as_str().trim().is_empty()
        || plan.new_snapshot() == state.kb().snapshot()
        || retained.iter().any(|old| old.kb().snapshot() == plan.new_snapshot())
        || retained.iter().enumerate().any(|(index, old)| retained[..index].iter()
            .any(|prior| prior.kb().snapshot() == old.kb().snapshot()))
        || plan.contexts().is_empty()
        || !selection_contexts_valid(state.contexts(), plan.contexts(), plan.new_snapshot())
        || !plan.contexts().iter().all(|item| item.context().policy().authorized_actors().contains(plan.actor()))
        // The current KB remains unchanged except its already-valid nominal snapshot.
        || !unique(plan.selected())
        || !plan.selected().iter().all(|reference| find_claim(state.kb(), reference).is_some())
        || !plan.contexts().iter().all(|item| context_wf(item.context()))
    {
        return false;
    }
    retained
        .iter()
        .find(|old| old.kb().snapshot() == plan.retained_snapshot())
        .is_some_and(|target| {
            portfolio_state_valid(target)
                && preserves_prefix(target.kb(), state.kb())
                && plan.selected() == target.selected()
        })
}

pub fn apply_portfolio_selection<'a>(
    state: &'a PortfolioState,
    journal: &'a [PortfolioSelectionJournalEntry],
    plan: &PortfolioSelectionPlan,
    trusted_approvals: &[ModelPortfolioSelectionApproval],
    retained: &[PortfolioState],
) -> PortfolioSelectionResult<'a> {
    if let Some(entry) = journal.iter().find(|entry| entry.plan().id() == plan.id()) {
        return PortfolioSelectionResult::unchanged(
            state,
            journal,
            replay_outcome(entry.plan(), plan),
        );
    }
    if plan.expected_snapshot() != state.kb().snapshot() {
        return PortfolioSelectionResult::unchanged(
            state,
            journal,
            CommitOutcome::PredecessorConflict,
        );
    }
    if !trusted_approvals
        .iter()
        .any(|approval| approval.plan() == plan)
        || !portfolio_selection_valid(state, plan, retained)
    {
        return PortfolioSelectionResult::unchanged(state, journal, CommitOutcome::Rejected);
    }
    let mut updated = Vec::with_capacity(journal.len() + 1);
    updated.push(PortfolioSelectionJournalEntry::new(plan.clone()));
    updated.extend_from_slice(journal);
    PortfolioSelectionResult::applied(selected_portfolio(state, plan), updated)
}
