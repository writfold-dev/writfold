//! Allocation-free scans used by the executable admission judgments.
#[cfg(verus_keep_ghost)]
use super::admission_model::*;
use crate::checker::{check_matches_snapshot, retracted, reusable, usable_for_snapshot};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;
use writfold_domain::*;

verus! {
pub(super) proof fn prefix_views<T: PartialEq + DeepView>(base: Seq<T>, candidate: Seq<T>)
    requires forall|a: T, b: T| #[trigger] a.eq_spec(&b) == (a.deep_view() == b.deep_view()),
    ensures (base.len() <= candidate.len()
        && forall|i: int| 0 <= i < base.len() ==> base[i].eq_spec(&candidate[i]))
        == prefix(Seq::new(base.len(), |i: int| base[i].deep_view()),
            Seq::new(candidate.len(), |i: int| candidate[i].deep_view())),
{
    let left = Seq::new(base.len(), |i: int| base[i].deep_view());
    let right = Seq::new(candidate.len(), |i: int| candidate[i].deep_view());
    assert(left.len() == base.len());
    assert(right.len() == candidate.len());
    if base.len() <= candidate.len() {
        if (forall|i: int| 0 <= i < base.len() ==> base[i].eq_spec(&candidate[i])) {
            assert forall|i: int| 0 <= i < left.len() implies left[i] == right[i] by {
                assert(base[i].eq_spec(&candidate[i]));
                assert(base[i].deep_view() == candidate[i].deep_view());
            }
        }
        if prefix(left, right) {
            assert forall|i: int| 0 <= i < base.len() implies base[i].eq_spec(&candidate[i]) by {
                assert(left[i] == right[i]);
                assert(base[i].deep_view() == candidate[i].deep_view());
            }
        }
    }
}
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
pub(crate) fn actor_authorized(context: &Context, actor: &ActorId) -> (valid: bool)
    ensures valid == context@.policy.authorized_actors.contains(actor@),
{
    let actors = context.policy().authorized_actors();
    let mut i = 0;
    proof { assert(actors.deep_view().len() == actors.len()); }
    while i < actors.len()
        invariant i <= actors.len(), actors.deep_view() == context@.policy.authorized_actors,
            actors.deep_view().len() == actors.len(),
            forall|j: int| 0 <= j < i ==> actors.deep_view()[j] != actor@,
        decreases actors.len() - i,
    {
        let current = &actors[i];
        proof { assert(current@ == actors.deep_view()[i as int]); }
        if current == actor {
            proof { assert(context@.policy.authorized_actors.contains(actor@)); }
            return true;
        }
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
                reveal(first_plan);
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
    proof { reveal(first_plan); }
    None
}
pub(super) fn prepend_entry(journal: &[JournalEntry], plan: &Plan) -> (updated: Vec<JournalEntry>)
    ensures updated.deep_view() == seq![JournalEntryView { plan: plan@ }] + journal.deep_view(),
{
    let mut updated = Vec::with_capacity(journal.len().saturating_add(1));
    updated.push(JournalEntry::new(plan.clone()));
    let mut i = 0;
    proof { assert(journal.deep_view().len() == journal.len()); }
    while i < journal.len()
        invariant i <= journal.len(), journal.deep_view().len() == journal.len(),
            updated.deep_view() == seq![JournalEntryView { plan: plan@ }] + journal.deep_view().take(i as int),
        decreases journal.len() - i,
    {
        let ghost previous = updated.deep_view();
        let entry = journal[i].clone();
        proof { assert(entry@ == journal.deep_view()[i as int]); }
        updated.push(entry);
        proof {
            assert(updated.deep_view() =~= previous.push(entry@));
            assert(seq![JournalEntryView { plan: plan@ }] + journal.deep_view().take(i as int + 1)
                =~= previous.push(entry@));
        }
        i += 1;
    }
    proof { assert(journal.deep_view().take(journal.len() as int) == journal.deep_view()); }
    updated
}
fn first_gate_receipt<'a>(receipts: &'a [Receipt], gate: &ReceiptId) -> (found: Option<&'a Receipt>)
    ensures match found {
        None => forall|i: int| 0 <= i < receipts.len() ==> receipts.deep_view()[i].id != gate@,
        Some(receipt) => exists|i: int| 0 <= i < receipts.len() && receipt@ == receipts.deep_view()[i]
            && receipt@.id == gate@ && forall|j: int| 0 <= j < i ==> receipts.deep_view()[j].id != gate@,
    },
        found.deep_view() == crate::checker::first_matching(receipts.deep_view(), |receipt: ReceiptView| receipt.id == gate@),
{
    let mut i = 0;
    proof { assert(receipts.deep_view().len() == receipts.len()); }
    while i < receipts.len()
        invariant i <= receipts.len(), receipts.deep_view().len() == receipts.len(),
            forall|j: int| 0 <= j < i ==> receipts.deep_view()[j].id != gate@,
        decreases receipts.len() - i,
    {
        let receipt = &receipts[i];
        proof { assert(receipt@ == receipts.deep_view()[i as int]); }
        if receipt.id() == gate {
            proof { crate::checker::first_matching_at(receipts.deep_view(), |row: ReceiptView| row.id == gate@, i as int); }
            return Some(receipt);
        }
        i += 1;
    }
    proof { crate::checker::first_matching_absent(receipts.deep_view(), |row: ReceiptView| row.id == gate@); }
    None
}
fn gate_usable(kb: &KB, context: &Context, target: &ClaimRef, gate: &ReceiptId) -> (valid: bool)
    ensures valid == gate_reusable(kb@, context@, target@, gate@),
{
    let receipts = kb.receipts();
    proof { assert(receipts.deep_view().len() == receipts.len()); }
    let Some(receipt) = first_gate_receipt(receipts, gate) else {
        proof { reveal(gate_reusable); }
        return false;
    };
    proof {
        let i = choose|i: int| 0 <= i < receipts.len() && receipt@ == receipts.deep_view()[i]
            && receipt@.id == gate@ && forall|j: int| 0 <= j < i ==> receipts.deep_view()[j].id != gate@;
        first_gate_decides(kb@, context@, target@, gate@, i);
    }
    reusable(kb, context, target, receipt)
}
pub(super) fn gates_usable(kb: &KB, context: &Context, proposal: &Proposal) -> (valid: bool)
    ensures valid == (forall|i: int| 0 <= i < proposal@.gates.len()
        ==> gate_reusable(kb@, context@, proposal@.target, proposal@.gates[i])),
{
    let gates = proposal.gates();
    let mut i = 0;
    proof { assert(gates.deep_view().len() == gates.len()); }
    while i < gates.len()
        invariant i <= gates.len(), gates.deep_view() == proposal@.gates,
            gates.deep_view().len() == gates.len(),
            forall|j: int| 0 <= j < i ==> gate_reusable(kb@, context@, proposal@.target, proposal@.gates[j]),
        decreases gates.len() - i,
    {
        let gate = &gates[i];
        proof { assert(gate@ == proposal@.gates[i as int]); }
        if !gate_usable(kb, context, proposal.target(), gate) {
            proof { assert(!gate_reusable(kb@, context@, proposal@.target, proposal@.gates[i as int])); }
            return false;
        }
        i += 1;
    }
    true
}
}
