use writfold_wire::{ApprovalDomain, Budget, Error, Limits, OperationVariant, RawJson, canonical};

fn limits() -> Result<Limits, Error> {
    Limits::new(16_384, 32, 128, 512)
}

#[test]
fn python_canonical_integer_and_unicode_golden() -> Result<(), Error> {
    // Derived from adapters/files/knowledge.py canonical() settings, including
    // Python scalar ordering (not JavaScript UTF-16 ordering) and -0 -> 0.
    let input = r#"{"😀": -0, "\ue000":"é/\u2028\b\f\n\r\t\u0000\u001f\"\\", "a":[-184467440737095516160000,184467440737095516160001]}"#;
    let raw = RawJson::parse(input.as_bytes(), limits()?)?;
    let expected = "{\"a\":[-184467440737095516160000,184467440737095516160001],\"\u{e000}\":\"é/\u{2028}\\b\\f\\n\\r\\t\\u0000\\u001f\\\"\\\\\",\"😀\":0}";
    assert_eq!(canonical(&raw, 16_384)?, expected.as_bytes());
    Ok(())
}

#[test]
fn nested_duplicate_keys_and_number_token_are_distinct() -> Result<(), Error> {
    for input in [
        r#"{"outer":[{"x":1,"\u0078":2}]}"#,
        r#"{"$serde_json::private::Number":1,"$serde_json::private::Number":2}"#,
    ] {
        assert!(matches!(
            RawJson::parse(input.as_bytes(), limits()?),
            Err(Error::DuplicateKey { .. })
        ));
    }
    let input = br#"{"$serde_json::private::Number":"123","n":999999999999999999999999999999999}"#;
    let raw = RawJson::parse(input, limits()?)?;
    assert_eq!(canonical(&raw, 16_384)?, input);
    Ok(())
}

