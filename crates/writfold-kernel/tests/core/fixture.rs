use std::error::Error;
use writfold_domain::*;

pub(super) fn fixture() -> Result<(KB, Context, ClaimRef, ClaimRef), Box<dyn Error>> {
    let scope = Scope::new("project".parse()?, String::new());
    let statement = Statement::new(
        Formula::Atomic(Atom::new("subject".into(), "predicate".into(), vec![])),
        scope.clone(),
        Polarity::Positive,
    );
    let a = Claim::new("a".parse()?, "1".parse()?, statement.clone());
    let b = Claim::new("b".parse()?, "1".parse()?, statement.clone());
    let ar = a.claim_ref();
    let br = b.claim_ref();
    let first = Observation::new(
        "o1".parse()?,
        "source1".parse()?,
        "r1".parse()?,
        "bytes1".parse()?,
        10.into(),
        ObsOutcome::Ok,
    );
    let second = Observation::new(
        "o2".parse()?,
        "source2".parse()?,
        "r2".parse()?,
        "bytes2".parse()?,
        10.into(),
        ObsOutcome::Ok,
    );
    let aa = Attestation::new(
        "aa".parse()?,
        ar.clone(),
        first.id().clone(),
        statement.clone(),
        "actor".parse()?,
    );
    let ab = Attestation::new(
        "ab".parse()?,
        br.clone(),
        second.id().clone(),
        statement,
        "actor".parse()?,
    );
    let wa = Witness::Attest {
        attestation: aa.id().clone(),
    };
    let wb = Witness::Attest {
        attestation: ab.id().clone(),
    };
    let context = evidence_context(scope, &first, &second)?;
    let kb = KB::new(
        context.snapshot().clone(),
        vec![first, second],
        vec![a, b],
        vec![aa, ab],
        vec![],
        vec![
            Derivation::new("da".parse()?, ar.clone(), wa.clone()),
            Derivation::new("db1".parse()?, br.clone(), wa),
            Derivation::new("db2".parse()?, br.clone(), wb),
        ],
        vec![],
        vec![],
    );
    Ok((kb, context, ar, br))
}

fn evidence_context(
    scope: Scope,
    first: &Observation,
    second: &Observation,
) -> Result<Context, IdentityError> {
    Ok(Context::new(
        "snapshot".parse()?,
        semantic_version().into(),
        "core-2".parse()?,
        "kctx-2".parse()?,
        Policy::new(
            "policy".parse()?,
            true,
            false,
            true,
            Ttl::ExpireAfter(10.into()),
            vec![],
        ),
        10.into(),
        scope,
        "use".into(),
        vec![
            SourceSelection::new(
                first.source().clone(),
                first.revision().clone(),
                first.content().clone(),
            ),
            SourceSelection::new(
                second.source().clone(),
                second.revision().clone(),
                second.content().clone(),
            ),
        ],
    ))
}
