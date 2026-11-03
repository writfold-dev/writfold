//! Declarative finite witness rules and exact ordered borrowed footprints.
use super::context::FrameView;
use super::footprint::statement_value;
use super::footprint::{EvaluationView, correction_events, extend_unique_spec};
use super::input::InputView;
use super::lookup::{attestation_value, claim_value, enumeration_value, observation_value};
use super::{retracted_spec, source_selected_spec};
use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub open spec fn frame_matches(kb: KBView, frame: FrameView) -> bool {
    kb.snapshot == frame.snapshot && crate::wellformed::spec::context_wf_spec(frame.base)
}

pub open spec fn correction_input(kb: KBView, frame: FrameView, target: ClaimRefView) -> InputView {
    InputView::Corrections { target, events: correction_events(kb.corrections, target, frame.base.now) }
}

pub open spec fn attested_leaf(
    kb: KBView, frame: FrameView, attestation: AttestationView,
    claim: ClaimView, observation: ObservationView,
) -> Option<EvaluationView> {
    if claim.statement != attestation.statement
        || !(attestation.statement.formula is Atomic || attestation.statement.formula is Inventory)
        || attestation.authority.len() == 0 || observation.outcome != ObsOutcome::Ok
        || observation.observed_at > frame.base.now || !source_selected_spec(frame.base, observation) {
        None
    } else {
        let dependencies = seq![InputView::Observation(observation), InputView::Claim(claim),
            InputView::Attestation(attestation), correction_input(kb, frame, attestation.claim)];
        match attestation.statement.formula {
            FormulaView::Inventory { enumeration } => match enumeration_value(kb, enumeration) {
                Some(row) => if attestation.statement.polarity == Polarity::Positive
                    && attestation.observation == row.observation && attestation.statement.scope == row.scope {
                    Some(EvaluationView { formula: attestation.statement.formula,
                        scope: attestation.statement.scope, polarity: attestation.statement.polarity,
                        dependencies: dependencies.push(InputView::Enumeration(row)) })
                } else { None },
                None => None,
            },
            _ => Some(EvaluationView { formula: attestation.statement.formula,
                scope: attestation.statement.scope, polarity: attestation.statement.polarity, dependencies }),
        }
    }
}

pub open spec fn attest_spec(kb: KBView, frame: FrameView, ident: Seq<char>) -> Option<EvaluationView> {
    match attestation_value(kb, ident) {
        Some(attestation) => if retracted_spec(kb, frame.base, attestation.claim) { None }
            else { match (claim_value(kb, attestation.claim), observation_value(kb, attestation.observation)) {
                (Some(claim), Some(observation)) => attested_leaf(kb, frame, attestation, claim, observation),
                _ => None,
            } },
        None => None,
    }
}

pub open spec fn inventory_ready(
    frame: FrameView, enumeration: EnumerationView, observation: ObservationView,
) -> bool {
    observation.outcome == ObsOutcome::Ok && observation.observed_at <= frame.base.now
        && source_selected_spec(frame.base, observation)
        && observation.content == ContentId::inventory_spec(enumeration.universe)
        && observation.source == enumeration.source && observation.revision == enumeration.revision
        && enumeration.members == enumeration.universe && enumeration.members.no_duplicates()
        && enumeration.method.len() > 0 && enumeration.query.len() > 0
}

pub open spec fn inventory_expected(enumeration: EnumerationView, predicate: Option<Seq<char>>) -> FormulaView {
    let inventory = FormulaView::Inventory { enumeration: enumeration.id };
    match predicate {
        None => inventory,
        Some(text) => FormulaView::Conjunction { left: Box::new(inventory),
            right: Box::new(member_formula_spec(enumeration.members, text)) },
    }
}

pub open spec fn inventory_conclusion(enumeration: EnumerationView, predicate: Option<Seq<char>>) -> FormulaView {
    match predicate {
        None => FormulaView::Absent { enumeration: enumeration.id },
        Some(text) => FormulaView::All { enumeration: enumeration.id, predicate: text },
    }
}

pub open spec fn enumeration_result(
    enumeration: EnumerationView, observation: ObservationView,
    premise: EvaluationView, predicate: Option<Seq<char>>,
) -> Option<EvaluationView> {
    let mode_valid = match predicate { None => enumeration.members.len() == 0, Some(text) => text.len() > 0 };
    if mode_valid && premise.formula == inventory_expected(enumeration, predicate)
        && premise.scope == enumeration.scope && premise.polarity == Polarity::Positive {
        Some(EvaluationView { formula: inventory_conclusion(enumeration, predicate),
            scope: enumeration.scope, polarity: Polarity::Positive,
            dependencies: extend_unique_spec(premise.dependencies,
                seq![InputView::Observation(observation), InputView::Enumeration(enumeration)]) })
    } else { None }
}

pub open spec fn enumerate_spec(
    kb: KBView, frame: FrameView, ident: Seq<char>, witness: WitnessView,
    predicate: Option<Seq<char>>,
) -> Option<EvaluationView>
    decreases witness, 1nat,
{
    match enumeration_value(kb, ident) {
        Some(enumeration) => match observation_value(kb, enumeration.observation) {
            Some(observation) => if inventory_ready(frame, enumeration, observation) {
                match evaluate_valid_spec(kb, frame, witness) {
                    Some(premise) => enumeration_result(enumeration, observation, premise, predicate),
                    None => None,
                }
            } else { None },
            None => None,
        },
        None => None,
    }
}

