//! Receipt identity, independent assessment dimensions, and freshness precedence.
//! Receipt dependencies are compared positionally, including normalized context time.
//! Stale observations dominate witness freshness; fresh alternatives dominate alternatives.
//! Empty evidence is unknown. Claim lookup keeps the first matching record, without a
//! well-formedness or globally unique identifier premise. Usability follows the selected
//! claim's polarity, while support, applicability, freshness and contention stay independent.
use super::context::{FrameView, effective_context, same_context_spec};
use super::evaluation_model::{alternatives_spec, frame_matches, supported_frame_spec};
use super::footprint::EvaluationView;
use super::input::{InputView, dependency_value};
use super::lookup::claim_value;
use crate::time::assessment_spec;
use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub open spec fn receipt_current(frame: FrameView, reference: ClaimRefView, receipt: ReceiptView) -> bool {
    receipt.claim == reference && receipt.result == ReceiptResult::Pass
        && same_context_spec(effective_context(frame), receipt.context)
        && receipt.context.now <= frame.base.now && frame.base.now <= receipt.expires_at
}

pub open spec fn dependencies_match(inputs: Seq<InputView>, dependencies: Seq<DependencyView>) -> bool {
    inputs.len() == dependencies.len()
        && forall|i: int| 0 <= i < inputs.len() ==> dependency_value(inputs[i]) == dependencies[i]
}

pub proof fn dependencies_match_projection(inputs: Seq<InputView>, dependencies: Seq<DependencyView>)
    ensures (dependencies_match(inputs, dependencies)
        == (inputs.map_values(|input: InputView| dependency_value(input)) == dependencies)),
{
    let projected = inputs.map_values(|input: InputView| dependency_value(input));
    if dependencies_match(inputs, dependencies) {
        assert(projected =~= dependencies);
    } else if projected == dependencies {
        assert forall|i: int| 0 <= i < inputs.len()
            implies dependency_value(inputs[i]) == dependencies[i] by {
            assert(projected[i] == dependency_value(inputs[i]));
        }
    }
}

pub open spec fn accepted_matches(accepted: (WitnessView, EvaluationView), receipt: ReceiptView) -> bool {
    accepted.0 == receipt.witness && dependencies_match(accepted.1.dependencies, receipt.dependencies)
}

pub open spec fn reusable_with_spec(
    frame: FrameView, reference: ClaimRefView, receipt: ReceiptView,
    accepted: Seq<(WitnessView, EvaluationView)>,
) -> bool {
    receipt_current(frame, reference, receipt)
        && exists|i: int| 0 <= i < accepted.len() && accepted_matches(accepted[i], receipt)
}

pub open spec fn reusable_spec(kb: KBView, context: ContextView, reference: ClaimRefView, receipt: ReceiptView) -> bool {
    let frame = FrameView { base: context, snapshot: context.snapshot };
    reusable_with_spec(frame, reference, receipt, alternatives_spec(kb, frame, reference))
}

pub open spec fn observation_state(context: ContextView, dependency: DependencyView) -> Option<Freshness> {
    match dependency {
        DependencyView::Observation(observation) =>
            Some(assessment_spec(context.policy.ttl, observation.observed_at, context.now)),
        _ => None,
    }
}

pub open spec fn witness_freshness_spec(context: ContextView, dependencies: Seq<DependencyView>) -> Freshness {
    if exists|i: int| 0 <= i < dependencies.len()
        && observation_state(context, dependencies[i]) == Some(Freshness::Stale) {
        Freshness::Stale
    } else if (exists|i: int| 0 <= i < dependencies.len() && observation_state(context, dependencies[i]).is_some())
        && (forall|i: int| 0 <= i < dependencies.len()
            ==> observation_state(context, dependencies[i]) != Some(Freshness::Unknown)) {
        Freshness::Fresh
    } else { Freshness::Unknown }
}

pub open spec fn input_freshness_spec(context: ContextView, dependencies: Seq<InputView>) -> Freshness {
    witness_freshness_spec(context, dependencies.map_values(|input: InputView| dependency_value(input)))
}

