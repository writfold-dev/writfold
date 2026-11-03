use super::context::Frame;
use super::footprint::{Evaluation, corrections, extend_unique, formula_ref, into_formula, push_unique};
use super::input::Input;
use super::inventory::matches_expected;
use super::lookup::{find_attestation, find_enumeration};
#[cfg(verus_keep_ghost)]
use super::evaluation_model::*;
#[cfg(verus_keep_ghost)]
use super::lookup::{claim_lookup_value, claim_value, observation_lookup_value};
#[cfg(verus_keep_ghost)]
use super::footprint::extend_unique_spec;
use super::{inventory_content, retracted, source_selected};
use crate::wellformed::unique;
use std::borrow::Cow;
use writfold_domain::*;
use vstd::prelude::*;

verus! {

pub(super) fn evaluate<'a>(
    kb: &'a KB,
    context: Frame<'a>,
    witness: &'a Witness,
) -> (result: Option<Evaluation<'a>>)
    ensures result.deep_view() == evaluate_spec(kb@, context@, witness@),
{
    if !context.matches(kb) {
        return None;
    }
    evaluate_valid(kb, context, witness)
}

fn evaluate_valid<'a>(kb: &'a KB, context: Frame<'a>, witness: &'a Witness) -> (result: Option<Evaluation<'a>>)
    ensures result.deep_view() == evaluate_valid_spec(kb@, context@, witness@),
    decreases witness, 0nat,
{
    match witness {
        Witness::Attest { attestation } => attest(kb, context, attestation),
        Witness::Truth { scope } => Some(Evaluation {
            formula: Cow::Owned(Formula::Truth),
            scope,
            polarity: Polarity::Positive,
            dependencies: Vec::new(),
        }),
        Witness::And { left, right } => {
            let left = evaluate_valid(kb, context, left)?;
            let right = evaluate_valid(kb, context, right)?;
            if left.scope != right.scope
                || left.polarity != Polarity::Positive
                || right.polarity != Polarity::Positive
            {
                return None;
            }
            let formula = Formula::Conjunction {
                left: Box::new(into_formula(left.formula)),
                right: Box::new(into_formula(right.formula)),
            };
            let mut dependencies = left.dependencies;
            extend_unique(&mut dependencies, right.dependencies);
            Some(Evaluation {
                formula: Cow::Owned(formula),
                scope: left.scope,
                polarity: Polarity::Positive,
                dependencies,
            })
        }
        Witness::Specialize { scope, premise } => {
            let premise = evaluate_valid(kb, context, premise)?;
            if !narrower(scope, premise.scope) {
                return None;
            }
            Some(Evaluation { scope, ..premise })
        }
        Witness::Absent {
            enumeration,
            premise,
        } => enumerate(kb, context, enumeration, premise, None),
        Witness::All {
            enumeration,
            predicate,
            premise,
        } => enumerate(kb, context, enumeration, premise, Some(predicate)),
    }
}

fn attest<'a>(kb: &'a KB, context: Frame<'a>, id: &AttestationId) -> (result: Option<Evaluation<'a>>)
    ensures result.deep_view() == attest_spec(kb@, context@, id@),
{
    proof { broadcast use claim_lookup_value, observation_lookup_value; }
    let attestation = find_attestation(kb, id)?;
    if retracted(kb, context.base, attestation.claim()) {
        return None;
    }
    let claim = find_claim(kb, attestation.claim())?;
    let observation = find_observation(kb, attestation.observation())?;
    if claim.statement() != attestation.statement()
        || !matches!(
            attestation.statement().formula(),
            Formula::Atomic(_) | Formula::Inventory { .. }
        )
        || attestation.authority().as_str().is_empty()
        || *observation.outcome() != ObsOutcome::Ok
        || observation.observed_at() > context.now()
        || !source_selected(context.base, observation)
    {
        return None;
    }
    let mut dependencies = vec![
        Input::Observation(observation),
        Input::Claim(claim),
        Input::Attestation(attestation),
        corrections(kb, context, attestation.claim()),
    ];
    if let Formula::Inventory { enumeration } = attestation.statement().formula() {
        let enumeration = find_enumeration(kb, enumeration)?;
        if *attestation.statement().polarity() != Polarity::Positive
            || attestation.observation() != enumeration.observation()
            || attestation.statement().scope() != enumeration.scope()
        {
            return None;
        }
        dependencies.push(Input::Enumeration(enumeration));
    }
    Some(Evaluation {
        formula: Cow::Borrowed(attestation.statement().formula()),
        scope: attestation.statement().scope(),
        polarity: *attestation.statement().polarity(),
        dependencies,
    })
}

