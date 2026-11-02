use std::error::Error;
use writfold_domain::*;
use writfold_kernel::{checker::*, time::*, wellformed::*};

type TestResult = Result<(), Box<dyn Error>>;
#[path = "core/fixture.rs"]
mod fixture;
use fixture::fixture;

fn at(context: &Context, now: Integer) -> Context {
    Context::new(
        context.snapshot().clone(),
        context.semantic_version().clone(),
        context.ruleset().clone(),
        context.checker().clone(),
        context.policy().clone(),
        now,
        context.target().clone(),
        context.intended_use().clone(),
        context.sources().clone(),
    )
}

fn corrected(kb: &KB, corrections: Vec<Correction>, receipts: Vec<Receipt>) -> KB {
    KB::new(
        kb.snapshot().clone(),
        kb.observations().clone(),
        kb.claims().clone(),
        kb.attestations().clone(),
        kb.enumerations().clone(),
        kb.derivations().clone(),
        corrections,
        receipts,
    )
}

#[test]
fn freshness_preserves_signed_unbounded_inclusive_arithmetic() -> TestResult {
    let observed: Integer = "100000000000000000000000000000000000000000000000000".parse()?;
    let duration: Integer = "100000000000000000000000000000000000000000000000001".parse()?;
    let expiry = &observed + &duration;
    let ttl = Ttl::ExpireAfter(duration);
    assert!(fresh_at(&ttl, &observed, &expiry));
    assert_eq!(assess_fresh(&ttl, &observed, &expiry), Freshness::Fresh);
    assert_eq!(
        assess_fresh(&ttl, &observed, &(&expiry + &1.into())),
        Freshness::Stale
    );
    assert_eq!(
        assess_fresh(&ttl, &observed, &(&observed - &1.into())),
        Freshness::Unknown
    );
    assert_eq!(
        assess_fresh(&Ttl::ExpireAfter((-1).into()), &0.into(), &0.into()),
        Freshness::Stale
    );
    assert!(!fresh_at(
        &Ttl::ExpireAfter((-1).into()),
        &0.into(),
        &0.into()
    ));
    assert_eq!(
        assess_fresh(&Ttl::NoExpiry, &(-10).into(), &(-9).into()),
        Freshness::Fresh
    );
    assert_eq!(
        assess_fresh(&Ttl::NoExpiry, &1.into(), &0.into()),
        Freshness::Unknown
    );
    assert_eq!(
        assess_fresh(&Ttl::Missing, &0.into(), &0.into()),
        Freshness::Unknown
    );
    assert!(!ttl_wf(&Ttl::ExpireAfter((-1).into())));
    Ok(())
}

#[test]
fn revoking_one_witness_preserves_independent_support_and_receipt() -> TestResult {
    let (kb, context, a, b) = fixture()?;
    let receipts: Vec<_> = accepted(&kb, &context, &b)
        .into_iter()
        .enumerate()
        .map(|(index, (witness, result))| {
            Ok(Receipt::new(
                format!("receipt{index}").parse()?,
                b.clone(),
                context.clone(),
                witness.clone(),
                result.dependencies().clone(),
                ReceiptResult::Pass,
                20.into(),
            ))
        })
        .collect::<Result<_, IdentityError>>()?;
    assert_eq!(receipts.len(), 2);
    let kb = corrected(
        &kb,
        vec![Correction::new(
            "retract".parse()?,
            CorrectionKind::Retract,
            a.clone(),
            None,
            10.into(),
        )],
        receipts,
    );
    assert!(well_formed(&kb));
    assert!(accepted(&kb, &context, &a).is_empty());
    assert_eq!(accepted(&kb, &context, &b).len(), 1);
    let reusable_count = kb
        .receipts()
        .iter()
        .filter(|r| reusable(&kb, &context, &b, r))
        .count();
    assert_eq!(reusable_count, 1);
    let assessment = assess(&kb, &context, &b);
    assert_eq!(*assessment.support(), Support::Positive);
    assert!(*assessment.validated());
    assert!(*assessment.usable());
    Ok(())
}

#[test]
fn receipts_require_exact_ordered_dependencies_and_context() -> TestResult {
    let (kb, context, _, b) = fixture()?;
    let Some((witness, checked)) = accepted(&kb, &context, &b).into_iter().next() else {
        return Err("fixture lacks accepted witness".into());
    };
    let receipt = Receipt::new(
        "receipt".parse()?,
        b.clone(),
        context.clone(),
        witness.clone(),
        checked.dependencies().clone(),
        ReceiptResult::Pass,
        20.into(),
    );
    assert!(reusable(&kb, &at(&context, 20.into()), &b, &receipt));
    assert!(!reusable(&kb, &at(&context, 21.into()), &b, &receipt));
    assert!(!reusable(&kb, &at(&context, 9.into()), &b, &receipt));
    let mut dependencies = checked.dependencies().clone();
    dependencies.reverse();
    let reordered = Receipt::new(
        receipt.id().clone(),
        b.clone(),
        context.clone(),
        witness.clone(),
        dependencies,
        ReceiptResult::Pass,
        20.into(),
    );
    assert!(!reusable(&kb, &context, &b, &reordered));
    let different_use = Context::new(
        context.snapshot().clone(),
        context.semantic_version().clone(),
        context.ruleset().clone(),
        context.checker().clone(),
        context.policy().clone(),
        context.now().clone(),
        context.target().clone(),
        "another-use".into(),
        context.sources().clone(),
    );
    assert!(!reusable(&kb, &different_use, &b, &receipt));
    Ok(())
}