#[test]
fn unicode_and_numeric_refusals() -> Result<(), Error> {
    for input in [
        br#""\ud800""#.as_slice(),
        br#""\udc00""#,
        br#""\ud800x""#,
        b"\"\xff\"",
        b"\"\xc0\xaf\"",
        b"\"\xed\xa0\x80\"",
        b"\"\x01\"",
    ] {
        assert!(RawJson::parse(input, limits()?).is_err());
    }
    for input in [
        "1.",
        "1e",
        "1e+",
        ".5",
        "1.e2",
        "01.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "01",
        "-01",
        "+1",
    ] {
        assert!(RawJson::parse(input.as_bytes(), limits()?).is_err());
    }
    for input in ["null true", "{}[]", "0 trailing"] {
        assert!(matches!(
            RawJson::parse(input.as_bytes(), limits()?),
            Err(Error::TrailingInput { .. })
        ));
    }
    let pair = RawJson::parse(br#""\ud83d\ude00""#, limits()?)?;
    assert_eq!(canonical(&pair, 16)?, "\"😀\"".as_bytes());
    Ok(())
}

#[test]
fn canonical_float_bytes_follow_python_binary64_repr() -> Result<(), Error> {
    // Python json.loads followed by knowledge.py canonical(): repr thresholds,
    // nearest-binary64 rounding, signed zero, subnormals and exponent spelling.
    for (input, expected) in [
        ("1.0", "1.0"),
        ("1e0", "1.0"),
        ("-0.0", "-0.0"),
        ("-0E+100", "-0.0"),
        ("1e-4", "0.0001"),
        ("1e-5", "1e-05"),
        ("1e15", "1000000000000000.0"),
        ("1e16", "1e+16"),
        ("1E+23", "1e+23"),
        ("-12.50", "-12.5"),
        ("0.10000000000000000555", "0.1"),
        ("9007199254740993.0", "9007199254740992.0"),
        ("1000000000000000100.0", "1.0000000000000001e+18"),
        ("1.234567890123456789", "1.2345678901234567"),
        ("2.2250738585072014e-308", "2.2250738585072014e-308"),
        ("5e-324", "5e-324"),
        ("1.7976931348623157e308", "1.7976931348623157e+308"),
        ("1e-4000", "0.0"),
        ("-1e-4000", "-0.0"),
    ] {
        let raw = RawJson::parse(input.as_bytes(), limits()?)?;
        assert_eq!(canonical(&raw, 128)?, expected.as_bytes(), "{input}");
    }
    let mixed = RawJson::parse(br#"{"b":1e-5,"a":[-0,0.0,9007199254740993]}"#, limits()?)?;
    assert_eq!(
        canonical(&mixed, 128)?,
        br#"{"a":[0,0.0,9007199254740993],"b":1e-05}"#
    );
    Ok(())
}

#[test]
fn float_tokens_remain_distinct_from_integers_and_overflow_is_not_signed() -> Result<(), Error> {
    for input in ["1.0", "1e0", "-0.0", "1e309", "-1e4000"] {
        let raw = RawJson::parse(input.as_bytes(), limits()?)?;
        let spelling = raw.value().to_string();
        assert!(spelling.contains(['.', 'e', 'E']), "{input}");
        if input == "1e309" || input == "-1e4000" {
            assert!(matches!(
                canonical(&raw, 128),
                Err(Error::UnsupportedNumber { .. })
            ));
        }
    }
    let raw = RawJson::parse(b"1e15", limits()?)?;
    assert!(matches!(
        canonical(&raw, 17),
        Err(Error::Limit {
            budget: Budget::Bytes,
            ..
        })
    ));
    Ok(())
}

#[test]
fn resource_limits_refuse_before_unbounded_work() -> Result<(), Error> {
    let cases = [
        ("null", Limits::new(3, 8, 8, 8)?, Budget::Bytes),
        ("[[]]", Limits::new(64, 1, 8, 8)?, Budget::Depth),
        ("[0,1]", Limits::new(64, 8, 1, 8)?, Budget::ContainerEntries),
        (
            r#"{"a":0,"b":1}"#,
            Limits::new(64, 8, 1, 8)?,
            Budget::ContainerEntries,
        ),
        ("[0]", Limits::new(64, 8, 8, 1)?, Budget::Nodes),
    ];
    for (input, limits, expected) in cases {
        assert!(matches!(RawJson::parse(input.as_bytes(), limits),
            Err(Error::Limit { budget, .. }) if budget == expected));
    }
    let raw = RawJson::parse(b"[0]", Limits::new(3, 1, 1, 2)?)?;
    assert_eq!(canonical(&raw, 3)?, b"[0]");
    assert!(matches!(
        canonical(&raw, 2),
        Err(Error::Limit {
            budget: Budget::Bytes,
            ..
        })
    ));
    Ok(())
}

#[test]
fn legacy_full_outer_plan_message_golden() -> Result<(), Error> {
    // Byte-level envelope fixture, not a semantically valid admission claim.
    // Exact field set from knowledge_models.LegacyAdmissionPlan.wire(); signing
    // composition from knowledge.py _verify() and test_knowledge_portfolio.py.
    let raw = RawJson::parse(br#"{"schema_version":2,"kind":"admission","id":"p","base":null,"policy":"q","captures":["c"],"context":{},"plan":{"z":-999999999999999999999999},"toolchain":"t"}"#, limits()?)?;
    let selected = OperationVariant::select(&raw)?;
    assert_eq!(selected, OperationVariant::LegacyAdmissionPlan);
    assert_eq!(selected.domain(), ApprovalDomain::HistoricalSchema2);
    let expected = b"kctx-operation-v2\0{\"base\":null,\"captures\":[\"c\"],\"context\":{},\"id\":\"p\",\"kind\":\"admission\",\"plan\":{\"z\":-999999999999999999999999},\"policy\":\"q\",\"schema_version\":2,\"toolchain\":\"t\"}";
    assert_eq!(selected.approval_message(&raw, 16_384)?, expected);
    assert!(
        OperationVariant::AdmissionPlan
            .approval_message(&raw, 16_384)
            .is_err()
    );
    Ok(())
}

#[test]
fn all_operation_variants_select_one_domain() -> Result<(), Error> {
    for (version, kind, extra, context, expected) in [
        (
            2,
            "selection",
            r#""retained":"r""#,
            r#""context":{}"#,
            OperationVariant::LegacyReleaseSelectionPlan,
        ),
        (
            3,
            "admission",
            r#""captures":[]"#,
            r#""target_use":"review""#,
            OperationVariant::AdmissionPlan,
        ),
        (
            3,
            "selection",
            r#""retained":"r""#,
            r#""target_use":"review""#,
            OperationVariant::ReleaseSelectionPlan,
        ),
    ] {
        let input = format!(
            r#"{{"schema_version":{version},"kind":"{kind}","id":"p","base":"b","policy":"q",{extra},{context},"plan":{{}},"toolchain":"t"}}"#
        );
        let raw = RawJson::parse(input.as_bytes(), limits()?)?;
        assert_eq!(OperationVariant::select(&raw)?, expected);
        assert!(
            expected
                .approval_message(&raw, 16_384)?
                .starts_with(expected.domain().prefix())
        );
    }
    for input in [
        r#"{"schema_version":4,"kind":"admission"}"#,
        r#"{"schema_version":true,"kind":"admission"}"#,
        r#"{"schema_version":3,"kind":"unknown"}"#,
        r#"{"schema_version":2,"kind":"admission"}"#,
    ] {
        let raw = RawJson::parse(input.as_bytes(), limits()?)?;
        assert!(OperationVariant::select(&raw).is_err());
    }
    Ok(())
}

fn deep_object_array(depth: usize) -> String {
    let objects = depth / 2;
    let arrays = depth - objects;
    format!(
        "{}{}-184467440737095516160001{}{}",
        r#"{"a":"#.repeat(objects),
        "[".repeat(arrays),
        "]".repeat(arrays),
        "}".repeat(objects)
    )
}

#[test]
fn deeply_nested_values_round_trip_and_drop_on_small_stack()
-> Result<(), Box<dyn std::error::Error>> {
    let task = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let input = deep_object_array(1024);
            let limits = Limits::new(input.len(), 1024, 2, 2048)?;
            let raw = RawJson::parse(input.as_bytes(), limits)?;
            assert_eq!(canonical(&raw, input.len())?, input.as_bytes());
            assert!(matches!(
                canonical(&raw, input.len() - 1),
                Err(Error::Limit {
                    budget: Budget::Bytes,
                    ..
                })
            ));
            drop(raw);
            assert!(matches!(
                RawJson::parse(input.as_bytes(), Limits::new(input.len(), 1023, 2, 2048)?),
                Err(Error::Limit {
                    budget: Budget::Depth,
                    ..
                })
            ));
            Ok::<(), Error>(())
        })?;
    task.join()
        .map_err(|_| std::io::Error::other("deep value worker failed"))??;
    Ok(())
}

#[test]
fn deep_partial_trees_are_cleaned_on_every_refusal_path() -> Result<(), Box<dyn std::error::Error>>
{
    let task = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let deep = deep_object_array(1000);
            let limits = Limits::new(64 * 1024, 1024, 4, 4096)?;
            for input in [
                format!("[{deep},"),
                format!(r#"{{"first":{deep},"first":0}}"#),
                format!(r#"{{"first":{deep},"next" 0}}"#),
                format!(r#"{{"first":{deep},"next":"\ud800"}}"#),
                format!("{deep} true"),
                format!("[{deep},{}0{}]", "[".repeat(1024), "]".repeat(1024)),
            ] {
                assert!(RawJson::parse(input.as_bytes(), limits).is_err());
            }
            let input = format!("[{deep},0]");
            assert!(matches!(
                RawJson::parse(input.as_bytes(), Limits::new(input.len(), 1024, 4, 1002)?),
                Err(Error::Limit {
                    budget: Budget::Nodes,
                    ..
                })
            ));
            assert!(matches!(
                RawJson::parse(input.as_bytes(), Limits::new(input.len(), 1024, 1, 4096)?),
                Err(Error::Limit {
                    budget: Budget::ContainerEntries,
                    ..
                })
            ));
            Ok::<(), Error>(())
        })?;
    task.join()
        .map_err(|_| std::io::Error::other("partial tree worker failed"))??;
    Ok(())
}
