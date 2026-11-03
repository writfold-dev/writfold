//! Allocation-free scans used by the executable admission judgments.
#[cfg(verus_keep_ghost)]
use super::admission_model::*;
use crate::checker::{check_matches_snapshot, retracted, reusable, usable_for_snapshot};
use vstd::prelude::*;
use writfold_domain::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;

verus! {
pub(super) fn slice_prefix<T: PartialEq>(base: &[T], candidate: &[T]) -> (valid: bool)
    requires T::obeys_eq_spec(),
    ensures valid == (base.len() <= candidate.len()
        && forall|i: int| 0 <= i < base.len() ==> base@[i].eq_spec(&candidate@[i])),
{
    if base.len() > candidate.len() { return false; }
    let mut i = 0;
    while i < base.len()
        invariant i <= base.len() <= candidate.len(), T::obeys_eq_spec(),
            forall|j: int| 0 <= j < i ==> base@[j].eq_spec(&candidate@[j]),
        decreases base.len() - i,
    {
        if base[i] != candidate[i] { return false; }
        i += 1;
    }
    true
}
pub(super) fn actor_authorized(context: &Context, actor: &ActorId) -> (valid: bool)
    ensures valid == context@.policy.authorized_actors.contains(actor@),
{
    let actors = context.policy().authorized_actors();
    let mut i = 0;
    while i < actors.len()
        invariant i <= actors.len(), actors.deep_view() == context@.policy.authorized_actors,
            forall|j: int| 0 <= j < i ==> actors.deep_view()[j] != actor@,
        decreases actors.len() - i,
    {
        if &actors[i] == actor { return true; }
        i += 1;
    }
    false
}
pub(super) fn target_derivation(plan: &Plan) -> (valid: bool)
    ensures valid == (exists|i: int| 0 <= i < plan@.candidate.derivations.len()
        && plan@.candidate.derivations[i].claim == plan@.claim
        && plan@.candidate.derivations[i].witness == plan@.witness),
{
    let rows = plan.candidate().derivations();
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(), rows.deep_view() == plan@.candidate.derivations,
            forall|j: int| 0 <= j < i ==> !(rows.deep_view()[j].claim == plan@.claim
                && rows.deep_view()[j].witness == plan@.witness),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        proof { assert(row@ == rows.deep_view()[i as int]); }
        if row.claim() == plan.claim() && row.witness() == plan.witness() { return true; }
        i += 1;
    }
    false
}
fn added_claim(candidate: &KB, context: &Context, claim: &Claim) -> (valid: bool)
    ensures valid == added_claim_spec(candidate@, context@, claim@),
{
    let rows = candidate.derivations();
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(), rows.deep_view() == candidate@.derivations,
            forall|j: int| 0 <= j < i ==> !added_derivation_spec(candidate@, context@, claim@, rows.deep_view()[j]),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        proof { assert(row@ == rows.deep_view()[i as int]); }
        if claim.matches_ref(row.claim()) && !retracted(candidate, context, row.claim())
            && check_matches_snapshot(candidate, context, candidate.snapshot(), row.witness(), claim.statement())
            && usable_for_snapshot(candidate, context, candidate.snapshot(), row.claim()) { return true; }
        i += 1;
    }
    false
}
pub(super) fn added_claims(candidate: &KB, context: &Context, start: usize) -> (valid: bool)
    requires start <= candidate@.claims.len(),
    ensures valid == (forall|i: int| start <= i < candidate@.claims.len()
        ==> added_claim_spec(candidate@, context@, candidate@.claims[i])),
{
    let claims = candidate.claims();
    let mut i = start;
    while i < claims.len()
        invariant start <= i <= claims.len(), claims.deep_view() == candidate@.claims,
            forall|j: int| start <= j < i ==> added_claim_spec(candidate@, context@, claims.deep_view()[j]),
        decreases claims.len() - i,
    {
        let claim = &claims[i];
        proof { assert(claim@ == claims.deep_view()[i as int]); }
        if !added_claim(candidate, context, claim) { return false; }
        i += 1;
    }
    true
}
pub(super) fn approval_present(approvals: &[ModelApproval], plan: &Plan) -> (present: bool)
    ensures present == (exists|i: int| 0 <= i < approvals.len() && approvals.deep_view()[i].plan == plan@),
{
    let mut i = 0;
    while i < approvals.len()
        invariant i <= approvals.len(),
            forall|j: int| 0 <= j < i ==> approvals.deep_view()[j].plan != plan@,
        decreases approvals.len() - i,
    {
        let approval = &approvals[i];
        proof { assert(approval@ == approvals.deep_view()[i as int]); }
        if approval.plan() == plan { return true; }
        i += 1;
    }
    false
}
pub(super) fn first_entry<'a>(journal: &'a [JournalEntry], plan: &Plan) -> (found: Option<&'a Plan>)
    ensures found.deep_view() == first_plan(journal.deep_view(), plan@),
{
    let mut i = 0;
    while i < journal.len()
        invariant i <= journal.len(),
            forall|j: int| 0 <= j < i ==> journal.deep_view()[j].plan.id != plan@.id,
        decreases journal.len() - i,
    {
        let entry = &journal[i];
        proof { assert(entry@ == journal.deep_view()[i as int]); }
        if entry.plan().id() == plan.id() {
            proof {
                assert(exists|k: int| 0 <= k < journal.len() && journal.deep_view()[k].plan.id == plan@.id);
                assert(exists|k: int| 0 <= k < journal.len() && journal.deep_view()[k].plan.id == plan@.id
                    && forall|j: int| 0 <= j < k ==> journal.deep_view()[j].plan.id != plan@.id);
                let k = choose|k: int| 0 <= k < journal.len() && journal.deep_view()[k].plan.id == plan@.id
                    && forall|j: int| 0 <= j < k ==> journal.deep_view()[j].plan.id != plan@.id;
                assert(k == i);
            }
            return Some(entry.plan());
        }
        i += 1;
    }
    None
}
fn gate_usable(kb: &KB, context: &Context, target: &ClaimRef, gate: &ReceiptId) -> (valid: bool)
    ensures valid == gate_reusable(kb@, context@, target@, gate@),
{
    let receipts = kb.receipts();
    let mut i = 0;
    while i < receipts.len()
        invariant i <= receipts.len(), receipts.deep_view() == kb@.receipts,
            forall|j: int| 0 <= j < i ==> kb@.receipts[j].id != gate@,
        decreases receipts.len() - i,
    {
        let receipt = &receipts[i];
        proof { assert(receipt@ == kb@.receipts[i as int]); }
        if receipt.id() == gate {
            let valid = reusable(kb, context, target, receipt);
            proof {
                assert forall|j: int| 0 <= j < receipts.len() && kb@.receipts[j].id == gate@
                    && (forall|k: int| 0 <= k < j ==> kb@.receipts[k].id != gate@)
                    implies j == i by { }
            }
            return valid;
        }
        i += 1;
    }
    false
}
pub(super) fn gates_usable(kb: &KB, context: &Context, proposal: &Proposal) -> (valid: bool)
    ensures valid == (forall|i: int| 0 <= i < proposal@.gates.len()
        ==> gate_reusable(kb@, context@, proposal@.target, proposal@.gates[i])),
{
    let gates = proposal.gates();
    let mut i = 0;
    while i < gates.len()
        invariant i <= gates.len(), gates.deep_view() == proposal@.gates,
            forall|j: int| 0 <= j < i ==> gate_reusable(kb@, context@, proposal@.target, proposal@.gates[j]),
        decreases gates.len() - i,
    {
        if !gate_usable(kb, context, proposal.target(), &gates[i]) { return false; }
        i += 1;
    }
    true
}
}