#[test]
fn snapshot_frame_changes_only_snapshot() -> TestResult {
    let (kb, context, _, b) = fixture()?;
    let candidate = KB::new(
        "candidate".parse()?,
        kb.observations().clone(),
        kb.claims().clone(),
        kb.attestations().clone(),
        kb.enumerations().clone(),
        kb.derivations().clone(),
        vec![],
        vec![],
    );
    assert!(!context_matches(&candidate, &context));
    assert!(!usable_for(&candidate, &context, &b));
    assert!(usable_for_snapshot(
        &candidate,
        &context,
        candidate.snapshot(),
        &b
    ));
    let Some(claim) = find_claim(&candidate, &b) else {
        return Err("fixture lacks claim".into());
    };
    let Some(derivation) = candidate.derivations().iter().find(|d| d.claim() == &b) else {
        return Err("fixture lacks derivation".into());
    };
    assert!(!check_matches(
        &candidate,
        &context,
        derivation.witness(),
        claim.statement()
    ));
    assert!(check_matches_snapshot(
        &candidate,
        &context,
        candidate.snapshot(),
        derivation.witness(),
        claim.statement()
    ));
    Ok(())
}

#[test]
fn contradictory_evidence_is_available_but_contested() -> TestResult {
    let (kb, context, _, b) = fixture()?;
    let Some(positive) = find_claim(&kb, &b) else {
        return Err("fixture lacks claim".into());
    };
    let negative = Claim::new(
        "negative".parse()?,
        "1".parse()?,
        Statement::new(
            positive.statement().formula().clone(),
            positive.statement().scope().clone(),
            Polarity::Negative,
        ),
    );
    let negative_ref = negative.claim_ref();
    let attestation = Attestation::new(
        "negative-attestation".parse()?,
        negative_ref.clone(),
        "o1".parse()?,
        negative.statement().clone(),
        "actor".parse()?,
    );
    let witness = Witness::Attest {
        attestation: attestation.id().clone(),
    };
    let mut claims = kb.claims().clone();
    claims.push(negative);
    let mut attestations = kb.attestations().clone();
    attestations.push(attestation);
    let mut derivations = kb.derivations().clone();
    derivations.push(Derivation::new(
        "negative-derivation".parse()?,
        negative_ref.clone(),
        witness,
    ));
    let kb = KB::new(
        kb.snapshot().clone(),
        kb.observations().clone(),
        claims,
        attestations,
        vec![],
        derivations,
        vec![],
        vec![],
    );
    for reference in [&b, &negative_ref] {
        let result = assess(&kb, &context, reference);
        assert_eq!(*result.support(), Support::Both);
        assert!(*result.available());
        assert!(*result.contested());
        assert!(!result.usable());
        assert_eq!(
            assessment_gaps(&kb, &context, reference),
            vec!["claim is contested by contrary evidence or dispute"]
        );
    }
    Ok(())
}

#[test]
fn freshness_and_validation_do_not_collapse_into_support() -> TestResult {
    let (kb, context, _, b) = fixture()?;
    let verdict = assess(&kb, &at(&context, 21.into()), &b);
    assert_eq!(*verdict.support(), Support::Positive);
    assert_eq!(*verdict.freshness(), Freshness::Stale);
    assert!(*verdict.available());
    assert!(!verdict.validated());
    assert!(!verdict.usable());
    assert_eq!(
        assessment_gaps(&kb, &at(&context, 21.into()), &b),
        vec!["observations are stale under policy ttl"]
    );
    assert_eq!(witness_freshness(&context, &[]), Freshness::Unknown);
    let future = Observation::new(
        "future".parse()?,
        "source".parse()?,
        "revision".parse()?,
        "content".parse()?,
        11.into(),
        ObsOutcome::Ok,
    );
    let stale = Observation::new(
        "stale".parse()?,
        "source".parse()?,
        "revision".parse()?,
        "content".parse()?,
        (-1).into(),
        ObsOutcome::Ok,
    );
    assert_eq!(
        witness_freshness(
            &context,
            &[
                Dependency::Observation(future),
                Dependency::Observation(stale)
            ]
        ),
        Freshness::Stale
    );
    Ok(())
}

#[test]
fn inventory_framing_counts_utf8_bytes_and_preserves_order() {
    assert_eq!(
        inventory_content(&["é".into(), "猫".into(), "".into()]).as_str(),
        "inventory:2:é3:猫0:"
    );
    assert_eq!(inventory_content(&[]).as_str(), "inventory:");
    assert_ne!(
        inventory_content(&["a".into(), "bc".into()]),
        inventory_content(&["ab".into(), "c".into()])
    );
}
