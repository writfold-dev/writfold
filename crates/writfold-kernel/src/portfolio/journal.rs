//! First matching operation controls replay; model approvals are not IO capabilities.
#[cfg(verus_keep_ghost)]
use super::model::*;
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub(super) fn first_entry<'a>(journal: &'a [PortfolioJournalEntry], plan: &PortfolioPlan) -> (found: Option<&'a PortfolioPlan>)
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
pub(super) fn approval_present(approvals: &[ModelPortfolioApproval], plan: &PortfolioPlan) -> (present: bool)
    ensures present == (exists|i: int| 0 <= i < approvals.len() && approvals.deep_view()[i].plan == plan@),
{
    let mut i = 0;
    while i < approvals.len()
        invariant i <= approvals.len(), forall|j: int| 0 <= j < i ==> approvals.deep_view()[j].plan != plan@,
        decreases approvals.len() - i,
    {
        let approval = &approvals[i];
        proof { assert(approval@ == approvals.deep_view()[i as int]); }
        if approval.plan() == plan { return true; }
        i += 1;
    }
    false
}
pub(super) fn first_selection_entry<'a>(journal: &'a [PortfolioSelectionJournalEntry], plan: &PortfolioSelectionPlan)
    -> (found: Option<&'a PortfolioSelectionPlan>)
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
pub(super) fn selection_approval_present(approvals: &[ModelPortfolioSelectionApproval], plan: &PortfolioSelectionPlan)
    -> (present: bool)
    ensures present == (exists|i: int| 0 <= i < approvals.len() && approvals.deep_view()[i].plan == plan@),
{
    let mut i = 0;
    while i < approvals.len()
        invariant i <= approvals.len(), forall|j: int| 0 <= j < i ==> approvals.deep_view()[j].plan != plan@,
        decreases approvals.len() - i,
    {
        let approval = &approvals[i];
        proof { assert(approval@ == approvals.deep_view()[i as int]); }
        if approval.plan() == plan { return true; }
        i += 1;
    }
    false
}
} // verus!
