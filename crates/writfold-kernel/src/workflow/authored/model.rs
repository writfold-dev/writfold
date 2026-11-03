#![cfg(verus_keep_ghost)]
use super::gates::*;
use super::landing::*;
use super::problems::*;
use super::replacement::*;
use super::super::actor_present_spec;
use vstd::prelude::*;
use writfold_domain::*;
use writfold_domain::text::nonblank_spec;

verus! {

pub open spec fn decided(target: AuthoredStatus) -> bool {
    matches!(target, AuthoredStatus::Accepted | AuthoredStatus::Landing
        | AuthoredStatus::Applied | AuthoredStatus::Verified)
}

pub open spec fn needs_readiness(target: AuthoredStatus) -> bool {
    target == AuthoredStatus::Proposed || decided(target)
}

pub open spec fn needs_actor(target: AuthoredStatus) -> bool {
    matches!(target, AuthoredStatus::Accepted | AuthoredStatus::Rejected
        | AuthoredStatus::Landing | AuthoredStatus::Applied)
}

pub open spec fn needs_executor(target: AuthoredStatus) -> bool {
    matches!(target, AuthoredStatus::Landing | AuthoredStatus::Applied | AuthoredStatus::Verified)
}

pub open spec fn needs_landing(target: AuthoredStatus) -> bool {
    matches!(target, AuthoredStatus::Applied | AuthoredStatus::Verified)
}

pub open spec fn accepted_material(p: AuthoredProposalView) -> bool {
    actor_present_spec(p.accepted_revision) && p.accepted_revision == p.proposal_revision
}

pub open spec fn authored_edge_spec(current: AuthoredStatus, target: AuthoredStatus) -> bool {
    use AuthoredStatus::*;
    matches!((current, target), (Draft, Proposed) | (Proposed, Draft)
        | (Proposed, Accepted) | (Proposed, Rejected) | (Accepted, Landing)
        | (Accepted, Deferred) | (Landing, Applied) | (Applied, Verified)
        | (Applied, Regressed) | (Verified, Regressed))
        || (target == Superseded && current != target)
}

// A declarative judgment, independently of diagnostic order and rendering.
pub open spec fn authored_candidate_valid_spec(p: AuthoredProposalView,
    target: AuthoredStatus, now: int) -> bool {
    (decided(target) ==> accepted_material(p))
        && (target == AuthoredStatus::Superseded ==> authored_supersession_valid_spec(p))
        && (needs_actor(target) ==> actor_present_spec(p.actor))
        && (needs_readiness(target) ==> authored_ready_spec(p))
        && (decided(target) ==> actor_present_spec(p.acceptor))
        && (needs_executor(target) ==> actor_present_spec(p.executor))
        && (needs_landing(target) ==> explicit_landed(p) && resolved_landed(p)
            && bound_landed(p) && applicable_gates_pass(p, target, now))
        && (target == AuthoredStatus::Applied ==> has_phase(p, GateWhen::Land))
        && (target == AuthoredStatus::Verified ==>
            has_phase(p, GateWhen::Deploy) && has_phase(p, GateWhen::Use))
        && (target == AuthoredStatus::Regressed ==> nonblank_spec(p.regression_note))
}

pub open spec fn authored_candidate_problems_spec(p: AuthoredProposalView,
    target: AuthoredStatus, now: int) -> Seq<AuthoredProblem> {
    when_problem(decided(target) && !accepted_material(p), AuthoredProblem::AcceptedRevision)
        + when_problem(target == AuthoredStatus::Superseded && !authored_supersession_valid_spec(p),
            AuthoredProblem::Supersession)
        + when_problem(needs_actor(target) && !actor_present_spec(p.actor), AuthoredProblem::Actors)
        + when_problem(needs_readiness(target) && !authored_ready_spec(p), AuthoredProblem::Readiness)
        + when_problem(decided(target) && !actor_present_spec(p.acceptor), AuthoredProblem::Acceptor)
        + when_problem(needs_executor(target) && !actor_present_spec(p.executor), AuthoredProblem::Executor)
        + (if needs_landing(target) { landing_problems_spec(p, target, now) } else { Seq::empty() })
        + when_problem(target == AuthoredStatus::Regressed && !nonblank_spec(p.regression_note),
            AuthoredProblem::Regression)
}

pub proof fn problems_characterize_validity(p: AuthoredProposalView, target: AuthoredStatus, now: int)
    ensures (authored_candidate_problems_spec(p, target, now).len() == 0)
        == authored_candidate_valid_spec(p, target, now),
{
}

pub open spec fn authored_transition_spec(p: AuthoredProposalView, target: AuthoredStatus,
    now: int) -> bool {
    authored_edge_spec(p.status, target) && authored_candidate_valid_spec(p, target, now)
}

} // verus!
