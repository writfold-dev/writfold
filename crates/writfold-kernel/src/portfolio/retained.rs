//! Retained portfolio snapshots are unique and looked up in original order.
#[cfg(verus_keep_ghost)]
use super::model::*;
use crate::wellformed::context_wf;
#[cfg(verus_keep_ghost)]
use crate::wellformed::spec::context_wf_spec;
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub(super) fn retained_valid(retained: &[PortfolioState], snapshot: &SnapshotId) -> (valid: bool)
    ensures valid == retained_valid_spec(retained.deep_view(), snapshot@),
{
    let mut i = 0;
    while i < retained.len()
        invariant i <= retained.len(),
            forall|a: int, b: int| 0 <= b < a < i ==> retained.deep_view()[b].kb.snapshot != retained.deep_view()[a].kb.snapshot,
            forall|a: int| 0 <= a < i ==> retained.deep_view()[a].kb.snapshot != snapshot@,
        decreases retained.len() - i,
    {
        let current = &retained[i];
        proof { assert(current@ == retained.deep_view()[i as int]); }
        if current.kb().snapshot() == snapshot { return false; }
        let mut j = 0;
        while j < i
            invariant j <= i < retained.len(), current@ == retained.deep_view()[i as int],
                forall|a: int, b: int| 0 <= b < a < i ==> retained.deep_view()[b].kb.snapshot != retained.deep_view()[a].kb.snapshot,
                forall|a: int| 0 <= a < i ==> retained.deep_view()[a].kb.snapshot != snapshot@,
                forall|a: int| 0 <= a < j ==> retained.deep_view()[a].kb.snapshot != current@.kb.snapshot,
            decreases i - j,
        {
            let prior = &retained[j];
            proof { assert(prior@ == retained.deep_view()[j as int]); }
            if prior.kb().snapshot() == current.kb().snapshot() { return false; }
            j += 1;
        }
        i += 1;
    }
    true
}
pub(super) fn retained_target<'a>(retained: &'a [PortfolioState], snapshot: &SnapshotId)
    -> (found: Option<&'a PortfolioState>)
    ensures match found {
        None => forall|i: int| 0 <= i < retained.len() ==> #[trigger] retained.deep_view()[i].kb.snapshot != snapshot@,
        Some(target) => exists|i: int| 0 <= i < retained.len() && target@ == #[trigger] retained.deep_view()[i]
            && target@.kb.snapshot == snapshot@
            && forall|j: int| 0 <= j < i ==> #[trigger] retained.deep_view()[j].kb.snapshot != snapshot@,
    },
{
    let mut i = 0;
    while i < retained.len()
        invariant i <= retained.len(),
            forall|j: int| 0 <= j < i ==> retained.deep_view()[j].kb.snapshot != snapshot@,
        decreases retained.len() - i,
    {
        let target = &retained[i];
        proof { assert(target@ == retained.deep_view()[i as int]); }
        if target.kb().snapshot() == snapshot { return Some(target); }
        i += 1;
    }
    None
}
pub(super) fn contexts_wellformed(contexts: &[NamedContext]) -> (valid: bool)
    ensures valid == (forall|i: int| 0 <= i < contexts.len() ==> context_wf_spec(contexts.deep_view()[i].context)),
{
    let mut i = 0;
    proof { assert(contexts.deep_view().len() == contexts.len()); }
    while i < contexts.len()
        invariant i <= contexts.len(),
            contexts.deep_view().len() == contexts.len(),
            forall|j: int| 0 <= j < i ==> context_wf_spec(contexts.deep_view()[j].context),
        decreases contexts.len() - i,
    {
        let named = &contexts[i];
        proof { assert(named@ == contexts.deep_view()[i as int]); }
        if !context_wf(named.context()) {
            proof { assert(!context_wf_spec(contexts.deep_view()[i as int].context)); }
            return false;
        }
        i += 1;
    }
    true
}
} // verus!
