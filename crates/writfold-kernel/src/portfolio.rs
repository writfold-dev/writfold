//! Named contexts authorize independently; evidence is never pooled.
mod selection;
use crate::checker::{assess, check_matches, context_matches, usable_for};
use crate::wellformed::well_formed;
use crate::workflow::result::replay_outcome;
pub use crate::workflow::result::{PortfolioResult, PortfolioSelectionResult};
use crate::workflow::{preserves_prefix, unique};
pub use selection::*;
use writfold_domain::*;

pub fn find_context<'a>(
    contexts: &'a [NamedContext],
    identity: &ContextId,
) -> Option<&'a NamedContext> {
    contexts.iter().find(|item| item.id() == identity)
}

pub fn context_family_valid(kb: &KB, contexts: &[NamedContext]) -> bool {
    contexts.iter().enumerate().all(|(index, item)| {
        !contexts[..index]
            .iter()
            .any(|prior| prior.id() == item.id())
            && context_matches(kb, item.context())
    })
}

pub fn portfolio_state_valid(state: &PortfolioState) -> bool {
    well_formed(state.kb())
        && unique(state.selected())
        && state
            .selected()
            .iter()
            .all(|reference| find_claim(state.kb(), reference).is_some())
        && context_family_valid(state.kb(), state.contexts())
        && (!state.contexts().is_empty() || state.kb().claims().is_empty())
}

pub fn retired_context_ids(before: &[NamedContext], after: &[NamedContext]) -> Vec<ContextId> {
    before
        .iter()
        .filter(|item| find_context(after, item.id()).is_none())
        .map(|item| item.id().clone())
        .collect()
}

pub fn context_evolution_valid(
    before: &[NamedContext],
    after: &[NamedContext],
    retired: &[ContextId],
) -> bool {
    unique(retired)
        && before
            .iter()
            .filter(|item| find_context(after, item.id()).is_none())
            .map(|item| item.id())
            .eq(retired.iter())
        && before.iter().all(|old| {
            find_context(after, old.id()).is_none_or(|current| {
                current.context().target() == old.context().target()
                    && current.context().intended_use() == old.context().intended_use()
            })
        })
}

pub fn assignment_valid(
    candidate: &KB,
    contexts: &[NamedContext],
    assignment: &ClaimAssignment,
) -> bool {
    let Some(named) = find_context(contexts, assignment.context()) else {
        return false;
    };
    let Some(claim) = find_claim(candidate, assignment.claim()) else {
        return false;
    };
    check_matches(
        candidate,
        named.context(),
        assignment.witness(),
        claim.statement(),
    ) && candidate.derivations().iter().any(|derivation| {
        derivation.claim() == assignment.claim() && derivation.witness() == assignment.witness()
    }) && usable_for(candidate, named.context(), assignment.claim())
}

pub fn portfolio_added_refs(state: &PortfolioState, plan: &PortfolioPlan) -> Vec<ClaimRef> {
    plan.candidate()
        .claims()
        .iter()
        .skip(state.kb().claims().len())
        .map(Claim::claim_ref)
        .collect()
}

pub fn admitted_portfolio(state: &PortfolioState, plan: &PortfolioPlan) -> PortfolioState {
    let mut selected = state.selected().clone();
    selected.extend(
        plan.candidate()
            .claims()
            .iter()
            .skip(state.kb().claims().len())
            .map(Claim::claim_ref),
    );
    PortfolioState::new(plan.candidate().clone(), selected, plan.contexts().clone())
}

pub fn portfolio_plan_valid(state: &PortfolioState, plan: &PortfolioPlan) -> bool {
    if !portfolio_state_valid(state)
        || plan.candidate().snapshot() == state.kb().snapshot()
        || !preserves_prefix(state.kb(), plan.candidate())
        || plan.actor().as_str().trim().is_empty()
        || plan.contexts().is_empty()
        || !plan.contexts().iter().all(|item| {
            item.context()
                .policy()
                .authorized_actors()
                .contains(plan.actor())
        })
        || !context_evolution_valid(state.contexts(), plan.contexts(), plan.retired_contexts())
    {
        return false;
    }
    let added = &plan.candidate().claims()[state.kb().claims().len()..];
    plan.assignments().len() == added.len()
        && plan.assignments().iter().zip(added).all(|(assignment, claim)| claim.matches_ref(assignment.claim()))
        && plan.assignments().iter().all(|item| assignment_valid(plan.candidate(), plan.contexts(), item))
        // Validate the resulting state without allocating an owned candidate state.
        && well_formed(plan.candidate()) && context_family_valid(plan.candidate(), plan.contexts())
        && added.iter().all(|claim| !state.selected().iter().any(|reference| claim.matches_ref(reference)))
}

pub fn apply_portfolio<'a>(
    state: &'a PortfolioState,
    journal: &'a [PortfolioJournalEntry],
    plan: &PortfolioPlan,
    trusted_approvals: &[ModelPortfolioApproval],
) -> PortfolioResult<'a> {
    if let Some(entry) = journal.iter().find(|entry| entry.plan().id() == plan.id()) {
        return PortfolioResult::unchanged(state, journal, replay_outcome(entry.plan(), plan));
    }
    if plan.expected_snapshot() != state.kb().snapshot() {
        return PortfolioResult::unchanged(state, journal, CommitOutcome::PredecessorConflict);
    }
    if !trusted_approvals
        .iter()
        .any(|approval| approval.plan() == plan)
        || !portfolio_plan_valid(state, plan)
    {
        return PortfolioResult::unchanged(state, journal, CommitOutcome::Rejected);
    }
    let mut updated = Vec::with_capacity(journal.len() + 1);
    updated.push(PortfolioJournalEntry::new(plan.clone()));
    updated.extend_from_slice(journal);
    PortfolioResult::applied(admitted_portfolio(state, plan), updated)
}

pub(crate) fn context_fixed_fields_equal(left: &Context, right: &Context) -> bool {
    left.semantic_version() == right.semantic_version()
        && left.ruleset() == right.ruleset()
        && left.checker() == right.checker()
        && left.policy() == right.policy()
        && left.intended_use() == right.intended_use()
        && left.sources() == right.sources()
}

pub fn query_context_allowed(
    state: &PortfolioState,
    identity: &ContextId,
    query: &Context,
) -> bool {
    context_family_valid(state.kb(), state.contexts())
        && context_matches(state.kb(), query)
        && find_context(state.contexts(), identity).is_some_and(|approved| {
            let context = approved.context();
            query.snapshot() == context.snapshot()
                && context_fixed_fields_equal(query, context)
                && narrower(query.target(), context.target())
                && context.now() <= query.now()
        })
}

pub fn assess_portfolio(
    state: &PortfolioState,
    identity: &ContextId,
    query: &Context,
    reference: &ClaimRef,
) -> Option<Assessment> {
    if query_context_allowed(state, identity, query) && state.selected().contains(reference) {
        Some(assess(state.kb(), query, reference))
    } else {
        None
    }
}
