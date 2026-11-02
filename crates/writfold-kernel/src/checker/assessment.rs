use super::context::Frame;
use super::evaluation::alternatives;
use super::footprint::{Evaluation, Input};
use super::{same_context, supported_frame};
use crate::time::assess_fresh;
use writfold_domain::*;

fn reusable_with(
    context: Frame<'_>,
    reference: &ClaimRef,
    receipt: &Receipt,
    accepted: &[(&Witness, Evaluation<'_>)],
) -> bool {
    receipt.claim() == reference
        && *receipt.result() == ReceiptResult::Pass
        && context.same_context(receipt.context())
        && receipt.context().now() <= context.now()
        && context.now() <= receipt.expires_at()
        && accepted.iter().any(|(witness, checked)| {
            *witness == receipt.witness()
                && checked.dependencies.len() == receipt.dependencies().len()
                && checked
                    .dependencies
                    .iter()
                    .zip(receipt.dependencies())
                    .all(|(a, b)| a.matches(b))
        })
}

pub fn reusable(kb: &KB, context: &Context, reference: &ClaimRef, receipt: &Receipt) -> bool {
    if receipt.claim() != reference
        || *receipt.result() != ReceiptResult::Pass
        || !same_context(receipt.context(), context)
        || receipt.context().now() > context.now()
        || context.now() > receipt.expires_at()
    {
        return false;
    }
    reusable_with(
        Frame::new(context),
        reference,
        receipt,
        &alternatives(kb, Frame::new(context), reference),
    )
}

fn observations_freshness<'a>(
    context: &Context,
    observations: impl Iterator<Item = &'a Observation>,
) -> Freshness {
    let mut state = Freshness::Fresh;
    let mut any = false;
    for observation in observations {
        any = true;
        match assess_fresh(
            context.policy().ttl(),
            observation.observed_at(),
            context.now(),
        ) {
            Freshness::Stale => return Freshness::Stale,
            Freshness::Unknown => state = Freshness::Unknown,
            Freshness::Fresh => {}
        }
    }
    if any { state } else { Freshness::Unknown }
}

pub fn witness_freshness(context: &Context, dependencies: &[Dependency]) -> Freshness {
    observations_freshness(
        context,
        dependencies.iter().filter_map(|dependency| {
            if let Dependency::Observation(value) = dependency {
                Some(value)
            } else {
                None
            }
        }),
    )
}

pub fn policy_allows(
    supported_positive: bool,
    fresh: bool,
    validated: bool,
    uncontested: bool,
    require_fresh: bool,
    require_validated: bool,
    require_uncontested: bool,
) -> bool {
    supported_positive
        && (!require_fresh || fresh)
        && (!require_validated || validated)
        && (!require_uncontested || uncontested)
}

fn alternative_freshness(context: Frame<'_>, accepted: &[(&Witness, Evaluation<'_>)]) -> Freshness {
    let mut all_stale = !accepted.is_empty();
    for (_, result) in accepted {
        let state = observations_freshness(
            context.base,
            result.dependencies.iter().filter_map(|dependency| {
                if let Input::Observation(value) = dependency {
                    Some(*value)
                } else {
                    None
                }
            }),
        );
        if state == Freshness::Fresh {
            return Freshness::Fresh;
        }
        all_stale &= state == Freshness::Stale;
    }
    if all_stale {
        Freshness::Stale
    } else {
        Freshness::Unknown
    }
}

pub fn assess(kb: &KB, context: &Context, reference: &ClaimRef) -> Assessment {
    assess_frame(kb, Frame::new(context), reference)
}

fn assess_frame(kb: &KB, context: Frame<'_>, reference: &ClaimRef) -> Assessment {
    let claim = find_claim(kb, reference);
    let accepted = alternatives(kb, context, reference);
    let selected = !accepted.is_empty();
    let selected_negative = claim.is_some_and(|c| *c.statement().polarity() == Polarity::Negative);
    let contrary = supported_frame(
        kb,
        context,
        reference,
        if selected_negative {
            Polarity::Positive
        } else {
            Polarity::Negative
        },
    );
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
    let applicable = claim
        .is_some_and(|c| context.matches(kb) && narrower(context.target(), c.statement().scope()));
    let freshness = alternative_freshness(context, &accepted);
    let validated = kb
        .receipts()
        .iter()
        .any(|r| reusable_with(context, reference, r, &accepted));
    let contested = contrary
        || kb.corrections().iter().any(|c| {
            c.target() == reference
                && *c.kind() == CorrectionKind::Dispute
                && c.recorded_at() <= context.now()
        });
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
    Assessment::new(
        support,
        applicable,
        freshness,
        validated,
        contested,
        positive || negative,
        usable,
    )
}

pub fn usable_for(kb: &KB, context: &Context, reference: &ClaimRef) -> bool {
    *assess(kb, context, reference).usable()
}

pub fn usable_for_snapshot(
    kb: &KB,
    context: &Context,
    snapshot: &SnapshotId,
    reference: &ClaimRef,
) -> bool {
    *assess_frame(
        kb,
        Frame {
            base: context,
            snapshot,
        },
        reference,
    )
    .usable()
}

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
