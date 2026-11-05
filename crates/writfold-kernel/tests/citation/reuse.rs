use super::{
    fixture::{Fixture, TestResult, allowed},
    legacy_fixture,
};
use writfold_domain::{citation::*, *};
use writfold_kernel::{checker, citation::*};

fn at(context: &Context, now: i32) -> Context {
    Context::new(
        context.snapshot().clone(),
        context.semantic_version().clone(),
        context.ruleset().clone(),
        context.checker().clone(),
        context.policy().clone(),
        now.into(),
        context.target().clone(),
        context.intended_use().clone(),
        context.sources().clone(),
    )
}
fn receipt(kb: &KB, context: &Context, claim: &ClaimRef) -> TestResult<Receipt> {
    let (witness, checked) = checker::accepted(kb, context, claim)
        .into_iter()
        .next()
        .ok_or("fixture has no accepted witness")?;
    Ok(Receipt::new(
        "receipt".parse()?,
        claim.clone(),
        context.clone(),
        witness.clone(),
        checked.dependencies().clone(),
        ReceiptResult::Pass,
        20.into(),
    ))
}

#[test]
fn citation_reuse_preserves_inclusive_legacy_ttl_and_never_updates_observations() -> TestResult {
    let (kb, context, claim, _) = legacy_fixture::fixture()?;
    let receipt = receipt(&kb, &context, &claim)?;
    let receipt_before = receipt.clone();
    let kb_before = kb.clone();
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let current = [binding];
    let access = [allowed(binding)];
    let validation = BindingValidation::new(&receipt, &f.context, &current);
    let store = [Representation::new(&f.reference, b"a")];
    for (time, expected) in [(9, false), (10, true), (20, true), (21, false)] {
        let now = at(&context, time);
        assert_eq!(checker::reusable(&kb, &now, &claim, &receipt), expected);
        assert_eq!(
            citation_reusable(
                &store,
                &access,
                &kb,
                &now,
                &f.context,
                &claim,
                &current,
                &validation
            ),
            expected
        );
    }
    assert_eq!(receipt, receipt_before);
    assert_eq!(kb, kb_before);
    assert_eq!(validation.legacy().dependencies(), receipt.dependencies());
    assert_eq!(validation.legacy().expires_at(), receipt.expires_at());
    Ok(())
}

#[test]
fn readable_unrelated_material_is_not_in_the_receipt_observation_footprint() -> TestResult {
    let (kb, context, claim, _) = legacy_fixture::fixture()?;
    let receipt = receipt(&kb, &context, &claim)?;
    let f = Fixture::new()?;
    let unrelated = RepresentationRef::new(
        f.reference.realm().clone(),
        "other".parse()?,
        f.reference.revision().clone(),
        f.reference.path().clone(),
        f.reference.content().clone(),
        RepresentationProfile::GitBlob,
    );
    let binding = f.binding_at(&unrelated, 0, 1, b"a")?;
    let current = [binding];
    let validation = BindingValidation::new(&receipt, &f.context, &current);
    let store = [Representation::new(&unrelated, b"a")];
    assert!(matches!(
        resolve(&store, binding, allowed(binding)),
        ResolveResult::Exact { .. }
    ));
    assert!(checker::reusable(&kb, &context, &claim, &receipt));
    assert!(!binding_in_footprint(receipt.dependencies(), binding));
    assert!(!citation_reusable(
        &store,
        &[allowed(binding)],
        &kb,
        &context,
        &f.context,
        &claim,
        &current,
        &validation
    ));
    Ok(())
}