pub open spec fn evaluate_valid_spec(kb: KBView, frame: FrameView, witness: WitnessView) -> Option<EvaluationView>
    decreases witness, 0nat,
{
    match witness {
        WitnessView::Attest { attestation } => attest_spec(kb, frame, attestation),
        WitnessView::Truth { scope } => Some(EvaluationView { formula: FormulaView::Truth,
            scope, polarity: Polarity::Positive, dependencies: Seq::empty() }),
        WitnessView::And { left, right } => match (
            evaluate_valid_spec(kb, frame, *left), evaluate_valid_spec(kb, frame, *right),
        ) {
            (Some(a), Some(b)) => if a.scope == b.scope && a.polarity == Polarity::Positive
                && b.polarity == Polarity::Positive {
                Some(EvaluationView { formula: FormulaView::Conjunction {
                    left: Box::new(a.formula), right: Box::new(b.formula),
                }, scope: a.scope, polarity: Polarity::Positive,
                    dependencies: extend_unique_spec(a.dependencies, b.dependencies) })
            } else { None },
            _ => None,
        },
        WitnessView::Specialize { scope, premise } => match evaluate_valid_spec(kb, frame, *premise) {
            Some(value) => if narrower_spec(scope, value.scope) { Some(EvaluationView { scope, ..value }) }
                else { None },
            None => None,
        },
        WitnessView::Absent { enumeration, premise } => enumerate_spec(kb, frame, enumeration, *premise, None),
        WitnessView::All { enumeration, predicate, premise } =>
            enumerate_spec(kb, frame, enumeration, *premise, Some(predicate)),
    }
}

pub open spec fn evaluate_spec(kb: KBView, frame: FrameView, witness: WitnessView) -> Option<EvaluationView> {
    if frame_matches(kb, frame) { evaluate_valid_spec(kb, frame, witness) } else { None }
}

pub open spec fn accepted_derivation_spec(
    kb: KBView, frame: FrameView, claim: ClaimView, derivation: DerivationView,
) -> Option<EvaluationView> {
    if !claim_matches_spec(claim, derivation.claim) { None }
    else {
        match evaluate_spec(kb, frame, derivation.witness) {
            Some(value) => if statement_value(value) == claim.statement {
                Some(EvaluationView {
                    dependencies: extend_unique_spec(value.dependencies, seq![
                        InputView::Claim(claim), correction_input(kb, frame, derivation.claim),
                        InputView::Context(frame),
                    ]), ..value
                })
            } else { None },
            None => None,
        }
    }
}

pub open spec fn accepted_rows(
    kb: KBView, frame: FrameView, claim: ClaimView, rows: Seq<DerivationView>,
) -> Seq<(WitnessView, EvaluationView)>
    decreases rows.len(),
{
    if rows.len() == 0 { Seq::empty() }
    else {
        let prior = accepted_rows(kb, frame, claim, rows.drop_last());
        let row = rows.last();
        match accepted_derivation_spec(kb, frame, claim, row) {
            Some(value) => prior.push((row.witness, value)),
            None => prior,
        }
    }
}

pub proof fn accepted_rows_step(
    kb: KBView, frame: FrameView, claim: ClaimView, rows: Seq<DerivationView>, index: int,
)
    requires 0 <= index < rows.len(),
    ensures accepted_rows(kb, frame, claim, rows.take(index + 1)) ==
        (match accepted_derivation_spec(kb, frame, claim, rows[index]) {
            Some(value) => accepted_rows(kb, frame, claim, rows.take(index)).push((rows[index].witness, value)),
            None => accepted_rows(kb, frame, claim, rows.take(index)),
        }),
{
    let prefix = rows.take(index + 1);
    assert(prefix.drop_last() =~= rows.take(index));
    assert(prefix.last() == rows[index]);
    reveal_with_fuel(accepted_rows, 2);
}

pub open spec fn alternatives_spec(kb: KBView, frame: FrameView, reference: ClaimRefView)
    -> Seq<(WitnessView, EvaluationView)> {
    if retracted_spec(kb, frame.base, reference) { Seq::empty() }
    else {
        match claim_value(kb, reference) {
            Some(claim) => accepted_rows(kb, frame, claim, kb.derivations),
            None => Seq::empty(),
        }
    }
}

pub open spec fn has_accepted_claim_spec(kb: KBView, frame: FrameView, claim: ClaimView) -> bool {
    !retracted_spec(kb, frame.base, ClaimRefView { id: claim.id, rev: claim.revision })
        && exists|i: int| 0 <= i < kb.derivations.len()
            && accepted_derivation_spec(kb, frame, claim, kb.derivations[i]).is_some()
}

pub open spec fn has_accepted_reference_spec(kb: KBView, frame: FrameView, reference: ClaimRefView) -> bool {
    match claim_value(kb, reference) {
        Some(claim) => has_accepted_claim_spec(kb, frame, claim),
        None => false,
    }
}

pub open spec fn supported_frame_spec(kb: KBView, frame: FrameView, reference: ClaimRefView, polarity: Polarity) -> bool {
    match claim_value(kb, reference) {
        None => false,
        Some(claim) => if polarity == claim.statement.polarity {
            has_accepted_claim_spec(kb, frame, claim)
        } else {
            claim.statement.formula is Atomic
                && exists|i: int| 0 <= i < kb.claims.len()
                    && kb.claims[i].statement.formula == claim.statement.formula
                    && kb.claims[i].statement.scope == claim.statement.scope
                    && kb.claims[i].statement.polarity == polarity
                    && has_accepted_reference_spec(kb, frame,
                        ClaimRefView { id: kb.claims[i].id, rev: kb.claims[i].revision })
        },
    }
}

} // verus!
