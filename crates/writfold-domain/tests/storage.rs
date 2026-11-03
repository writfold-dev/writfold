use writfold_domain::{GraphCommitId, GraphId, Integer, NativeFormatVersion};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BLOCK: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const NONCE: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";

#[test]
fn graph_registry_names_match_the_pinned_native_boundary() -> TestResult {
    for value in ["alpha", "X-9-z", "-", "POLICIES", "healthZ"] {
        assert_eq!(value.parse::<GraphId>()?.as_str(), value);
    }
    assert!("a".repeat(64).parse::<GraphId>().is_ok());
    for value in [
        "",
        "policies",
        "healthz",
        "openapi",
        "graphs",
        "_internal",
        "alpha_beta",
        "../other",
        "alpha/beta",
        "alpha\\beta",
        "alpha.beta",
        " alpha",
        "alpha ",
        "αlpha",
        "alpha\0",
    ] {
        assert!(value.parse::<GraphId>().is_err(), "{value:?}");
    }
    assert!("a".repeat(65).parse::<GraphId>().is_err());
    Ok(())
}

#[test]
fn native_commit_ids_preserve_their_exact_canonical_spelling() -> TestResult {
    for value in [
        BLOCK.to_owned(),
        "00000000000000000000000000".to_owned(),
        "7ZZZZZZZZZZZZZZZZZZZZZZZZZ".to_owned(),
        format!("hb1.{BLOCK}.0.{NONCE}"),
        format!("hb1.{BLOCK}.9999.{NONCE}"),
        format!("hb1.{BLOCK}.10000.{NONCE}"),
        format!("hb1.{BLOCK}.16383.{NONCE}"),
    ] {
        assert_eq!(GraphCommitId::try_from(value.clone())?.into_string(), value);
    }
    let addressed: GraphCommitId = format!("hb1.{BLOCK}.0.{NONCE}").parse()?;
    let nonce: GraphCommitId = NONCE.parse()?;
    assert_ne!(addressed, nonce);
    Ok(())
}

#[test]
fn commit_ids_refuse_aliases_overflow_and_other_identity_namespaces() {
    for value in [
        "main".to_owned(),
        "HEAD".to_owned(),
        "hb1".to_owned(),
        "".to_owned(),
        BLOCK.to_lowercase(),
        "8ZZZZZZZZZZZZZZZZZZZZZZZZZ".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAI".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAO".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAL".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAU".to_owned(),
        "a".repeat(40),
        "a".repeat(64),
        "0".repeat(32),
        format!(" {BLOCK}"),
        format!("{BLOCK}\n"),
        format!("hb1.{BLOCK}.00.{NONCE}"),
        format!("hb1.{BLOCK}.+1.{NONCE}"),
        format!("hb1.{BLOCK}.-1.{NONCE}"),
        format!("hb1.{BLOCK}.16384.{NONCE}"),
        format!("hb1.{BLOCK}.99999.{NONCE}"),
        format!("hb1.{BLOCK}.100000.{NONCE}"),
        format!("hb1.{BLOCK}.1.{NONCE}.extra"),
        format!("hb2.{BLOCK}.1.{NONCE}"),
        format!("hb1.{BLOCK}.1.{}", NONCE.to_lowercase()),
        format!("hb1.{BLOCK}/1.{NONCE}"),
        format!("hb1.{BLOCK}..{NONCE}"),
    ] {
        assert!(value.parse::<GraphCommitId>().is_err(), "{value:?}");
    }
}

#[test]
fn history_slot_ceiling_cannot_be_bypassed_by_a_decimal_prefix() {
    for (slot, accepted) in [
        (15999, true),
        (16000, true),
        (16299, true),
        (16300, true),
        (16379, true),
        (16380, true),
        (16383, true),
        (16384, false),
        (16390, false),
        (16400, false),
        (17000, false),
        (20000, false),
    ] {
        assert_eq!(
            format!("hb1.{BLOCK}.{slot}.{NONCE}")
                .parse::<GraphCommitId>()
                .is_ok(),
            accepted
        );
    }
}

#[test]
fn native_formats_are_checked_units_not_a_hardcoded_release_number() -> TestResult {
    assert!(NativeFormatVersion::new(0).is_err());
    assert!(NativeFormatVersion::try_from(&Integer::from(-1_i64)).is_err());
    for value in [1, 14, 16, u64::MAX] {
        assert_eq!(NativeFormatVersion::new(value)?.get(), value);
    }
    let too_large: Integer = "18446744073709551616".parse()?;
    assert!(NativeFormatVersion::try_from(&too_large).is_err());
    Ok(())
}
