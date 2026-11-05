use writfold_domain::{citation::*, *};

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

#[test]
fn citation_namespaces_preserve_exact_checked_tokens() -> TestResult {
    for text in ["realm", "scope:one", "role-1"] {
        assert_eq!(text.parse::<RealmId>()?.as_str(), text);
        assert_eq!(text.parse::<EvidenceRoleId>()?.as_str(), text);
        assert_eq!(text.parse::<AliasId>()?.as_str(), text);
    }
    for text in ["", " bad", "bad ", "bad\nname"] {
        assert!(text.parse::<RealmId>().is_err());
        assert!(text.parse::<EvidenceRoleId>().is_err());
        assert!(text.parse::<AliasId>().is_err());
    }
    Ok(())
}

#[test]
fn citation_digest_namespaces_require_exact_lower_hex() -> TestResult {
    let valid = "ab".repeat(32);
    assert_eq!(
        valid.parse::<CitationRepresentationDigest>()?.as_str(),
        valid
    );
    assert_eq!(valid.parse::<CitationBindingDigest>()?.as_str(), valid);
    assert_eq!(
        valid.parse::<CitationCorrespondenceDigest>()?.as_str(),
        valid
    );
    for invalid in [
        "AB".repeat(32),
        "a".repeat(63),
        "a".repeat(65),
        format!(" {valid}"),
    ] {
        assert!(invalid.parse::<CitationRepresentationDigest>().is_err());
        assert!(invalid.parse::<CitationBindingDigest>().is_err());
        assert!(invalid.parse::<CitationCorrespondenceDigest>().is_err());
    }
    Ok(())
}

#[test]
fn selector_admission_checks_order_but_does_not_authenticate_or_assume_object_length() -> TestResult
{
    let first = ByteOffset::new(2)?;
    let last = ByteOffset::new(1)?;
    assert_eq!(
        Selector::new(first, last, CoordinateUnit::Byte),
        Err(SelectorError::ReversedBounds)
    );
    let empty =
        Selector::new(first, first, CoordinateUnit::Byte).map_err(|error| format!("{error:?}"))?;
    assert_eq!(empty.first().get(), 2);
    assert_eq!(empty.last().get(), 2);
    let unsupported =
        Selector::new(last, first, CoordinateUnit::Utf16).map_err(|error| format!("{error:?}"))?;
    assert_eq!(unsupported.unit(), CoordinateUnit::Utf16);
    let wide = Selector::new(
        ByteOffset::new(0)?,
        ByteOffset::new(u64::MAX)?,
        CoordinateUnit::Byte,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(wide.first().get(), 0);
    assert_eq!(wide.last().get(), u64::MAX);
    assert!(!Coverage::Unavailable.complete());
    assert!(
        Coverage::Measured {
            examined: 0,
            total: 0
        }
        .complete()
    );
    assert!(
        !Coverage::Measured {
            examined: 0,
            total: 1
        }
        .complete()
    );
    Ok(())
}

#[test]
fn citation_table_epoch_exhaustion_cannot_reuse_an_old_mapping() -> TestResult {
    let penultimate = CitationTableGeneration::from_u64(u64::MAX - 1);
    let last = penultimate.checked_next();
    assert_eq!(last.map(CitationTableGeneration::get), Some(u64::MAX));
    assert_eq!(last.and_then(CitationTableGeneration::checked_next), None);
    Ok(())
}
