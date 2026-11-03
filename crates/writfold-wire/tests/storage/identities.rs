use super::*;

#[test]
fn malformed_commit_spellings_are_not_normalized_or_aliased() -> Result {
    let invalid = [
        "".to_owned(),
        "HEAD".to_owned(),
        "a".repeat(40),
        "0123456789abcdefghijklmnopqrstuv".to_owned(),
        ULID.to_lowercase(),
        format!(" {ULID}"),
        format!("{ULID}\n"),
        "8ZZZZZZZZZZZZZZZZZZZZZZZZZ".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAI".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAO".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAL".to_owned(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAU".to_owned(),
        ULID[..25].to_owned(),
        format!("{ULID}0"),
        format!("hb1.{ULID}.00.{ULID}"),
        format!("hb1.{ULID}.01.{ULID}"),
        format!("hb1.{ULID}.+1.{ULID}"),
        format!("hb1.{ULID}.-1.{ULID}"),
        format!("hb1.{ULID}.16384.{ULID}"),
        format!("hb1.{ULID}.184467440737095516160.{ULID}"),
        format!("hb1.{ULID}..{ULID}"),
        format!("hb1.{ULID}.1.0.{ULID}"),
        format!("hb1.{ULID}.1e0.{ULID}"),
        format!("hb1.{ULID}.0.{}", ULID.to_lowercase()),
        format!("hb1.{}.0.{ULID}", ULID.to_lowercase()),
        format!("hb1.{ULID}.0.8ZZZZZZZZZZZZZZZZZZZZZZZZZ"),
        format!("hb1.{ULID}.0.{ULID}.extra"),
        format!("hb2.{ULID}.0.{ULID}"),
        format!("HB1.{ULID}.0.{ULID}"),
    ];
    for commit in invalid {
        assert_eq!(
            decode_pin(
                &changed(OMNI, "commit", Value::from(commit.as_str()))?,
                limits()?
            ),
            Err(StorageWireError::InvalidIdentity { field: "commit" }),
            "{commit}"
        );
    }
    Ok(())
}

#[test]
fn every_nominal_identity_is_checked_at_decode_boundary() -> Result {
    for (field, value) in [
        ("kb", ""),
        ("graph", ""),
        ("graph", "policies"),
        ("graph", "healthz"),
        ("graph", "openapi"),
        ("graph", "graphs"),
        ("graph", "graph_name"),
        ("graph", "graph/name"),
        ("graph", "gráph"),
        ("epoch", ""),
        ("epoch", "two words"),
        ("storage_profile", ""),
        ("storage_profile", "reader/profile"),
        ("schema_digest", ""),
        ("query_digest", "v1"),
        ("producer_generation", "0.14.0"),
    ] {
        assert_eq!(
            decode_pin(&changed(OMNI, field, Value::from(value))?, limits()?),
            Err(StorageWireError::InvalidIdentity { field })
        );
    }
    for field in ["schema_digest", "query_digest", "producer_generation"] {
        for invalid in [
            "A".repeat(64),
            "g".repeat(64),
            "a".repeat(63),
            "a".repeat(65),
        ] {
            assert_eq!(
                decode_pin(&changed(OMNI, field, Value::from(invalid))?, limits()?),
                Err(StorageWireError::InvalidIdentity { field })
            );
        }
    }
    for field in ["kb", "dolt_commit"] {
        assert_eq!(
            decode_pin(&changed(LEGACY, field, Value::from(""))?, limits()?),
            Err(StorageWireError::InvalidIdentity { field })
        );
    }
    for invalid in [
        ULID.to_owned(),
        "A".repeat(32),
        "w".repeat(32),
        "a".repeat(31),
        "a".repeat(33),
    ] {
        assert_eq!(
            decode_pin(
                &changed(LEGACY, "dolt_commit", Value::from(invalid))?,
                limits()?
            ),
            Err(StorageWireError::InvalidIdentity {
                field: "dolt_commit"
            })
        );
    }
    Ok(())
}

#[test]
fn graph_length_boundary_and_case_are_exact() -> Result {
    for graph in [
        "a".to_owned(),
        "-".to_owned(),
        "a".repeat(64),
        "Policies".to_owned(),
    ] {
        let pin = decode_pin(
            &changed(OMNI, "graph", Value::from(graph.as_str()))?,
            limits()?,
        )?;
        let StoragePin::Omnigraph(pin) = pin else {
            return Err("decoded wrong backend".into());
        };
        assert_eq!(pin.graph().as_str(), graph);
    }
    assert_eq!(
        decode_pin(
            &changed(OMNI, "graph", Value::from("a".repeat(65)))?,
            limits()?
        ),
        Err(StorageWireError::InvalidIdentity { field: "graph" })
    );
    Ok(())
}