#[test]
fn changed_material_named_context_or_current_access_refuses_reuse() -> TestResult {
    let (kb, context, claim, _) = legacy_fixture::fixture()?;
    let receipt = receipt(&kb, &context, &claim)?;
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let original = [binding];
    let validation = BindingValidation::new(&receipt, &f.context, &original);
    let store = [Representation::new(&f.reference, b"a")];
    let role: EvidenceRoleId = "contrary".parse()?;
    let changed = Binding::new(
        binding.id(),
        binding.release(),
        binding.context(),
        &role,
        binding.representation(),
        binding.selector(),
        binding.expected(),
    );
    let reusable = |objects: &[Representation<'_>],
                    access: &[AccessDecision<'_>],
                    named: &ContextId,
                    current: &[Binding<'_>]| {
        citation_reusable(
            objects,
            access,
            &kb,
            &context,
            named,
            &claim,
            current,
            &validation,
        )
    };
    assert!(!reusable(
        &store,
        &[allowed(changed)],
        &f.context,
        &[changed]
    ));
    let foreign: ContextId = "foreign".parse()?;
    assert!(!reusable(&store, &[allowed(binding)], &foreign, &original));
    assert!(!reusable(
        &store,
        &[AccessDecision::new(binding, false)],
        &f.context,
        &original
    ));
    assert!(!reusable(&[], &[allowed(binding)], &f.context, &original));
    assert!(!reusable(&store, &[], &f.context, &original));
    let invalid = [Representation::new(&f.reference, b"z")];
    assert!(!reusable(
        &invalid,
        &[allowed(binding)],
        &f.context,
        &original
    ));
    Ok(())
}

#[test]
fn citation_association_does_not_replace_legacy_dependency_and_witness_checks() -> TestResult {
    let (kb, context, claim, _) = legacy_fixture::fixture()?;
    let good = receipt(&kb, &context, &claim)?;
    let mut dependencies = good.dependencies().clone();
    dependencies.reverse();
    let reordered = Receipt::new(
        good.id().clone(),
        claim.clone(),
        context.clone(),
        good.witness().clone(),
        dependencies,
        ReceiptResult::Pass,
        20.into(),
    );
    let failed = Receipt::new(
        good.id().clone(),
        claim.clone(),
        context.clone(),
        good.witness().clone(),
        good.dependencies().clone(),
        ReceiptResult::Fail,
        20.into(),
    );
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let current = [binding];
    let store = [Representation::new(&f.reference, b"a")];
    for receipt in [&reordered, &failed] {
        assert!(!checker::reusable(&kb, &context, &claim, receipt));
        let validation = BindingValidation::new(receipt, &f.context, &current);
        assert!(!citation_reusable(
            &store,
            &[allowed(binding)],
            &kb,
            &context,
            &f.context,
            &claim,
            &current,
            &validation
        ));
    }
    Ok(())
}

#[test]
fn revoking_one_legacy_witness_does_not_revoke_an_independent_citation_bound_witness() -> TestResult
{
    let (kb, context, a, b) = legacy_fixture::fixture()?;
    let receipts: Vec<_> = checker::accepted(&kb, &context, &b)
        .into_iter()
        .enumerate()
        .map(|(index, (witness, checked))| -> TestResult<Receipt> {
            Ok(Receipt::new(
                format!("receipt{index}").parse()?,
                b.clone(),
                context.clone(),
                witness.clone(),
                checked.dependencies().clone(),
                ReceiptResult::Pass,
                20.into(),
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    assert_eq!(receipts.len(), 2);
    let revoked = KB::new(
        kb.snapshot().clone(),
        kb.observations().clone(),
        kb.claims().clone(),
        kb.attestations().clone(),
        kb.enumerations().clone(),
        kb.derivations().clone(),
        vec![Correction::new(
            "retract".parse()?,
            CorrectionKind::Retract,
            a,
            None,
            10.into(),
        )],
        vec![],
    );
    let f = Fixture::new()?;
    let mut usable = 0;
    for receipt in &receipts {
        let observation = receipt
            .dependencies()
            .iter()
            .find_map(|dependency| match dependency {
                Dependency::Observation(observation) => Some(observation),
                _ => None,
            })
            .ok_or("receipt has no observation dependency")?;
        let reference = RepresentationRef::new(
            f.reference.realm().clone(),
            observation.source().clone(),
            observation.revision().clone(),
            "file.txt".into(),
            observation.content().clone(),
            RepresentationProfile::GitBlob,
        );
        let binding = f.binding_at(&reference, 0, 1, b"a")?;
        let current = [binding];
        let validation = BindingValidation::new(receipt, &f.context, &current);
        let store = [Representation::new(&reference, b"a")];
        let expected = checker::reusable(&revoked, &context, &b, receipt);
        assert_eq!(
            citation_reusable(
                &store,
                &[allowed(binding)],
                &revoked,
                &context,
                &f.context,
                &b,
                &current,
                &validation
            ),
            expected
        );
        usable += usize::from(expected);
    }
    assert_eq!(usable, 1);
    Ok(())
}
