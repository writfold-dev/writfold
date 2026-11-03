use serde_json::Value;
use writfold_domain::{LegacyDoltPin, NativeFormatVersion, OmnigraphPin, SchemaEpoch, StoragePin};
use writfold_wire::storage::{StorageWireError, decode_pin, encode_pin};
use writfold_wire::{Budget, Error, Limits};

#[path = "storage/boundaries.rs"]
mod boundaries;
#[path = "storage/identities.rs"]
mod identities;

const OMNI: &str = include_str!("../fixtures/storage/omnigraph.json");
const LEGACY: &str = include_str!("../fixtures/storage/legacy-dolt.json");
const ULID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const COMMIT: &str = "hb1.01ARZ3NDEKTSV4RRFFQ69G5FAV.16383.7ZZZZZZZZZZZZZZZZZZZZZZZZZ";
type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn limits() -> Result<Limits> {
    Ok(Limits::new(16_384, 16, 128, 512)?)
}

fn omnigraph() -> Result<StoragePin> {
    Ok(StoragePin::Omnigraph(OmnigraphPin::new(
        "kb:é".parse()?,
        "Graph-01".parse()?,
        COMMIT.parse()?,
        SchemaEpoch::new(
            "epoch-1".parse()?,
            "a".repeat(64).parse()?,
            "b".repeat(64).parse()?,
        ),
        "reader-1".parse()?,
        NativeFormatVersion::new(u64::MAX)?,
        "c".repeat(64).parse()?,
    )))
}

fn legacy() -> Result<StoragePin> {
    Ok(StoragePin::LegacyDolt(LegacyDoltPin::new(
        "kb:é".parse()?,
        "0123456789abcdefghijklmnopqrstuv".parse()?,
    )))
}

fn changed(fixture: &str, field: &str, value: Value) -> Result<Vec<u8>> {
    let mut object: Value = serde_json::from_str(fixture)?;
    object[field] = value;
    Ok(serde_json::to_vec(&object)?)
}

#[test]
fn canonical_goldens_match_independently_constructed_domain_pins() -> Result {
    for (pin, fixture) in [(omnigraph()?, OMNI), (legacy()?, LEGACY)] {
        assert_eq!(encode_pin(&pin, fixture.len())?, fixture.as_bytes());
        assert_eq!(decode_pin(fixture.as_bytes(), limits()?)?, pin);
        let value: Value = serde_json::from_str(fixture)?;
        let pretty = serde_json::to_vec_pretty(&value)?;
        assert_eq!(decode_pin(&pretty, limits()?)?, pin);
    }
    Ok(())
}

#[test]
fn escaping_and_field_order_are_canonicalized_not_required_on_input() -> Result {
    let input = r#" { "schema": 1, "namespace": "writfold-storage-pin", "kb": "kb:\u00e9", "dolt_commit": "0123456789abcdefghijklmnopqrstuv", "\u0062ackend": "legacy-dolt" } "#;
    let pin = decode_pin(input.as_bytes(), limits()?)?;
    assert_eq!(pin, legacy()?);
    assert_eq!(encode_pin(&pin, LEGACY.len())?, LEGACY.as_bytes());
    Ok(())
}

#[test]
fn exact_integer_values_survive_above_binary64_precision() -> Result {
    for number in [1, 9_007_199_254_740_993, u64::MAX] {
        let bytes = changed(OMNI, "native_format", Value::from(number))?;
        let pin = decode_pin(&bytes, limits()?)?;
        let StoragePin::Omnigraph(omni) = &pin else {
            return Err("decoded wrong backend".into());
        };
        assert_eq!(omni.native_format().get(), number);
        let encoded = encode_pin(&pin, 16_384)?;
        let value: Value = serde_json::from_slice(&encoded)?;
        assert_eq!(value["native_format"].as_u64(), Some(number));
    }
    Ok(())
}

#[test]
fn commit_forms_remain_exact_and_distinct() -> Result {
    let values = [
        ULID.to_owned(),
        "00000000000000000000000000".to_owned(),
        "7ZZZZZZZZZZZZZZZZZZZZZZZZZ".to_owned(),
        format!("hb1.{ULID}.0.{ULID}"),
        format!("hb1.{ULID}.16383.{ULID}"),
        COMMIT.to_owned(),
    ];
    for commit in &values {
        let bytes = changed(OMNI, "commit", Value::from(commit.as_str()))?;
        let pin = decode_pin(&bytes, limits()?)?;
        let StoragePin::Omnigraph(omni) = &pin else {
            return Err("decoded wrong backend".into());
        };
        assert_eq!(omni.commit().as_str(), commit);
        let value: Value = serde_json::from_slice(&encode_pin(&pin, 16_384)?)?;
        assert_eq!(value["commit"].as_str(), Some(commit.as_str()));
    }
    let first = decode_pin(&changed(OMNI, "commit", Value::from(ULID))?, limits()?)?;
    let second = decode_pin(
        &changed(OMNI, "commit", Value::from(format!("hb1.{ULID}.0.{ULID}")))?,
        limits()?,
    )?;
    assert_ne!(first, second);
    Ok(())
}

#[test]
fn budgets_accept_exact_boundary_and_reject_one_less() -> Result {
    for (fixture, entries) in [(OMNI, 12), (LEGACY, 5)] {
        let exact = Limits::new(fixture.len(), 1, entries, entries + 1)?;
        let pin = decode_pin(fixture.as_bytes(), exact)?;
        assert_eq!(encode_pin(&pin, fixture.len())?, fixture.as_bytes());
        let cases = [
            (
                Limits::new(fixture.len() - 1, 1, entries, entries + 1)?,
                Budget::Bytes,
            ),
            (
                Limits::new(fixture.len(), 0, entries, entries + 1)?,
                Budget::Depth,
            ),
            (
                Limits::new(fixture.len(), 1, entries - 1, entries + 1)?,
                Budget::ContainerEntries,
            ),
            (
                Limits::new(fixture.len(), 1, entries, entries)?,
                Budget::Nodes,
            ),
        ];
        for (limit, expected) in cases {
            assert!(matches!(
                decode_pin(fixture.as_bytes(), limit),
                Err(StorageWireError::Wire(Error::Limit { budget, .. })) if budget == expected
            ));
        }
        for budget in [0, fixture.len() - 1] {
            assert!(matches!(
                encode_pin(&pin, budget),
                Err(StorageWireError::Wire(Error::Limit {
                    budget: Budget::Bytes,
                    ..
                }))
            ));
        }
    }
    Ok(())
}

#[test]
fn escaping_expansion_obeys_output_budget() -> Result {
    let pin = StoragePin::LegacyDolt(LegacyDoltPin::new(
        "\n\u{0}\"\\é".parse()?,
        "0123456789abcdefghijklmnopqrstuv".parse()?,
    ));
    let golden = "{\"backend\":\"legacy-dolt\",\"dolt_commit\":\"0123456789abcdefghijklmnopqrstuv\",\"kb\":\"\\n\\u0000\\\"\\\\é\",\"namespace\":\"writfold-storage-pin\",\"schema\":1}";
    assert_eq!(encode_pin(&pin, golden.len())?, golden.as_bytes());
    assert!(matches!(
        encode_pin(&pin, golden.len() - 1),
        Err(StorageWireError::Wire(Error::Limit {
            budget: Budget::Bytes,
            ..
        }))
    ));
    assert_eq!(decode_pin(golden.as_bytes(), limits()?)?, pin);
    Ok(())
}
