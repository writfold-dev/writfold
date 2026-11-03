//! Pure admission, replay and authored judgments from the reference workflow.
mod admission;
#[cfg(verus_keep_ghost)]
pub mod admission_model;
mod authored;
pub mod result;
mod selection;
mod selection_scan;
use crate::checker::{
    check_matches_snapshot, context_matches, retracted, usable_for, usable_for_snapshot,
};
use crate::wellformed::well_formed;
pub(crate) use admission::actor_authorized;
#[cfg(verus_keep_ghost)]
use admission_model::*;
pub use authored::*;
use result::replay_outcome;
pub use result::{Application, SelectionResult};
pub use selection::*;
pub(crate) use selection_scan::{references_present, selected_unique};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;
use writfold_domain::*;

verus! {
pub fn preserves_prefix(base: &KB, candidate: &KB) -> (valid: bool)
    ensures valid == preserves_prefix_spec(base@, candidate@),
{
    let observations = (base.observations(), candidate.observations());
    let claims = (base.claims(), candidate.claims());
    let attestations = (base.attestations(), candidate.attestations());
    let enumerations = (base.enumerations(), candidate.enumerations());
    let derivations = (base.derivations(), candidate.derivations());
    let corrections = (base.corrections(), candidate.corrections());
    let receipts = (base.receipts(), candidate.receipts());
    proof {
        admission::prefix_views(observations.0@, observations.1@);
        admission::prefix_views(claims.0@, claims.1@);
        admission::prefix_views(attestations.0@, attestations.1@);
        admission::prefix_views(enumerations.0@, enumerations.1@);
        admission::prefix_views(derivations.0@, derivations.1@);
        admission::prefix_views(corrections.0@, corrections.1@);
        admission::prefix_views(receipts.0@, receipts.1@);
    }
    admission::slice_prefix(observations.0, observations.1)
        && admission::slice_prefix(claims.0, claims.1)
        && admission::slice_prefix(attestations.0, attestations.1)
        && admission::slice_prefix(enumerations.0, enumerations.1)
        && admission::slice_prefix(derivations.0, derivations.1)
        && admission::slice_prefix(corrections.0, corrections.1)
        && admission::slice_prefix(receipts.0, receipts.1)
}
pub(crate) fn kb_at(kb: &KB, snapshot: &SnapshotId) -> (result: KB)
    ensures result@ == kb_at_spec(kb@, snapshot@),
{
    proof { broadcast use vstd::std_specs::vec::group_vec_axioms; }
    KB::new(snapshot.clone(), kb.observations().clone(), kb.claims().clone(),
        kb.attestations().clone(), kb.enumerations().clone(), kb.derivations().clone(),
        kb.corrections().clone(), kb.receipts().clone())
}
pub(crate) fn unique<T: PartialEq>(items: &[T]) -> (valid: bool)
    requires T::obeys_eq_spec(),
    ensures valid == (forall|a: int, b: int| 0 <= a < b < items.len()
        ==> !items@[a].eq_spec(&items@[b])),
{
    let mut i = 0;
    while i < items.len()
        invariant i <= items.len(), T::obeys_eq_spec(),
            forall|a: int, b: int| 0 <= a < b < i ==> !items@[a].eq_spec(&items@[b]),
        decreases items.len() - i,
    {
        let mut j = 0;
        while j < i
            invariant j <= i < items.len(), T::obeys_eq_spec(),
                forall|a: int, b: int| 0 <= a < b < i ==> !items@[a].eq_spec(&items@[b]),
                forall|a: int| 0 <= a < j ==> !items@[a].eq_spec(&items@[i as int]),
            decreases i - j,
        {
            if items[j] == items[i] { return false; }
            j += 1;
        }
        i += 1;
    }
    true
}
pub fn plan_valid(kb: &KB, context: &Context, plan: &Plan) -> (valid: bool)
    ensures valid == plan_valid_spec(kb@, context@, plan@),
{
    proof { reveal(plan_valid_spec); }
    if !context_matches(kb, context) || !well_formed(plan.candidate())
        || plan.candidate().snapshot() == kb.snapshot() || !preserves_prefix(kb, plan.candidate()) {
        return false;
    }
    let Some(claim) = find_claim(plan.candidate(), plan.claim()) else { return false; };
    proof {
        assert forall|other: ClaimView|
            first_claim_match(plan@.candidate.claims, plan@.claim, Some(other)) implies other == claim@ by {
            let i = choose|i: int| 0 <= i < plan@.candidate.claims.len()
                && other == plan@.candidate.claims[i] && claim_matches_spec(other, plan@.claim)
                && forall|j: int| 0 <= j < i ==> !claim_matches_spec(plan@.candidate.claims[j], plan@.claim);
            let k = choose|k: int| 0 <= k < plan@.candidate.claims.len()
                && claim@ == plan@.candidate.claims[k] && claim_matches_spec(claim@, plan@.claim)
                && forall|j: int| 0 <= j < k ==> !claim_matches_spec(plan@.candidate.claims[j], plan@.claim);
            assert(i == k);
        }
    }
    let authority = plan.authority();
    let candidate = plan.candidate();
    let added = &candidate.claims()[kb.claims().len()..];
    proof { assert(added.deep_view() =~= candidate@.claims.subrange(kb@.claims.len() as int, candidate@.claims.len() as int)); }
    let approvals = authority.approvals();
    proof { admission::prefix_views(approvals@, added@); }
    if !writfold_domain::text::nonblank(authority.actor().as_str())
        || authority.target() != plan.claim() || !admission::actor_authorized(context, authority.actor())
        || authority.statement() != claim.statement() || authority.policy() != context.policy().id()
        || !authority.intended_use().eq(context.intended_use()) || authority.context() != context
        || authority.approved() != candidate || approvals.len() != added.len()
        || !admission::slice_prefix(approvals, added) { return false; }
    proof { assert(authority@.approvals =~= added.deep_view()); }
    let valid = !retracted(candidate, context, plan.claim()) && admission::target_derivation(plan)
        && check_matches_snapshot(candidate, context, candidate.snapshot(), plan.witness(), claim.statement())
        && usable_for_snapshot(candidate, context, candidate.snapshot(), plan.claim())
        && admission::added_claims(candidate, context, kb.claims().len());
    proof {
        if valid {
            assert(first_claim_match(candidate@.claims, plan@.claim, Some(claim@)));
            assert(exists|found: ClaimView| first_claim_match(candidate@.claims, plan@.claim, Some(found))
                && authority@.statement == found.statement
                && crate::checker::spec::check_matches_snapshot_spec(candidate@, context@,
                    candidate@.snapshot, plan@.witness, found.statement));
        }
    }
    valid
}
pub fn apply_plan<'a>(kb: &'a KB, context: &Context, journal: &'a [JournalEntry], plan: &Plan,
    trusted_approvals: &[ModelApproval]) -> (result: Application<'a>)
    ensures result@ == apply_plan_spec(kb@, context@, journal.deep_view(), plan@, trusted_approvals.deep_view()),
{
    if let Some(recorded) = admission::first_entry(journal, plan) {
        return Application::unchanged(kb, journal, replay_outcome(recorded, plan));
    }
    if plan.expected_snapshot() != kb.snapshot() {
        return Application::unchanged(kb, journal, CommitOutcome::PredecessorConflict);
    }
    if !admission::approval_present(trusted_approvals, plan) || !plan_valid(kb, context, plan) {
        return Application::unchanged(kb, journal, CommitOutcome::Rejected);
    }
    proof { assert(apply_plan_spec(kb@, context@, journal.deep_view(), plan@, trusted_approvals.deep_view()).outcome
        == CommitOutcome::Applied); }
    let updated = admission::prepend_entry(journal, plan);
    Application::applied(plan.candidate().clone(), updated)
}
pub fn transition(kb: &KB, context: &Context, proposal: &Proposal, target: ProposalStatus) -> (valid: bool)
    ensures valid == transition_spec(kb@, context@, proposal@, target),
{
    use ProposalStatus::*;
    let legal = matches!((*proposal.status(), target), (Draft, Proposed) | (Proposed, Approved)
        | (Proposed, Rejected) | (Approved, Applied) | (Approved, Rejected) | (Applied, Verified));
    if !legal || !actor_present(proposal.actor()) || !usable_for(kb, context, proposal.target()) { return false; }
    if !admission::gates_usable(kb, context, proposal) { return false; }
    if matches!(target, Approved | Applied | Verified) && proposal.gates().is_empty() { return false; }
    if target == Applied && !actor_present(proposal.executor()) { return false; }
    target != Verified || (actor_present(proposal.executor()) && actor_present(proposal.acceptor())
        && proposal.executor() != proposal.acceptor())
}
pub fn conclude(investigation: &Investigation) -> (valid: bool)
    ensures valid == conclude_spec(investigation@),
{
    if !writfold_domain::text::nonblank(investigation.checked_against())
        || !writfold_domain::text::nonblank(investigation.scope_and_limits()) { return false; }
    let hypotheses = investigation.hypotheses();
    let mut i = 0;
    while i < hypotheses.len()
        invariant i <= hypotheses.len(), hypotheses.deep_view() == investigation@.hypotheses,
            forall|j: int| 0 <= j < i ==> writfold_domain::text::nonblank_spec(hypotheses.deep_view()[j].text)
                && hypotheses.deep_view()[j].status != HypothesisStatus::Open,
        decreases hypotheses.len() - i,
    {
        let hypothesis = &hypotheses[i];
        proof { assert(hypothesis@ == hypotheses.deep_view()[i as int]); }
        if !writfold_domain::text::nonblank(hypothesis.text()) || *hypothesis.status() == HypothesisStatus::Open {
            return false;
        }
        i += 1;
    }
    true
}
pub open spec fn actor_present_spec(actor: Option<Seq<char>>) -> bool {
    match actor { Some(value) => writfold_domain::text::nonblank_spec(value), None => false }
}
pub(crate) fn actor_present(actor: &Option<ActorId>) -> (present: bool)
    ensures present == actor_present_spec(actor.deep_view()),
{
    match actor { Some(value) => writfold_domain::text::nonblank(value.as_str()), None => false }
}
}
