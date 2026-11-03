//! Ordered retained-state, journal, and approval scans for selection.
#[cfg(verus_keep_ghost)]
use super::admission_model::*;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;
use writfold_domain::*;

verus! {
pub(crate) fn selected_unique(selected: &[ClaimRef]) -> (valid: bool)
    ensures valid == selected.deep_view().no_duplicates(),
{
    let valid = super::unique(selected);
    proof {
        if valid {
            assert forall|a: int, b: int| 0 <= a < selected.len() && 0 <= b < selected.len()
                && a != b implies selected.deep_view()[a] != selected.deep_view()[b] by {
                if a < b { assert(!selected@[a].eq_spec(&selected@[b])); }
                else { assert(!selected@[b].eq_spec(&selected@[a])); }
            }
        } else {
            let (a, b) = choose|a: int, b: int| 0 <= a < b < selected.len()
                && selected@[a].eq_spec(&selected@[b]);
            assert(selected.deep_view()[a] == selected.deep_view()[b]);
        }
    }
    valid
}
pub(crate) fn references_present(kb: &KB, selected: &[ClaimRef]) -> (valid: bool)
    ensures valid == selected_present(kb@, selected.deep_view()),
{
    let mut i = 0;
    proof { assert(selected.deep_view().len() == selected.len()); }
    while i < selected.len()
        invariant i <= selected.len(),
            selected.deep_view().len() == selected.len(),
            forall|a: int| 0 <= a < i
                ==> #[trigger] crate::wellformed::spec::claim_present(kb@, selected.deep_view()[a]),
        decreases selected.len() - i,
    {
        let reference = &selected[i];
        proof { assert(reference@ == selected.deep_view()[i as int]); }
        if find_claim(kb, reference).is_none() {
            proof {
                assert(!crate::wellformed::spec::claim_present(kb@, reference@));
                if selected_present(kb@, selected.deep_view()) {
                    assert(crate::wellformed::spec::claim_present(kb@, selected.deep_view()[i as int]));
                }
            }
            return false;
        }
        i += 1;
    }
    true
}
pub(super) fn retained_valid(retained: &[KB], snapshot: &SnapshotId) -> (valid: bool)
    ensures valid == (retained_unique(retained.deep_view())
        && forall|i: int| 0 <= i < retained.len() ==> retained.deep_view()[i].snapshot != snapshot@),
{
    let mut i = 0;
    while i < retained.len()
        invariant i <= retained.len(),
            forall|a: int, b: int| 0 <= b < a < i ==> retained.deep_view()[b].snapshot != retained.deep_view()[a].snapshot,
            forall|a: int| 0 <= a < i ==> retained.deep_view()[a].snapshot != snapshot@,
        decreases retained.len() - i,
    {
        let current = &retained[i];
        proof { assert(current@ == retained.deep_view()[i as int]); }
        let mut j = 0;
        while j < i
            invariant j <= i < retained.len(), current@ == retained.deep_view()[i as int],
                forall|a: int, b: int| 0 <= b < a < i ==> retained.deep_view()[b].snapshot != retained.deep_view()[a].snapshot,
                forall|a: int| 0 <= a < i ==> retained.deep_view()[a].snapshot != snapshot@,
                forall|a: int| 0 <= a < j ==> retained.deep_view()[a].snapshot != current@.snapshot,
            decreases i - j,
        {
            let prior = &retained[j];
            proof { assert(prior@ == retained.deep_view()[j as int]); }
            if prior.snapshot() == current.snapshot() { return false; }
            j += 1;
        }
        if current.snapshot() == snapshot { return false; }
        i += 1;
    }
    true
}
pub(super) fn retained_target<'a>(retained: &'a [KB], snapshot: &SnapshotId) -> (found: Option<&'a KB>)
    ensures match found {
        None => forall|i: int| 0 <= i < retained.len() ==> #[trigger] retained.deep_view()[i].snapshot != snapshot@,
        Some(target) => exists|i: int| 0 <= i < retained.len() && target@ == #[trigger] retained.deep_view()[i]
            && target@.snapshot == snapshot@
            && forall|j: int| 0 <= j < i ==> #[trigger] retained.deep_view()[j].snapshot != snapshot@,
    },
{
    let mut i = 0;
    while i < retained.len()
        invariant i <= retained.len(),
            forall|j: int| 0 <= j < i ==> retained.deep_view()[j].snapshot != snapshot@,
        decreases retained.len() - i,
    {
        let target = &retained[i];
        proof { assert(target@ == retained.deep_view()[i as int]); }
        if target.snapshot() == snapshot { return Some(target); }
        i += 1;
    }
    None
}
pub(super) fn selected_exact(target: &KB, selected: &[ClaimRef]) -> (valid: bool)
    ensures valid == (selected.len() == target@.claims.len()
        && forall|i: int| 0 <= i < selected.len() ==> claim_matches_spec(target@.claims[i], selected.deep_view()[i])),
{
    let claims = target.claims();
    proof {
        assert(selected.deep_view().len() == selected.len());
        assert(claims.deep_view().len() == claims.len());
    }
    if selected.len() != claims.len() { return false; }
    let mut i = 0;
    while i < selected.len()
        invariant i <= selected.len(), selected.len() == claims.len(), claims.deep_view() == target@.claims,
            selected.deep_view().len() == selected.len(), claims.deep_view().len() == claims.len(),
            forall|j: int| 0 <= j < i ==> claim_matches_spec(target@.claims[j], selected.deep_view()[j]),
        decreases selected.len() - i,
    {
        let claim = &claims[i];
        proof { assert(claim@ == target@.claims[i as int]); }
        let reference = &selected[i];
        proof { assert(reference@ == selected.deep_view()[i as int]); }
        if !claim.matches_ref(reference) {
            proof { assert(!claim_matches_spec(target@.claims[i as int], selected.deep_view()[i as int])); }
            return false;
        }
        i += 1;
    }
    true
}
pub(super) fn approval_present(approvals: &[ModelSelectionApproval], plan: &SelectionPlan) -> (present: bool)
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
pub(super) fn first_entry<'a>(journal: &'a [SelectionJournalEntry], plan: &SelectionPlan) -> (found: Option<&'a SelectionPlan>)
    ensures found.deep_view() == first_selection_plan(journal.deep_view(), plan@),
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
pub(super) fn prepend_entry(journal: &[SelectionJournalEntry], plan: &SelectionPlan) -> (updated: Vec<SelectionJournalEntry>)
    ensures updated.deep_view() == seq![SelectionJournalEntryView { plan: plan@ }] + journal.deep_view(),
{
    let mut updated = Vec::with_capacity(journal.len().saturating_add(1));
    updated.push(SelectionJournalEntry::new(plan.clone()));
    let mut i = 0;
    proof { assert(journal.deep_view().len() == journal.len()); }
    while i < journal.len()
        invariant i <= journal.len(), journal.deep_view().len() == journal.len(),
            updated.deep_view() == seq![SelectionJournalEntryView { plan: plan@ }] + journal.deep_view().take(i as int),
        decreases journal.len() - i,
    {
        let ghost previous = updated.deep_view();
        let entry = journal[i].clone();
        proof { assert(entry@ == journal.deep_view()[i as int]); }
        updated.push(entry);
        proof {
            assert(updated.deep_view() =~= previous.push(entry@));
            assert(seq![SelectionJournalEntryView { plan: plan@ }] + journal.deep_view().take(i as int + 1)
                =~= previous.push(entry@));
        }
        i += 1;
    }
    proof { assert(journal.deep_view().take(journal.len() as int) == journal.deep_view()); }
    updated
}
}
