//! Named contexts authorize independently; evidence is never pooled.
mod contexts;
mod journal;
#[cfg(verus_keep_ghost)]
pub mod model;
#[cfg(verus_keep_ghost)]
mod projection;
mod retained;
mod selection;
#[cfg(verus_keep_ghost)]
use crate::checker::spec::assess_spec;
use crate::checker::{assess, check_matches, context_matches, usable_for};
use crate::wellformed::well_formed;
use crate::workflow::result::replay_outcome;
pub use crate::workflow::result::{PortfolioResult, PortfolioSelectionResult};
use crate::workflow::{preserves_prefix, references_present, selected_unique};
use contexts::{all_authorized, context_fixed_fields_equal};
pub use contexts::{
    context_evolution_valid, context_family_valid, find_context, retired_context_ids,
};
#[cfg(verus_keep_ghost)]
use model::*;
#[cfg(verus_keep_ghost)]
use projection::*;
pub use selection::*;
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub fn portfolio_state_valid(state: &PortfolioState) -> (valid: bool)
    ensures valid == portfolio_state_valid_spec(state@),
{
    well_formed(state.kb()) && selected_unique(state.selected())
        && references_present(state.kb(), state.selected()) && context_family_valid(state.kb(), state.contexts())
        && (!state.contexts().is_empty() || state.kb().claims().is_empty())
}
fn assignment_registered(candidate: &KB, assignment: &ClaimAssignment) -> (registered: bool)
    ensures registered == (exists|i: int| 0 <= i < candidate@.derivations.len()
        && candidate@.derivations[i].claim == assignment@.claim
        && candidate@.derivations[i].witness == assignment@.witness),
{
    let rows = candidate.derivations();
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(), rows.deep_view() == candidate@.derivations,
            forall|j: int| 0 <= j < i ==> rows.deep_view()[j].claim != assignment@.claim
                || rows.deep_view()[j].witness != assignment@.witness,
        decreases rows.len() - i,
    {
        let row = &rows[i];
        proof { assert(row@ == rows.deep_view()[i as int]); }
        if row.claim() == assignment.claim() && row.witness() == assignment.witness() { return true; }
        i += 1;
    }
    false
}
pub fn assignment_valid(candidate: &KB, contexts: &[NamedContext], assignment: &ClaimAssignment) -> (valid: bool)
    ensures valid == assignment_valid_spec(candidate@, contexts.deep_view(), assignment@),
{
    let Some(named) = find_context(contexts, assignment.context()) else { return false; };
    let Some(claim) = find_claim(candidate, assignment.claim()) else { return false; };
    proof {
        assert forall|other: ClaimView| first_claim_match(candidate@.claims, assignment@.claim, Some(other))
            implies other == claim@ by {
            let i = choose|i: int| 0 <= i < candidate@.claims.len()
                && other == candidate@.claims[i] && claim_matches_spec(other, assignment@.claim)
                && forall|j: int| 0 <= j < i ==> !claim_matches_spec(candidate@.claims[j], assignment@.claim);
            let k = choose|k: int| 0 <= k < candidate@.claims.len()
                && claim@ == candidate@.claims[k] && claim_matches_spec(claim@, assignment@.claim)
                && forall|j: int| 0 <= j < k ==> !claim_matches_spec(candidate@.claims[j], assignment@.claim);
            assert(i == k);
        }
    }
    check_matches(candidate, named.context(), assignment.witness(), claim.statement())
        && assignment_registered(candidate, assignment) && usable_for(candidate, named.context(), assignment.claim())
}
fn append_added(state: &PortfolioState, plan: &PortfolioPlan, selected: &mut Vec<ClaimRef>)
    ensures final(selected).deep_view() == old(selected).deep_view() + added_refs_spec(state@, plan@),
{
    let claims = plan.candidate().claims();
    let start = if state.kb().claims().len() < claims.len() { state.kb().claims().len() } else { claims.len() };
    let ghost initial = selected.deep_view();
    let ghost added = added_refs_spec(state@, plan@);
    proof {
        assert(claims.deep_view().len() == claims.len());
        assert(added.len() == claims.len() - start);
    }
    let mut i = start;
    while i < claims.len()
        invariant start <= i <= claims.len(), claims.deep_view() == plan@.candidate.claims,
            claims.deep_view().len() == claims.len(), added == added_refs_spec(state@, plan@),
            added.len() == claims.len() - start,
            start == if state@.kb.claims.len() < claims.len() { state@.kb.claims.len() } else { claims.len() as nat },
            selected.deep_view() == initial + added_refs_spec(state@, plan@).take((i - start) as int),
        decreases claims.len() - i,
    {
        let ghost prefix = selected.deep_view();
        let claim = &claims[i];
        proof { assert(claim@ == plan@.candidate.claims[i as int]); }
        let reference = claim.claim_ref();
        proof { assert(reference@ == added[(i - start) as int]); }
        selected.push(reference);
        proof {
            assert(selected.deep_view() =~= prefix.push(reference@));
            assert(initial + added.take((i - start + 1) as int) =~= prefix.push(reference@));
        }
        i += 1;
    }
    proof { assert(added.take((i - start) as int) == added); }
}
pub fn portfolio_added_refs(state: &PortfolioState, plan: &PortfolioPlan) -> (added: Vec<ClaimRef>)
    ensures added.deep_view() == added_refs_spec(state@, plan@),
{
    let mut added = Vec::with_capacity(plan.candidate().claims().len().saturating_sub(state.kb().claims().len()));
    append_added(state, plan, &mut added);
    added
}
pub fn admitted_portfolio(state: &PortfolioState, plan: &PortfolioPlan) -> (result: PortfolioState)
    ensures result@ == admitted_portfolio_spec(state@, plan@),
{
    let prior = state.selected();
    let additional = plan.candidate().claims().len().saturating_sub(state.kb().claims().len());
    let mut selected = Vec::with_capacity(prior.len().saturating_add(additional));
    let mut i = 0;
    proof { assert(prior.deep_view().len() == prior.len()); }
    while i < prior.len()
        invariant i <= prior.len(), prior.deep_view() == state@.selected,
            prior.deep_view().len() == prior.len(), selected.deep_view() == prior.deep_view().take(i as int),
        decreases prior.len() - i,
    {
        let ghost prefix = prior.deep_view().take(i as int);
        let reference = prior[i].clone();
        proof { assert(reference@ == prior.deep_view()[i as int]); }
        selected.push(reference);
        proof {
            assert(selected.deep_view() =~= prefix.push(reference@));
            assert(prior.deep_view().take(i as int + 1) =~= prefix.push(reference@));
        }
        i += 1;
    }
    proof { assert(prior.deep_view().take(prior.len() as int) == prior.deep_view()); }
    append_added(state, plan, &mut selected);
    PortfolioState::new(plan.candidate().clone(), selected, contexts::clone_contexts(plan.contexts()))
}
fn assignments_valid(state: &PortfolioState, plan: &PortfolioPlan) -> (valid: bool)
    requires state@.kb.claims.len() <= plan@.candidate.claims.len(),
    ensures valid == assignments_exact(state@, plan@),
{
    let start = state.kb().claims().len();
    let candidate = plan.candidate();
    let claims = candidate.claims();
    let assignments = plan.assignments();
    proof {
        assert(claims.deep_view().len() == claims.len());
        assert(assignments.deep_view().len() == assignments.len());
    }
    if assignments.len() != claims.len() - start { return false; }
    let mut i = 0;
    while i < assignments.len()
        invariant i <= assignments.len(), start == state@.kb.claims.len(), start <= claims.len(),
            assignments.len() == claims.len() - start, claims.deep_view() == plan@.candidate.claims,
            claims.deep_view().len() == claims.len(), assignments.deep_view() == plan@.assignments,
            candidate@ == plan@.candidate,
            assignments.deep_view().len() == assignments.len(),
            forall|j: int| 0 <= j < i ==> claim_matches_spec(plan@.candidate.claims[start + j], plan@.assignments[j].claim),
        decreases assignments.len() - i,
    {
        let assignment = &assignments[i];
        let claim = &claims[start + i];
        proof {
            assert(assignment@ == plan@.assignments[i as int]);
            assert(claim@ == plan@.candidate.claims[start + i]);
        }
        if !claim.matches_ref(assignment.claim()) {
            proof { if assignments_exact(state@, plan@) {
                assert(claim_matches_spec(plan@.candidate.claims[start + i], plan@.assignments[i as int].claim));
            } }
            return false;
        }
        i += 1;
    }
    let mut i = 0;
    while i < assignments.len()
        invariant i <= assignments.len(), start == state@.kb.claims.len(),
            start <= claims.len(), assignments.len() == claims.len() - start,
            claims.deep_view() == plan@.candidate.claims, claims.deep_view().len() == claims.len(),
            candidate@ == plan@.candidate,
            assignments.deep_view() == plan@.assignments, assignments.deep_view().len() == assignments.len(),
            forall|j: int| 0 <= j < assignments.len()
                ==> claim_matches_spec(plan@.candidate.claims[start + j], plan@.assignments[j].claim),
            forall|j: int| 0 <= j < i ==> assignment_valid_spec(plan@.candidate, plan@.contexts, plan@.assignments[j]),
        decreases assignments.len() - i,
    {
        let assignment = &assignments[i];
        proof { assert(assignment@ == plan@.assignments[i as int]); }
        if !assignment_valid(candidate, plan.contexts(), assignment) {
            proof { if assignments_exact(state@, plan@) {
                assert(assignment_valid_spec(plan@.candidate, plan@.contexts, plan@.assignments[i as int]));
            } }
            return false;
        }
        i += 1;
    }
    true
}
fn added_disjoint(state: &PortfolioState, plan: &PortfolioPlan) -> (valid: bool)
    requires state@.kb.claims.len() <= plan@.candidate.claims.len(),
    ensures valid == appended_disjoint(state@, plan@),
{
    let claims = plan.candidate().claims();
    let selected = state.selected();
    let mut i = state.kb().claims().len();
    proof {
        assert(claims.deep_view().len() == claims.len());
        assert(selected.deep_view().len() == selected.len());
    }
    while i < claims.len()
        invariant state@.kb.claims.len() <= i <= claims.len(),
            claims.deep_view() == plan@.candidate.claims, claims.deep_view().len() == claims.len(),
            selected.deep_view() == state@.selected, selected.deep_view().len() == selected.len(),
            forall|a: int, b: int| state@.kb.claims.len() <= a < i && 0 <= b < selected.len()
                ==> !claim_matches_spec(plan@.candidate.claims[a], state@.selected[b]),
        decreases claims.len() - i,
    {
        let claim = &claims[i];
        proof { assert(claim@ == plan@.candidate.claims[i as int]); }
        let mut j = 0;
        while j < selected.len()
            invariant state@.kb.claims.len() <= i < claims.len(), j <= selected.len(),
                claims.deep_view() == plan@.candidate.claims, claims.deep_view().len() == claims.len(),
                selected.deep_view() == state@.selected, selected.deep_view().len() == selected.len(),
                claim@ == plan@.candidate.claims[i as int],
                forall|a: int, b: int| state@.kb.claims.len() <= a < i && 0 <= b < selected.len()
                    ==> !claim_matches_spec(plan@.candidate.claims[a], state@.selected[b]),
                forall|b: int| 0 <= b < j ==> !claim_matches_spec(claim@, state@.selected[b]),
            decreases selected.len() - j,
        {
            let reference = &selected[j];
            proof { assert(reference@ == state@.selected[j as int]); }
            if claim.matches_ref(reference) {
                proof { if appended_disjoint(state@, plan@) {
                    assert(!claim_matches_spec(plan@.candidate.claims[i as int], state@.selected[j as int]));
                } }
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}
pub fn portfolio_plan_valid(state: &PortfolioState, plan: &PortfolioPlan) -> (valid: bool)
    ensures valid == portfolio_plan_valid_spec(state@, plan@),
{
    if !portfolio_state_valid(state) || plan.candidate().snapshot() == state.kb().snapshot()
        || !preserves_prefix(state.kb(), plan.candidate()) || !writfold_domain::text::nonblank(plan.actor().as_str())
        || plan.contexts().is_empty() || !all_authorized(plan.contexts(), plan.actor())
        || !context_evolution_valid(state.contexts(), plan.contexts(), plan.retired_contexts()) { return false; }
    proof { admission_projection(state@, plan@); }
    assignments_valid(state, plan) && well_formed(plan.candidate())
        && context_family_valid(plan.candidate(), plan.contexts()) && added_disjoint(state, plan)
}
pub fn apply_portfolio<'a>(state: &'a PortfolioState, journal: &'a [PortfolioJournalEntry], plan: &PortfolioPlan,
    trusted_approvals: &[ModelPortfolioApproval]) -> (result: PortfolioResult<'a>)
    ensures result@ == apply_portfolio_spec(state@, journal.deep_view(), plan@, trusted_approvals.deep_view()),
{
    if let Some(recorded) = journal::first_entry(journal, plan) {
        return PortfolioResult::unchanged(state, journal, replay_outcome(recorded, plan));
    }
    if plan.expected_snapshot() != state.kb().snapshot() {
        return PortfolioResult::unchanged(state, journal, CommitOutcome::PredecessorConflict);
    }
    if !journal::approval_present(trusted_approvals, plan) || !portfolio_plan_valid(state, plan) {
        return PortfolioResult::unchanged(state, journal, CommitOutcome::Rejected);
    }
    proof { assert(apply_portfolio_spec(state@, journal.deep_view(), plan@, trusted_approvals.deep_view()).outcome
        == CommitOutcome::Applied); }
    let mut updated = Vec::with_capacity(journal.len().saturating_add(1));
    updated.push(PortfolioJournalEntry::new(plan.clone()));
    let mut i = 0;
    proof { assert(journal.deep_view().len() == journal.len()); }
    while i < journal.len()
        invariant i <= journal.len(), updated.deep_view() == seq![PortfolioJournalEntryView { plan: plan@ }]
            + journal.deep_view().take(i as int),
            journal.deep_view().len() == journal.len(),
            apply_portfolio_spec(state@, journal.deep_view(), plan@, trusted_approvals.deep_view()).outcome
                == CommitOutcome::Applied,
        decreases journal.len() - i,
    {
        let ghost prefix = updated.deep_view();
        let entry = journal[i].clone();
        proof { assert(entry@ == journal.deep_view()[i as int]); }
        updated.push(entry);
        proof {
            assert(updated.deep_view() =~= prefix.push(entry@));
            assert(seq![PortfolioJournalEntryView { plan: plan@ }] + journal.deep_view().take(i as int + 1)
                =~= prefix.push(entry@));
        }
        i += 1;
    }
    proof { assert(journal.deep_view().take(journal.len() as int) == journal.deep_view()); }
    PortfolioResult::applied(admitted_portfolio(state, plan), updated)
}
pub fn query_context_allowed(state: &PortfolioState, identity: &ContextId, query: &Context) -> (allowed: bool)
    ensures allowed == query_context_allowed_spec(state@, identity@, query@),
{
    if !context_family_valid(state.kb(), state.contexts()) || !context_matches(state.kb(), query) { return false; }
    let Some(approved) = find_context(state.contexts(), identity) else { return false; };
    let context = approved.context();
    query.snapshot() == context.snapshot() && context_fixed_fields_equal(query, context)
        && narrower(query.target(), context.target()) && context.now() <= query.now()
}
fn selection_contains(selected: &[ClaimRef], reference: &ClaimRef) -> (contains: bool)
    ensures contains == selected.deep_view().contains(reference@),
{
    let mut i = 0;
    while i < selected.len()
        invariant i <= selected.len(),
            forall|j: int| 0 <= j < i ==> selected.deep_view()[j] != reference@,
        decreases selected.len() - i,
    {
        let current = &selected[i];
        proof { assert(current@ == selected.deep_view()[i as int]); }
        if current == reference {
            proof { assert(selected.deep_view().contains(reference@)); }
            return true;
        }
        i += 1;
    }
    false
}
pub fn assess_portfolio(state: &PortfolioState, identity: &ContextId, query: &Context, reference: &ClaimRef)
    -> (result: Option<Assessment>)
    ensures result.deep_view() == if query_context_allowed_spec(state@, identity@, query@)
        && state@.selected.contains(reference@) { Some(assess_spec(state@.kb, query@, reference@)) } else { None },
{
    if query_context_allowed(state, identity, query) && selection_contains(state.selected(), reference) {
        Some(assess(state.kb(), query, reference))
    } else { None }
}
} // verus!
