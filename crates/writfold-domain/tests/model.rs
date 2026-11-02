use std::error::Error;
use writfold_domain::*;

#[test]
fn semantic_integers_preserve_negatives_and_values_beyond_u64() -> Result<(), Box<dyn Error>> {
    let huge: Integer = "184467440737095516160000000000000000000".parse()?;
    let negative: Integer = "-184467440737095516160000000000000000000".parse()?;
    assert_eq!((&huge + &negative).to_i64(), Some(0));
    assert_eq!(huge.to_u64(), None);
    assert_eq!(negative.to_u64(), None);
    assert!(negative.is_negative());
    assert_eq!(huge.to_string(), "184467440737095516160000000000000000000");
    assert!(EpochSeconds::try_from(&negative).is_err());
    assert!(ByteCount::try_from(&huge).is_err());
    assert!(PageSize::new(0).is_err());
    assert!(RowLimit::new(0).is_err());
    assert_eq!(ByteCount::new(0)?.get(), 0);
    assert_eq!(WitnessDepth::new(0)?.get(), 0);
    assert_eq!(DurationSeconds::new(u64::MAX)?.get(), u64::MAX);
    let ttl = Ttl::ExpireAfter(negative.clone());
    assert_eq!(ttl.secs(), Some(&negative));
    Ok(())
}

#[test]
fn candidate_knowledge_keeps_invalid_semantics_for_kernel_judgment() -> Result<(), Box<dyn Error>> {
    let scope = Scope::new("project".parse()?, String::new());
    let statement = Statement::new(Formula::Truth, scope, Polarity::Positive);
    let claim = Claim::new("claim".parse()?, "r1".parse()?, statement);
    let reference = claim.claim_ref();
    let kb = KB::new(
        "snapshot".parse()?,
        vec![],
        vec![claim.clone(), claim],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(kb.claims().len(), 2);
    assert!(find_claim(&kb, &reference).is_some());
    let assessment = Assessment::new(
        Support::Both,
        false,
        Freshness::Unknown,
        true,
        true,
        false,
        false,
    );
    assert_eq!(assessment.support(), &Support::Both);
    assert_eq!(assessment.freshness(), &Freshness::Unknown);
    assert_eq!(assessment.validated(), &true);
    assert_eq!(assessment.available(), &false);
    Ok(())
}

#[test]
fn finite_member_formula_preserves_order_and_truth_tail() {
    let members = vec!["first".to_owned(), "second".to_owned()];
    let expected = Formula::Conjunction {
        left: Box::new(Formula::Atomic(Atom::new(
            "first".to_owned(),
            "p".to_owned(),
            vec![],
        ))),
        right: Box::new(Formula::Conjunction {
            left: Box::new(Formula::Atomic(Atom::new(
                "second".to_owned(),
                "p".to_owned(),
                vec![],
            ))),
            right: Box::new(Formula::Truth),
        }),
    };
    assert_eq!(member_formula(&members, "p"), expected);
    assert_eq!(member_formula(&[], "p"), Formula::Truth);
}
