//! The preserved finite-witness calculus; acquisition alone proves no claim.
mod context;
pub use assessment::usable_for_snapshot;
use context::Frame;
mod assessment;
mod evaluation;
mod footprint;

pub use assessment::{
    assess, assessment_gaps, policy_allows, reusable, usable_for, witness_freshness,
};
use evaluation::{accepted_derivation, evaluate, has_accepted_claim};
use footprint::{Input, corrections, extend_unique};
use writfold_domain::*;

pub fn context_matches(kb: &KB, context: &Context) -> bool {
    kb.snapshot() == context.snapshot() && crate::wellformed::context_wf(context)
}

pub fn source_selected(context: &Context, observation: &Observation) -> bool {
    context.sources().iter().any(|source| {
        source.source() == observation.source()
            && source.revision() == observation.revision()
            && source.content() == observation.content()
    })
}

pub fn inventory_content(members: &[String]) -> ContentId {
    ContentId::inventory(members)
}

pub fn retracted(kb: &KB, context: &Context, reference: &ClaimRef) -> bool {
    kb.corrections().iter().any(|c| {
        *c.kind() == CorrectionKind::Retract
            && c.target() == reference
            && c.recorded_at() <= context.now()
    })
}

pub fn dependencies_union(left: &[Dependency], right: &[Dependency]) -> Vec<Dependency> {
    let mut result = Vec::new();
    for dependency in left.iter().chain(right) {
        if !result.contains(dependency) {
            result.push(dependency.clone());
        }
    }
    result
}

pub fn correction_dependency(kb: &KB, context: &Context, reference: &ClaimRef) -> Dependency {
    corrections(kb, Frame::new(context), reference).into_owned()
}

pub fn check(kb: &KB, context: &Context, witness: &Witness) -> Option<Checked> {
    let mut result = evaluate(kb, Frame::new(context), witness)?;
    extend_unique(
        &mut result.dependencies,
        [Input::Context(Frame::new(context))],
    );
    Some(result.into_owned())
}

pub fn accepted<'a>(
    kb: &'a KB,
    context: &Context,
    reference: &ClaimRef,
) -> Vec<(&'a Witness, Checked)> {
    let Some(claim) = find_claim(kb, reference) else {
        return Vec::new();
    };
    if retracted(kb, context, reference) {
        return Vec::new();
    }
    kb.derivations()
        .iter()
        .filter_map(|d| {
            accepted_derivation(kb, Frame::new(context), claim, d)
                .map(|result| (d.witness(), result.into_owned()))
        })
        .collect()
}

pub fn supported(kb: &KB, context: &Context, reference: &ClaimRef, polarity: Polarity) -> bool {
    supported_frame(kb, Frame::new(context), reference, polarity)
}

fn supported_frame(kb: &KB, context: Frame<'_>, reference: &ClaimRef, polarity: Polarity) -> bool {
    let Some(claim) = find_claim(kb, reference) else {
        return false;
    };
    if polarity == *claim.statement().polarity() {
        return has_accepted_claim(kb, context, claim);
    }
    if !matches!(claim.statement().formula(), Formula::Atomic(_)) {
        return false;
    }
    kb.claims().iter().any(|c| {
        c.statement().formula() == claim.statement().formula()
            && c.statement().scope() == claim.statement().scope()
            && *c.statement().polarity() == polarity
            && has_accepted_claim(kb, context, c)
    })
}

pub fn same_context(left: &Context, right: &Context) -> bool {
    left.snapshot() == right.snapshot()
        && left.semantic_version() == right.semantic_version()
        && left.ruleset() == right.ruleset()
        && left.checker() == right.checker()
        && left.policy() == right.policy()
        && left.target() == right.target()
        && left.intended_use() == right.intended_use()
        && left.sources() == right.sources()
}

pub fn check_matches(kb: &KB, context: &Context, witness: &Witness, statement: &Statement) -> bool {
    evaluate(kb, Frame::new(context), witness)
        .is_some_and(|result| result.matches_statement(statement))
}

pub fn check_matches_snapshot(
    kb: &KB,
    context: &Context,
    snapshot: &SnapshotId,
    witness: &Witness,
    statement: &Statement,
) -> bool {
    evaluate(
        kb,
        Frame {
            base: context,
            snapshot,
        },
        witness,
    )
    .is_some_and(|result| result.matches_statement(statement))
}
