//! Public mathematical contracts for consumers of the finite-witness checker.
pub use super::assessment_model::{assess_spec, reusable_spec, usable_for_spec, usable_for_snapshot_spec};
use super::context::FrameView;
use super::evaluation_model::{alternatives_spec, correction_input, evaluate_spec};
use super::footprint::{EvaluationView, checked_value, push_unique_spec, statement_value};
use super::input::{InputView, dependency_value};
use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub open spec fn frame(context: ContextView) -> FrameView {
    FrameView { base: context, snapshot: context.snapshot }
}

pub open spec fn check_spec(kb: KBView, context: ContextView, witness: WitnessView) -> Option<CheckedView> {
    match evaluate_spec(kb, frame(context), witness) {
        Some(value) => Some(checked_value(EvaluationView {
            dependencies: push_unique_spec(value.dependencies, InputView::Context(frame(context))), ..value
        })),
        None => None,
    }
}

pub open spec fn check_matches_snapshot_spec(
    kb: KBView, context: ContextView, snapshot: Seq<char>, witness: WitnessView, statement: StatementView,
) -> bool {
    match evaluate_spec(kb, FrameView { base: context, snapshot }, witness) {
        Some(value) => statement_value(value) == statement,
        None => false,
    }
}

pub open spec fn check_matches_spec(
    kb: KBView, context: ContextView, witness: WitnessView, statement: StatementView,
) -> bool {
    check_matches_snapshot_spec(kb, context, context.snapshot, witness, statement)
}

pub open spec fn correction_dependency_spec(kb: KBView, context: ContextView, reference: ClaimRefView) -> DependencyView {
    dependency_value(correction_input(kb, frame(context), reference))
}

pub open spec fn accepted_spec(kb: KBView, context: ContextView, reference: ClaimRefView) -> Seq<(WitnessView, CheckedView)> {
    alternatives_spec(kb, frame(context), reference).map_values(
        |item: (WitnessView, EvaluationView)| (item.0, checked_value(item.1)))
}

} // verus!
