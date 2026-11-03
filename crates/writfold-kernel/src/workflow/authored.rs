//! Authored decisions over deep domain models, preserving the Python workflow
//! diagnostics and the retained lifecycle. Unicode nonblank uses WF-SEM-001;
//! LegacyLimited waives readiness only, never material or landing bindings.
mod gates;
mod landing;
mod model;
mod problems;
mod replacement;

pub use gates::{authored_gate_passes, authored_gate_valid, authored_ready};
#[cfg(verus_keep_ghost)]
pub use gates::{authored_gate_passes_spec, authored_gate_valid_spec, authored_ready_spec};
#[cfg(verus_keep_ghost)]
pub use model::*;
pub use problems::AuthoredProblem;
#[cfg(verus_keep_ghost)]
pub use problems::problem_text;
pub use replacement::authored_supersession_valid;
#[cfg(verus_keep_ghost)]
pub use replacement::authored_supersession_valid_spec;
use gates::material_present;
use landing::visit_landing_problems;
use problems::ProblemReducer;
use super::actor_present;
use writfold_domain::{Integer, *};
use vstd::prelude::*;

verus! {

pub fn authored_edge(current: AuthoredStatus, target: AuthoredStatus) -> (legal: bool)
    ensures legal == authored_edge_spec(current, target),
{
    use AuthoredStatus::*;
    matches!(
        (current, target),
        (Draft, Proposed)
            | (Proposed, Draft)
            | (Proposed, Accepted)
            | (Proposed, Rejected)
            | (Accepted, Landing)
            | (Accepted, Deferred)
            | (Landing, Applied)
            | (Applied, Verified)
            | (Applied, Regressed)
            | (Verified, Regressed)
    ) || (target == Superseded && current != target)
}

pub fn authored_transition(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> (valid: bool)
    ensures valid == authored_transition_spec(proposal@, target, now@),
{
    authored_edge(*proposal.status(), target) && authored_candidate_valid(proposal, target, now)
}

pub fn authored_candidate_problems(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> (problems: Vec<&'static str>)
    ensures
        problems@.len() == authored_candidate_problems_spec(proposal@, target, now@).len(),
        forall|i: int| 0 <= i < problems@.len() ==>
            problems@[i]@ == problem_text(authored_candidate_problems_spec(proposal@, target, now@)[i]),
{
    let reducer = candidate_decision(proposal, target, now, true);
    reducer.problems
}

pub fn authored_candidate_valid(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> (valid: bool)
    ensures valid == authored_candidate_valid_spec(proposal@, target, now@),
{
    let reducer = candidate_decision(proposal, target, now, false);
    proof { problems_characterize_validity(proposal@, target, now@); }
    reducer.valid
}

// The typed reducer replaces the callback, not the decision algorithm. It
// records the same ordered failures in diagnostic mode and only a bool otherwise.
fn candidate_decision(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
    collect: bool,
) -> (problems: ProblemReducer)
    ensures problems.invariant(), problems.collect == collect,
        problems.trace@ == authored_candidate_problems_spec(proposal@, target, now@),
{
    use AuthoredStatus::*;
    let mut problems = ProblemReducer::new(collect);
    let decided = matches!(target, Accepted | Landing | Applied | Verified);
    if decided
        && (!material_present(proposal.accepted_revision())
            || proposal.accepted_revision() != proposal.proposal_revision())
    {
        problems.emit(AuthoredProblem::AcceptedRevision);
    }
    if target == Superseded && !authored_supersession_valid(proposal) {
        problems.emit(AuthoredProblem::Supersession);
    }
    if matches!(target, Accepted | Rejected | Landing | Applied) && !actor_present(proposal.actor())
    {
        problems.emit(AuthoredProblem::Actors);
    }
    if !authored_ready(proposal)
        && matches!(target, Proposed | Accepted | Landing | Applied | Verified)
    {
        problems.emit(AuthoredProblem::Readiness);
    }
    if decided && !actor_present(proposal.acceptor()) {
        problems.emit(AuthoredProblem::Acceptor);
    }
    if matches!(target, Landing | Applied | Verified) && !actor_present(proposal.executor()) {
        problems.emit(AuthoredProblem::Executor);
    }
    if matches!(target, Applied | Verified) {
        visit_landing_problems(proposal, target, now, &mut problems);
    }
    if target == Regressed && !writfold_domain::text::nonblank(proposal.regression_note()) {
        problems.emit(AuthoredProblem::Regression);
    }
    problems
}

} // verus!