pub open spec fn alternative_freshness_spec(context: ContextView, accepted: Seq<(WitnessView, EvaluationView)>) -> Freshness {
    if exists|i: int| 0 <= i < accepted.len()
        && input_freshness_spec(context, accepted[i].1.dependencies) == Freshness::Fresh {
        Freshness::Fresh
    } else if accepted.len() > 0 && (forall|i: int| 0 <= i < accepted.len()
        ==> input_freshness_spec(context, accepted[i].1.dependencies) == Freshness::Stale) {
        Freshness::Stale
    } else { Freshness::Unknown }
}

pub open spec fn policy_allows_spec(
    supported_positive: bool, fresh: bool, validated: bool, uncontested: bool,
    require_fresh: bool, require_validated: bool, require_uncontested: bool,
) -> bool {
    supported_positive && (!require_fresh || fresh) && (!require_validated || validated)
        && (!require_uncontested || uncontested)
}

pub open spec fn selected_negative_spec(kb: KBView, reference: ClaimRefView) -> bool {
    match claim_value(kb, reference) {
        Some(claim) => claim.statement.polarity == Polarity::Negative,
        None => false,
    }
}

pub open spec fn contrary_spec(kb: KBView, frame: FrameView, reference: ClaimRefView) -> bool {
    supported_frame_spec(kb, frame, reference,
        if selected_negative_spec(kb, reference) { Polarity::Positive } else { Polarity::Negative })
}

pub open spec fn applicable_spec(kb: KBView, frame: FrameView, reference: ClaimRefView) -> bool {
    match claim_value(kb, reference) {
        Some(claim) => frame_matches(kb, frame) && narrower_spec(frame.base.target, claim.statement.scope),
        None => false,
    }
}

pub open spec fn validated_spec(
    kb: KBView, frame: FrameView, reference: ClaimRefView, accepted: Seq<(WitnessView, EvaluationView)>,
) -> bool {
    exists|i: int| 0 <= i < kb.receipts.len() && reusable_with_spec(frame, reference, kb.receipts[i], accepted)
}

pub open spec fn dispute_matches(context: ContextView, reference: ClaimRefView, correction: CorrectionView) -> bool {
    correction.target == reference && correction.kind == CorrectionKind::Dispute && correction.recorded_at <= context.now
}

pub open spec fn disputed_spec(kb: KBView, context: ContextView, reference: ClaimRefView) -> bool {
    exists|i: int| 0 <= i < kb.corrections.len() && dispute_matches(context, reference, kb.corrections[i])
}

pub open spec fn support_spec(positive: bool, negative: bool) -> Support {
    if positive { if negative { Support::Both } else { Support::Positive } }
    else { if negative { Support::Negative } else { Support::None } }
}

pub open spec fn assess_frame_spec(kb: KBView, frame: FrameView, reference: ClaimRefView) -> AssessmentView {
    let accepted = alternatives_spec(kb, frame, reference);
    let selected = accepted.len() > 0;
    let contrary = contrary_spec(kb, frame, reference);
    let positive = if selected_negative_spec(kb, reference) { contrary } else { selected };
    let negative = if selected_negative_spec(kb, reference) { selected } else { contrary };
    let applicable = applicable_spec(kb, frame, reference);
    let freshness = alternative_freshness_spec(frame.base, accepted);
    let validated = validated_spec(kb, frame, reference, accepted);
    let contested = contrary || disputed_spec(kb, frame.base, reference);
    AssessmentView {
        support: support_spec(positive, negative), applicable, freshness, validated, contested,
        available: positive || negative,
        usable: applicable && policy_allows_spec(selected, freshness == Freshness::Fresh, validated, !contested,
            frame.base.policy.require_fresh, frame.base.policy.require_validation, frame.base.policy.require_uncontested),
    }
}

pub open spec fn assess_spec(kb: KBView, context: ContextView, reference: ClaimRefView) -> AssessmentView {
    assess_frame_spec(kb, FrameView { base: context, snapshot: context.snapshot }, reference)
}

pub open spec fn usable_for_spec(kb: KBView, context: ContextView, reference: ClaimRefView) -> bool {
    assess_spec(kb, context, reference).usable
}

pub open spec fn usable_for_snapshot_spec(kb: KBView, context: ContextView, snapshot: Seq<char>, reference: ClaimRefView) -> bool {
    assess_frame_spec(kb, FrameView { base: context, snapshot }, reference).usable
}

} // verus!
