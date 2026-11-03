use super::{TestResult, fixture};
use std::error::Error;
use writfold_domain::*;
use writfold_kernel::{checker::*, wellformed::*};

fn candidate(canonical_support: bool) -> Result<(KB, Context, ClaimRef), Box<dyn Error>> {
    let (base, context, reference, _) = fixture()?;
    let Some(canonical) = find_claim(&base, &reference) else {
        return Err("fixture lacks canonical claim".into());
    };
    let negative_statement = Statement::new(
        canonical.statement().formula().clone(),
        canonical.statement().scope().clone(),
        Polarity::Negative,
    );
    let negative = Claim::new(
        "negative".parse()?,
        "1".parse()?,
        negative_statement.clone(),
    );
    let negative_attestation = Attestation::new(
        "negative-attestation".parse()?,
        negative.claim_ref(),
        "o1".parse()?,
        negative_statement.clone(),
        "actor".parse()?,
    );
    let canonical_attestation = Attestation::new(
        "canonical-attestation".parse()?,
        reference.clone(),
        "o1".parse()?,
        canonical.statement().clone(),
        "actor".parse()?,
    );
    let mut derivations = vec![Derivation::new(
        "mismatched-derivation".parse()?,
        reference.clone(),
        Witness::Attest {
            attestation: negative_attestation.id().clone(),
        },
    )];
    if canonical_support {
        derivations.push(Derivation::new(
            "canonical-derivation".parse()?,
            reference.clone(),
            Witness::Attest {
                attestation: canonical_attestation.id().clone(),
            },
        ));
    }
    let kb = KB::new(
        base.snapshot().clone(),
        base.observations().clone(),
        vec![
            canonical.clone(),
            Claim::new(
                reference.id().clone(),
                reference.rev().clone(),
                negative_statement,
            ),
            negative,
        ],
        vec![negative_attestation, canonical_attestation],
        vec![],
        derivations,
        vec![],
        vec![],
    );
    Ok((kb, context, reference))
}

#[test]
fn contrary_duplicate_uses_first_claim_acceptance_without_rejecting_the_kb() -> TestResult {
    for canonical_support in [false, true] {
        let (kb, context, reference) = candidate(canonical_support)?;
        let negative_ref = kb.claims()[2].claim_ref();
        assert!(context_matches(&kb, &context));
        assert!(!well_formed(&kb));
        assert!(check(&kb, &context, kb.derivations()[0].witness()).is_some());
        assert!(accepted(&kb, &context, &negative_ref).is_empty());
        let canonical_results = accepted(&kb, &context, &reference);
        assert_eq!(canonical_results.len(), usize::from(canonical_support));
        if canonical_support {
            assert_eq!(canonical_results[0].0, kb.derivations()[1].witness());
            assert_eq!(
                canonical_results[0].1.statement(),
                kb.claims()[0].statement()
            );
        }
        assert_eq!(
            supported(&kb, &context, &reference, Polarity::Positive),
            canonical_support
        );
        // The contrary row selects a reference; acceptance resolves its first row.
        // Ignoring duplicates or requiring whole-KB admission fails this control.
        assert_eq!(
            supported(&kb, &context, &reference, Polarity::Negative),
            canonical_support
        );
        let assessment = assess(&kb, &context, &reference);
        assert_eq!(
            *assessment.support(),
            if canonical_support {
                Support::Both
            } else {
                Support::None
            }
        );
        assert_eq!(*assessment.available(), canonical_support);
        assert_eq!(*assessment.contested(), canonical_support);
        assert!(!assessment.usable());
    }
    Ok(())
}
