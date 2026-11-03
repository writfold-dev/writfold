//! Assessment decision contracts; diagnostic rendering remains ordinary Rust.
mod freshness;
mod receipts;

use super::context::Frame;
use super::evaluation::alternatives;
use super::supported_frame;
#[cfg(verus_keep_ghost)]
use super::assessment_model::*;
#[cfg(verus_keep_ghost)]
use super::lookup::claim_lookup_value;
use freshness::alternative_freshness;
pub use freshness::witness_freshness;
use receipts::{current, reusable_with, validated};
use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub fn reusable(kb: &KB, context: &Context, reference: &ClaimRef, receipt: &Receipt) -> (reusable: bool)
    ensures reusable == reusable_spec(kb@, context@, reference@, receipt@),
{
    let frame = Frame::new(context);
    if !current(frame, reference, receipt) { return false; }
    reusable_with(frame, reference, receipt, &alternatives(kb, frame, reference))
}

pub fn policy_allows(
    supported_positive: bool,
    fresh: bool,
    validated: bool,
    uncontested: bool,
    require_fresh: bool,
    require_validated: bool,
    require_uncontested: bool,
) -> (allowed: bool)
    ensures allowed == policy_allows_spec(supported_positive, fresh, validated, uncontested,
        require_fresh, require_validated, require_uncontested),
{
    supported_positive
        && (!require_fresh || fresh)
        && (!require_validated || validated)
        && (!require_uncontested || uncontested)
}

fn disputed(kb: &KB, context: Frame<'_>, reference: &ClaimRef) -> (disputed: bool)
    ensures disputed == disputed_spec(kb@, context@.base, reference@),
{
    let corrections = kb.corrections();
    let mut index = 0;
    while index < corrections.len()
        invariant index <= corrections.len(), corrections.deep_view() == kb@.corrections,
            forall|i: int| 0 <= i < index ==> !dispute_matches(context@.base, reference@, kb@.corrections[i]),
        decreases corrections.len() - index,
    {
        let correction = &corrections[index];
        if correction.target() == reference && *correction.kind() == CorrectionKind::Dispute
            && correction.recorded_at() <= context.now() {
            proof { assert(dispute_matches(context@.base, reference@, kb@.corrections[index as int])); }
            return true;
        }
        index += 1;
    }
    false
}

pub fn assess(kb: &KB, context: &Context, reference: &ClaimRef) -> (assessment: Assessment)
    ensures assessment@ == assess_spec(kb@, context@, reference@),
{
    assess_frame(kb, Frame::new(context), reference)
}

fn assess_frame(kb: &KB, context: Frame<'_>, reference: &ClaimRef) -> (assessment: Assessment)
    ensures assessment@ == assess_frame_spec(kb@, context@, reference@),
{
    proof { broadcast use claim_lookup_value; }
    let claim = find_claim(kb, reference);
    let accepted = alternatives(kb, context, reference);
    let selected = !accepted.is_empty();
    let selected_negative = match claim {
        Some(value) => *value.statement().polarity() == Polarity::Negative,
        None => false,
    };
    let contrary = supported_frame(kb, context, reference,
        if selected_negative { Polarity::Positive } else { Polarity::Negative });
    let (positive, negative) = if selected_negative {
        (contrary, selected)
    } else {
        (selected, contrary)
    };
    let support = match (positive, negative) {
        (true, true) => Support::Both,
        (true, false) => Support::Positive,
        (false, true) => Support::Negative,
        (false, false) => Support::None,
    };
    let applicable = match claim {
        Some(value) => context.matches(kb) && narrower(context.target(), value.statement().scope()),
        None => false,
    };
    let freshness = alternative_freshness(context, &accepted);
    let validated = validated(kb, context, reference, &accepted);
    let contested = contrary || disputed(kb, context, reference);
    let usable = applicable
        && policy_allows(
            selected,
            freshness == Freshness::Fresh,
            validated,
            !contested,
            *context.policy().require_fresh(),
            *context.policy().require_validation(),
            *context.policy().require_uncontested(),
        );
    Assessment::new(support, applicable, freshness, validated, contested, positive || negative, usable)
}

pub fn usable_for(kb: &KB, context: &Context, reference: &ClaimRef) -> (usable: bool)
    ensures usable == usable_for_spec(kb@, context@, reference@),
{
    *assess(kb, context, reference).usable()
}

pub fn usable_for_snapshot(
    kb: &KB,
    context: &Context,
    snapshot: &SnapshotId,
    reference: &ClaimRef,
) -> (usable: bool)
    ensures usable == usable_for_snapshot_spec(kb@, context@, snapshot@, reference@),
{
    *assess_frame(kb, Frame { base: context, snapshot }, reference).usable()
}

} // verus!

pub fn assessment_gaps(kb: &KB, context: &Context, reference: &ClaimRef) -> Vec<String> {
    let verdict = assess(kb, context, reference);
    let selected = find_claim(kb, reference).is_some_and(|claim| {
        matches!(
            (claim.statement().polarity(), verdict.support()),
            (_, Support::Both)
                | (Polarity::Positive, Support::Positive)
                | (Polarity::Negative, Support::Negative)
        )
    });
    let mut gaps = Vec::new();
    if !verdict.applicable() {
        gaps.push("claim is not applicable to the context target".to_owned());
    }
    if !selected {
        let support = match verdict.support() {
            Support::None => "none",
            Support::Positive => "positive",
            Support::Negative => "negative",
            Support::Both => "both",
        };
        gaps.push(format!(
            "no accepted witness of the claim's polarity (support={support})"
        ));
    }
    if *context.policy().require_fresh() && *verdict.freshness() != Freshness::Fresh {
        let freshness = match verdict.freshness() {
            Freshness::Fresh => "fresh",
            Freshness::Stale => "stale",
            Freshness::Unknown => "unknown",
        };
        gaps.push(format!("observations are {freshness} under policy ttl"));
    }
    if *context.policy().require_validation() && !verdict.validated() {
        gaps.push("no reusable validation receipt".to_owned());
    }
    if *context.policy().require_uncontested() && *verdict.contested() {
        gaps.push("claim is contested by contrary evidence or dispute".to_owned());
    }
    gaps
}