fn enumerate<'a>(
    kb: &'a KB,
    context: Frame<'a>,
    id: &EnumerationId,
    witness: &'a Witness,
    predicate: Option<&str>,
) -> (result: Option<Evaluation<'a>>)
    ensures result.deep_view() == enumerate_spec(kb@, context@, id@, witness@, predicate.deep_view()),
    decreases witness, 1nat,
{
    proof {
        broadcast use observation_lookup_value, writfold_domain::equality::vector_equality;
    }
    let enumeration = find_enumeration(kb, id)?;
    let observation = find_observation(kb, enumeration.observation())?;
    if *observation.outcome() != ObsOutcome::Ok
        || observation.observed_at() > context.now()
        || !source_selected(context.base, observation)
        || observation.content() != &inventory_content(enumeration.universe())
        || observation.source() != enumeration.source()
        || observation.revision() != enumeration.revision()
        || enumeration.members() != enumeration.universe()
        || !unique(enumeration.members())
        || enumeration.method().is_empty()
        || enumeration.query().is_empty()
    {
        return None;
    }
    let mut premise = evaluate_valid(kb, context, witness)?;
    let mode_valid = match predicate {
        None => enumeration.members().is_empty(),
        Some(value) => !value.is_empty(),
    };
    if !mode_valid || !matches_expected(formula_ref(&premise.formula), enumeration, predicate)
        || premise.scope != enumeration.scope()
        || premise.polarity != Polarity::Positive
    {
        return None;
    }
    let conclusion = match predicate {
        None => Formula::Absent { enumeration: enumeration.id().clone() },
        Some(value) => Formula::All {
            enumeration: enumeration.id().clone(), predicate: value.to_owned(),
        },
    };
    push_unique(&mut premise.dependencies, Input::Observation(observation));
    push_unique(&mut premise.dependencies, Input::Enumeration(enumeration));
    Some(Evaluation {
        formula: Cow::Owned(conclusion),
        scope: enumeration.scope(),
        polarity: Polarity::Positive,
        dependencies: premise.dependencies,
    })
}

pub(super) fn accepted_derivation<'a>(
    kb: &'a KB,
    context: Frame<'a>,
    claim: &'a Claim,
    derivation: &'a Derivation,
) -> (accepted: Option<Evaluation<'a>>)
    ensures accepted.deep_view() == accepted_derivation_spec(kb@, context@, claim@, derivation@),
{
    proof { reveal_with_fuel(extend_unique_spec, 4); }
    if !claim.matches_ref(derivation.claim()) {
        return None;
    }
    let mut result = evaluate(kb, context, derivation.witness())?;
    if !result.matches_statement(claim.statement()) {
        return None;
    }
    push_unique(&mut result.dependencies, Input::Claim(claim));
    push_unique(&mut result.dependencies, corrections(kb, context, derivation.claim()));
    push_unique(&mut result.dependencies, Input::Context(context));
    Some(result)
}

pub(super) fn alternatives<'a>(
    kb: &'a KB,
    context: Frame<'a>,
    reference: &ClaimRef,
) -> (accepted: Vec<(&'a Witness, Evaluation<'a>)>)
    ensures accepted.deep_view() == alternatives_spec(kb@, context@, reference@),
{
    proof { broadcast use claim_lookup_value; }
    let Some(claim) = find_claim(kb, reference) else {
        return Vec::new();
    };
    if retracted(kb, context.base, reference) {
        return Vec::new();
    }
    let rows = kb.derivations();
    let mut accepted = Vec::new();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            rows.deep_view() == kb@.derivations, rows.len() == kb@.derivations.len(),
            index <= rows.len(), claim_value(kb@, reference@) == Some(claim@),
            !super::retracted_spec(kb@, context@.base, reference@),
            accepted.deep_view() == accepted_rows(kb@, context@, claim@, rows.deep_view().take(index as int)),
        decreases rows.len() - index,
    {
        let row = &rows[index];
        proof { assert(row@ == kb@.derivations[index as int]); }
        if let Some(value) = accepted_derivation(kb, context, claim, row) {
            accepted.push((row.witness(), value));
        }
        index += 1;
        proof {
            let prefix = kb@.derivations.take(index as int);
            assert(prefix.drop_last() == kb@.derivations.take(index as int - 1));
            assert(prefix.last() == row@);
        }
    }
    proof { assert(kb@.derivations.take(rows.len() as int) == kb@.derivations); }
    accepted
}

pub(super) fn has_accepted_claim(kb: &KB, context: Frame<'_>, claim: &Claim) -> (accepted: bool)
    ensures accepted == has_accepted_claim_spec(kb@, context@, claim@),
{
    let corrections = kb.corrections();
    let mut index = 0;
    proof { assert(corrections.deep_view().len() == corrections.len()); }
    while index < corrections.len()
        invariant
            corrections.deep_view() == kb@.corrections, corrections.len() == kb@.corrections.len(),
            index <= corrections.len(),
            forall|i: int| 0 <= i < index ==> !(kb@.corrections[i].kind == CorrectionKind::Retract
                && claim_matches_spec(claim@, kb@.corrections[i].target)
                && kb@.corrections[i].recorded_at <= context@.base.now),
        decreases corrections.len() - index,
    {
        let correction = &corrections[index];
        proof { assert(correction@ == kb@.corrections[index as int]); }
        if *correction.kind() == CorrectionKind::Retract
            && claim.matches_ref(correction.target()) && correction.recorded_at() <= context.now() {
            return false;
        }
        index += 1;
    }
    let rows = kb.derivations();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            !super::retracted_spec(kb@, context@.base, ClaimRefView { id: claim@.id, rev: claim@.revision }),
            rows.deep_view() == kb@.derivations, rows.len() == kb@.derivations.len(), index <= rows.len(),
            forall|i: int| 0 <= i < index ==> accepted_derivation_spec(kb@, context@, claim@, kb@.derivations[i]).is_none(),
        decreases rows.len() - index,
    {
        let row = &rows[index];
        proof { assert(row@ == kb@.derivations[index as int]); }
        if accepted_derivation(kb, context, claim, row).is_some() { return true; }
        index += 1;
    }
    false
}

} // verus!
