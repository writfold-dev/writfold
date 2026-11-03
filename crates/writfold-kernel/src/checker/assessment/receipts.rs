use super::super::context::Frame;
use super::super::footprint::Evaluation;
use super::super::input::Input;
#[cfg(verus_keep_ghost)]
use super::super::assessment_model::*;
#[cfg(verus_keep_ghost)]
use super::super::input::dependency_value;
use vstd::prelude::*;
use writfold_domain::*;

verus! {

fn matching_dependencies(inputs: &[Input<'_>], dependencies: &[Dependency]) -> (matches: bool)
    ensures matches == dependencies_match(inputs.deep_view(), dependencies.deep_view()),
{
    if inputs.len() != dependencies.len() { return false; }
    let mut index = 0;
    while index < inputs.len()
        invariant index <= inputs.len(), inputs.len() == dependencies.len(),
            forall|i: int| 0 <= i < index ==> dependency_value(inputs.deep_view()[i]) == dependencies.deep_view()[i],
        decreases inputs.len() - index,
    {
        if !inputs[index].matches(&dependencies[index]) {
            proof { assert(dependency_value(inputs.deep_view()[index as int]) != dependencies.deep_view()[index as int]); }
            return false;
        }
        index += 1;
    }
    true
}

pub(super) fn current(context: Frame<'_>, reference: &ClaimRef, receipt: &Receipt) -> (current: bool)
    ensures current == receipt_current(context@, reference@, receipt@),
{
    receipt.claim() == reference && *receipt.result() == ReceiptResult::Pass
        && context.same_context(receipt.context())
        && receipt.context().now() <= context.now() && context.now() <= receipt.expires_at()
}

pub(super) fn reusable_with(
    context: Frame<'_>, reference: &ClaimRef, receipt: &Receipt,
    accepted: &[(&Witness, Evaluation<'_>)],
) -> (reusable: bool)
    ensures reusable == reusable_with_spec(context@, reference@, receipt@, accepted.deep_view()),
{
    if !current(context, reference, receipt) { return false; }
    let mut index = 0;
    while index < accepted.len()
        invariant index <= accepted.len(), receipt_current(context@, reference@, receipt@),
            forall|i: int| 0 <= i < index ==> !accepted_matches(accepted.deep_view()[i], receipt@),
        decreases accepted.len() - index,
    {
        let (witness, checked) = &accepted[index];
        if *witness == receipt.witness() && matching_dependencies(&checked.dependencies, receipt.dependencies()) {
            proof { assert(accepted_matches(accepted.deep_view()[index as int], receipt@)); }
            return true;
        }
        index += 1;
    }
    false
}

pub(super) fn validated(
    kb: &KB, context: Frame<'_>, reference: &ClaimRef, accepted: &[(&Witness, Evaluation<'_>)],
) -> (validated: bool)
    ensures validated == validated_spec(kb@, context@, reference@, accepted.deep_view()),
{
    let receipts = kb.receipts();
    let mut index = 0;
    while index < receipts.len()
        invariant index <= receipts.len(), receipts.deep_view() == kb@.receipts,
            forall|i: int| 0 <= i < index ==> !reusable_with_spec(context@, reference@, kb@.receipts[i], accepted.deep_view()),
        decreases receipts.len() - index,
    {
        if reusable_with(context, reference, &receipts[index], accepted) {
            proof { assert(reusable_with_spec(context@, reference@, kb@.receipts[index as int], accepted.deep_view())); }
            return true;
        }
        index += 1;
    }
    false
}

} // verus!
