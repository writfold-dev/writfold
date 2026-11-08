use std::error::Error;
use writfold_domain::*;

#[test]
fn identifier_families_keep_their_exact_grammar() -> Result<(), Box<dyn Error>> {
    for value in ["a", "a-b", "0", "a--b"] {
        assert_eq!(value.parse::<AssetId>()?.as_str(), value);
    }
    for value in ["", "A", "-a", "a-", "a_b", "é", " a", "a\n"] {
        assert!(value.parse::<AssetId>().is_err());
    }
    for value in ["A", "0:a.b-c_d", "x::", "x_"] {
        assert_eq!(value.parse::<ClaimId>()?.as_str(), value);
    }
    for value in ["", "_x", "a/b", "a b", "a\n", "é"] {
        assert!(value.parse::<ObservationId>().is_err());
    }
    for value in ["a", "a__b", "a-b__c__0"] {
        assert!(value.parse::<RelationshipId>().is_ok());
    }
    for value in ["__a", "a__", "a___b", "a_b", "A__b"] {
        assert!(value.parse::<RelationshipId>().is_err());
    }
    for value in ["A_B:C", "proposal/name"] {
        assert!(value.parse::<ProposalId>().is_ok());
    }
    for value in ["Proposal/name", "a/b/c", "a/-b"] {
        assert!(value.parse::<ProposalId>().is_err());
    }
    for value in ["INV:1", "2026/2025-99-00-slug"] {
        assert!(value.parse::<InvestigationId>().is_ok());
    }
    for value in ["202/2026-01-01-a", "2026/2026-1-01-a", "2026/2026-01-01-A"] {
        assert!(value.parse::<InvestigationId>().is_err());
    }
    Ok(())
}

#[test]
fn opaque_values_are_not_normalized_or_confused_with_digests() -> Result<(), Box<dyn Error>> {
    for value in [" ", "\n", "nul\0value", "é", "git:https://example/repo"] {
        assert_eq!(value.parse::<SourceId>()?.into_string(), value);
        assert_eq!(value.parse::<CheckerId>()?.as_str(), value);
    }
    assert!("".parse::<SourceId>().is_err());
    assert!("name".parse::<ContentId>().is_ok());
    assert!("name".parse::<ContentDigest>().is_err());
    let digest = "a".repeat(64);
    assert_eq!(digest.parse::<ContentDigest>()?.as_str(), digest);
    assert!("A".repeat(64).parse::<ContentDigest>().is_err());
    assert!("a".repeat(40).parse::<PlanId>().is_err());
    assert!("a".repeat(40).parse::<ReleaseId>().is_ok());
    assert!(digest.parse::<ReleaseId>().is_ok());
    assert!("a".repeat(41).parse::<ReleaseId>().is_err());
    assert!("v".repeat(32).parse::<DoltCommitId>().is_ok());
    assert!("w".repeat(32).parse::<DoltCommitId>().is_err());
    assert!("V".repeat(32).parse::<DoltCommitId>().is_err());
    assert!(GitObjectId::new(GitObjectFormat::Sha1, digest).is_err());
    assert_eq!(
        "a".repeat(64).parse::<GitObjectId>()?.format(),
        GitObjectFormat::Sha256
    );
    Ok(())
}

#[test]
fn policy_identities_preserve_historical_text() -> Result<(), Box<dyn Error>> {
    let kb = " work 知識 ";
    let version = " policy revision 1 ";
    assert_eq!(kb.parse::<KbId>()?.as_str(), kb);
    assert_eq!(version.parse::<PolicyRevision>()?.as_str(), version);
    assert!("".parse::<KbId>().is_err());
    assert!("".parse::<PolicyRevision>().is_err());
    Ok(())
}

#[test]
fn ledger_names_are_distinct_tokens_without_normalization() -> Result<(), Box<dyn Error>> {
    use std::any::TypeId;
    let identities = [
        TypeId::of::<StreamId>(),
        TypeId::of::<LedgerRecordId>(),
        TypeId::of::<RunId>(),
        TypeId::of::<ContentDigest>(),
    ];
    for (index, identity) in identities.iter().enumerate() {
        assert!(!identities[..index].contains(identity));
    }
    for value in ["A", "0:a.b-c_d", "x::", "x_"] {
        assert_eq!(value.parse::<StreamId>()?.as_str(), value);
        assert_eq!(LedgerRecordId::try_from(value.to_owned())?.as_str(), value);
    }
    for value in ["", "_x", "a/b", " a", "a ", "a\n", "é"] {
        assert!(value.parse::<StreamId>().is_err());
        assert!(LedgerRecordId::try_from(value.to_owned()).is_err());
    }
    Ok(())
}

#[test]
fn ledger_counters_are_distinct_positive_u64_values() -> Result<(), Box<dyn Error>> {
    use std::any::TypeId;
    assert_ne!(
        TypeId::of::<LedgerSequence>(),
        TypeId::of::<OwnershipGeneration>()
    );
    assert_ne!(
        TypeId::of::<OwnershipGeneration>(),
        TypeId::of::<RequestGeneration>()
    );
    for value in [
        Integer::from(-1_i64),
        Integer::from(0_u64),
        "18446744073709551616".parse()?,
    ] {
        assert!(LedgerSequence::try_from(&value).is_err());
        assert!(OwnershipGeneration::try_from(&value).is_err());
    }
    assert!(LedgerSequence::new(0).is_err());
    assert!(OwnershipGeneration::try_from(0_u64).is_err());
    for value in [1, u64::MAX] {
        let integer = Integer::from(value);
        assert_eq!(Integer::from(LedgerSequence::try_from(&integer)?), integer);
        assert_eq!(
            Integer::from(OwnershipGeneration::try_from(&integer)?),
            integer
        );
        assert_eq!(
            LedgerSequence::try_from(value)?,
            LedgerSequence::new(value)?
        );
        assert_eq!(
            OwnershipGeneration::try_from(value)?,
            OwnershipGeneration::new(value)?
        );
    }
    Ok(())
}
