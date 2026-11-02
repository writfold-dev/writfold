use super::context::Frame;
use super::footprint::{Evaluation, Input, corrections, extend_unique};
use super::{inventory_content, retracted, source_selected};
use crate::wellformed::unique;
use std::borrow::Cow;
use writfold_domain::*;

pub(super) fn evaluate<'a>(
    kb: &'a KB,
    context: Frame<'a>,
    witness: &'a Witness,
) -> Option<Evaluation<'a>> {
    if !context.matches(kb) {
        return None;
    }
    match witness {
        Witness::Attest { attestation } => attest(kb, context, attestation),
        Witness::Truth { scope } => Some(Evaluation {
            formula: Cow::Owned(Formula::Truth),
            scope,
            polarity: Polarity::Positive,
            dependencies: Vec::new(),
        }),
        Witness::And { left, right } => {
            let left = evaluate(kb, context, left)?;
            let right = evaluate(kb, context, right)?;
            if left.scope != right.scope
                || left.polarity != Polarity::Positive
                || right.polarity != Polarity::Positive
            {
                return None;
            }
            let formula = Formula::Conjunction {
                left: Box::new(left.formula.into_owned()),
                right: Box::new(right.formula.into_owned()),
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
            let premise = evaluate(kb, context, premise)?;
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

fn attest<'a>(kb: &'a KB, context: Frame<'a>, id: &AttestationId) -> Option<Evaluation<'a>> {
    let attestation = kb.attestations().iter().find(|a| a.id() == id)?;
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
        let enumeration = kb.enumerations().iter().find(|e| e.id() == enumeration)?;
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
) -> Option<Evaluation<'a>> {
    let enumeration = kb.enumerations().iter().find(|e| e.id() == id)?;
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
    let mut premise = evaluate(kb, context, witness)?;
    let inventory = Formula::Inventory {
        enumeration: enumeration.id().clone(),
    };
    let (expected, conclusion) = match predicate {
        None => {
            if !enumeration.members().is_empty() {
                return None;
            }
            (
                inventory,
                Formula::Absent {
                    enumeration: enumeration.id().clone(),
                },
            )
        }
        Some(predicate) => {
            if predicate.is_empty() {
                return None;
            }
            (
                Formula::Conjunction {
                    left: Box::new(inventory),
                    right: Box::new(member_formula(enumeration.members(), predicate)),
                },
                Formula::All {
                    enumeration: enumeration.id().clone(),
                    predicate: predicate.to_owned(),
                },
            )
        }
    };
    if premise.formula.as_ref() != &expected
        || premise.scope != enumeration.scope()
        || premise.polarity != Polarity::Positive
    {
        return None;
    }
    extend_unique(
        &mut premise.dependencies,
        [
            Input::Observation(observation),
            Input::Enumeration(enumeration),
        ],
    );
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
) -> Option<Evaluation<'a>> {
    if !claim.matches_ref(derivation.claim()) {
        return None;
    }
    let mut result = evaluate(kb, context, derivation.witness())?;
    if !result.matches_statement(claim.statement()) {
        return None;
    }
    extend_unique(
        &mut result.dependencies,
        [
            Input::Claim(claim),
            corrections(kb, context, derivation.claim()),
            Input::Context(context),
        ],
    );
    Some(result)
}

pub(super) fn alternatives<'a>(
    kb: &'a KB,
    context: Frame<'a>,
    reference: &ClaimRef,
) -> Vec<(&'a Witness, Evaluation<'a>)> {
    let Some(claim) = find_claim(kb, reference) else {
        return Vec::new();
    };
    if retracted(kb, context.base, reference) {
        return Vec::new();
    }
    kb.derivations()
        .iter()
        .filter_map(|d| {
            accepted_derivation(kb, context, claim, d).map(|result| (d.witness(), result))
        })
        .collect()
}

pub(super) fn has_accepted_claim(kb: &KB, context: Frame<'_>, claim: &Claim) -> bool {
    if kb.corrections().iter().any(|c| {
        *c.kind() == CorrectionKind::Retract
            && claim.matches_ref(c.target())
            && c.recorded_at() <= context.now()
    }) {
        return false;
    }
    kb.derivations()
        .iter()
        .any(|d| accepted_derivation(kb, context, claim, d).is_some())
}
