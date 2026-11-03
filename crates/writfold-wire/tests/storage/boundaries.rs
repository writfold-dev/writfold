use super::*;

#[test]
fn duplicate_and_escape_equivalent_keys_never_reach_domain_decode() -> Result {
    for fixture in [OMNI, LEGACY] {
        for duplicate in [r#""kb":"other""#, r#""\u006bb":"kb:é""#] {
            let input = format!("{},{} }}", &fixture[..fixture.len() - 1], duplicate);
            assert!(matches!(
                decode_pin(input.as_bytes(), limits()?),
                Err(StorageWireError::Wire(Error::DuplicateKey { key, .. })) if key == "kb"
            ));
        }
    }
    // Unknown metadata is not silently discarded before raw duplicate detection.
    let input = format!(
        "{},\"extra\":{{\"x\":0,\"\\u0078\":0}}}}",
        &OMNI[..OMNI.len() - 1]
    );
    assert!(matches!(
        decode_pin(input.as_bytes(), limits()?),
        Err(StorageWireError::Wire(Error::DuplicateKey { .. }))
    ));
    Ok(())
}

#[test]
fn unknown_namespace_schema_and_backend_are_closed_refusals() -> Result {
    for (field, value, error) in [
        (
            "namespace",
            Value::from("writfold-operation"),
            StorageWireError::UnsupportedNamespace,
        ),
        (
            "schema",
            Value::from(0),
            StorageWireError::UnsupportedVersion,
        ),
        (
            "schema",
            Value::from(2),
            StorageWireError::UnsupportedVersion,
        ),
        (
            "schema",
            Value::from(-1),
            StorageWireError::UnsupportedVersion,
        ),
        (
            "schema",
            serde_json::from_str("18446744073709551616")?,
            StorageWireError::UnsupportedVersion,
        ),
        (
            "backend",
            Value::from("dolt"),
            StorageWireError::UnsupportedBackend,
        ),
        (
            "backend",
            Value::from("Omnigraph"),
            StorageWireError::UnsupportedBackend,
        ),
        (
            "backend",
            Value::from(""),
            StorageWireError::UnsupportedBackend,
        ),
    ] {
        for fixture in [OMNI, LEGACY] {
            let decoded = decode_pin(&changed(fixture, field, value.clone())?, limits()?);
            assert_eq!(decoded.as_ref().err(), Some(&error));
        }
    }
    Ok(())
}

#[test]
fn root_shapes_unknown_fields_and_mixed_variants_are_rejected() -> Result {
    for input in ["null", "true", "1", "\"pin\"", "[]", "[{}]"] {
        assert_eq!(
            decode_pin(input.as_bytes(), limits()?),
            Err(StorageWireError::InvalidShape)
        );
    }
    for (fixture, field, value) in [
        (OMNI, "extra", Value::Null),
        (LEGACY, "extra", Value::from("")),
        (
            OMNI,
            "dolt_commit",
            Value::from("0123456789abcdefghijklmnopqrstuv"),
        ),
        (LEGACY, "commit", Value::from(ULID)),
        (OMNI, "backend", Value::from("legacy-dolt")),
        (LEGACY, "backend", Value::from("omnigraph")),
    ] {
        assert_eq!(
            decode_pin(&changed(fixture, field, value)?, limits()?),
            Err(StorageWireError::InvalidShape)
        );
    }
    Ok(())
}

#[test]
fn every_field_is_required_and_nonnullable_with_exact_type() -> Result {
    for fixture in [OMNI, LEGACY] {
        let original: Value = serde_json::from_str(fixture)?;
        let object = original.as_object().ok_or("fixture is not an object")?;
        for (field, value) in object {
            let mut missing = object.clone();
            missing.remove(field);
            assert!(
                decode_pin(&serde_json::to_vec(&missing)?, limits()?).is_err(),
                "{field}"
            );
            for wrong in [
                Value::Null,
                Value::Bool(true),
                Value::Array(Vec::new()),
                Value::Object(Default::default()),
            ] {
                assert!(
                    matches!(
                        decode_pin(&changed(fixture, field, wrong)?, limits()?),
                        Err(StorageWireError::InvalidField { field: rejected }) if rejected == field
                    ),
                    "{field}"
                );
            }
            let wrong = if value.is_string() {
                Value::from(1)
            } else {
                Value::from("1")
            };
            assert!(
                matches!(
                    decode_pin(&changed(fixture, field, wrong)?, limits()?),
                    Err(StorageWireError::InvalidField { field: rejected }) if rejected == field
                ),
                "{field}"
            );
        }
    }
    Ok(())
}

#[test]
fn integers_do_not_accept_float_exponent_string_bool_or_overflow() -> Result {
    for (field, old, fixture) in [
        ("schema", "1", OMNI),
        ("schema", "1", LEGACY),
        ("native_format", "18446744073709551615", OMNI),
    ] {
        for token in [
            "1.0", "1e0", "1E+0", "\"1\"", "true", "false", "null", "[]", "{}", "-0.0",
        ] {
            let bytes = fixture.replace(
                &format!("\"{field}\":{old}"),
                &format!("\"{field}\":{token}"),
            );
            assert_eq!(
                decode_pin(bytes.as_bytes(), limits()?),
                Err(StorageWireError::InvalidField { field }),
                "{token}"
            );
        }
    }
    for token in [
        "0",
        "-0",
        "-1",
        "18446744073709551616",
        "9999999999999999999999999999999999999999",
    ] {
        let input = OMNI.replace("18446744073709551615", token);
        assert_eq!(
            decode_pin(input.as_bytes(), limits()?),
            Err(StorageWireError::InvalidField {
                field: "native_format"
            }),
            "{token}"
        );
    }
    Ok(())
}

#[test]
fn malformed_json_trailing_data_and_utf8_are_raw_errors() -> Result {
    for input in [b"{".as_slice(), b"\xff", b"{}{}", b"{\"schema\":01}"] {
        assert!(matches!(
            decode_pin(input, limits()?),
            Err(StorageWireError::Wire(_))
        ));
    }
    let trailing = format!("{OMNI} null");
    assert!(matches!(
        decode_pin(trailing.as_bytes(), limits()?),
        Err(StorageWireError::Wire(Error::TrailingInput { .. }))
    ));
    Ok(())
}
