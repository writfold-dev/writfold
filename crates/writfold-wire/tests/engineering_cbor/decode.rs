use super::{TestResult, limits, unhex};
use writfold_wire::engineering::cbor::{Decoder, IntegerRef, TokenValue};

#[test]
fn typed_schema_reads_borrowed_fields_and_exact_container_boundaries() -> TestResult {
    // [null, false, true, h'00ff', "e◌́", {"b": [0, -1], "aa": {}}]
    let bytes = unhex("86f6f4f54200ff6365cc81a26162820020626161a0")?;
    let mut decoder = Decoder::new(&bytes, limits()?)?;
    assert_eq!(decoder.array()?, 6);
    decoder.null()?;
    assert!(!decoder.boolean()?);
    assert!(decoder.boolean()?);
    let payload = decoder.bytes()?;
    assert_eq!(payload, &[0, 255]);
    let text = decoder.text()?;
    assert_eq!(text, "e\u{301}");
    assert_eq!(decoder.map()?, 2);
    assert_eq!(decoder.key()?, "b");
    assert_eq!(decoder.array()?, 2);
    assert_eq!(decoder.unsigned()?, 0);
    assert_eq!(decoder.integer()?, IntegerRef::Negative(0));
    let array = decoder.end_array()?;
    assert_eq!(array, &[0x82, 0, 0x20]);
    assert_eq!(decoder.key()?, "aa");
    assert_eq!(decoder.map()?, 0);
    assert_eq!(decoder.end_map()?, &[0xa0]);
    assert_eq!(decoder.end_map()?, &bytes[11..]);
    assert_eq!(decoder.end_array()?, bytes.as_slice());
    let raw = decoder.finish()?;
    assert_eq!(raw.as_bytes(), bytes);
    Ok(())
}

#[test]
fn native_integer_extremes_and_borrowed_bignums_are_lossless() -> TestResult {
    for (hex, expected) in [
        ("00", IntegerRef::Unsigned(0)),
        ("17", IntegerRef::Unsigned(23)),
        ("1818", IntegerRef::Unsigned(24)),
        ("190100", IntegerRef::Unsigned(256)),
        ("1a00010000", IntegerRef::Unsigned(65_536)),
        ("1b0000000100000000", IntegerRef::Unsigned(1_u64 << 32)),
        ("1bffffffffffffffff", IntegerRef::Unsigned(u64::MAX)),
        ("20", IntegerRef::Negative(0)),
        ("3bffffffffffffffff", IntegerRef::Negative(u64::MAX)),
    ] {
        let bytes = unhex(hex)?;
        let mut decoder = Decoder::new(&bytes, limits()?)?;
        assert_eq!(decoder.integer()?, expected);
        assert_eq!(decoder.finish()?.as_bytes(), bytes);
    }
    assert_eq!(
        IntegerRef::Negative(u64::MAX).as_native_i128(),
        Some(-(1_i128 << 64))
    );
    assert_eq!(IntegerRef::Unsigned(u64::MAX).as_u64(), Some(u64::MAX));
    assert_eq!(IntegerRef::Negative(0).as_u64(), None);
    for (tag, negative) in [(0xc2, false), (0xc3, true)] {
        for length in [9_usize, 16, 33] {
            let mut bytes = vec![tag];
            if length < 24 {
                bytes.push(0x40 + u8::try_from(length)?);
            } else {
                bytes.extend_from_slice(&[0x58, u8::try_from(length)?]);
            }
            let start = bytes.len();
            bytes.push(1);
            bytes.resize(start + length, 0);
            let mut decoder = Decoder::new(&bytes, limits()?)?;
            let integer = decoder.integer()?;
            let magnitude = match integer {
                IntegerRef::PositiveBig(value) if !negative => value,
                IntegerRef::NegativeBig(value) if negative => value,
                _ => return Err("incorrect bignum sign or representation".into()),
            };
            assert_eq!(magnitude, &bytes[start..]);
            assert_eq!(integer.as_native_i128(), None);
            assert_eq!(decoder.finish()?.as_bytes(), bytes);
        }
    }
    Ok(())
}

#[test]
fn tokens_retain_unicode_key_order_and_absolute_spans() -> TestResult {
    let bytes = unhex("a262c3a9806365cc81630d0a00")?;
    let mut decoder = Decoder::new(&bytes, limits()?)?;
    let expected = [
        (TokenValue::Map(2), 0, 1),
        (TokenValue::Key("é"), 1, 4),
        (TokenValue::Array(0), 4, 5),
        (TokenValue::EndArray, 4, 5),
        (TokenValue::Key("e\u{301}"), 5, 9),
        (TokenValue::Text("\r\n\0"), 9, 13),
        (TokenValue::EndMap, 0, 13),
    ];
    for (value, start, end) in expected {
        let token = decoder.next_token()?.ok_or("missing token")?;
        assert_eq!(token.value, value);
        assert_eq!(token.offset, start);
        assert_eq!(token.bytes, &bytes[start..end]);
    }
    assert_eq!(decoder.next_token()?, None);
    assert_eq!(decoder.finish()?.as_bytes(), bytes);
    Ok(())
}
