use super::{Budget, Error, Limits, MAX_SUPPORTED_DEPTH, RawCbor, TestResult, limits, unhex};
use writfold_wire::engineering::cbor::{DecodeError, Decoder, Kind};

fn consume(bytes: &[u8], limits: Limits) -> Result<(), DecodeError> {
    let mut decoder = Decoder::new(bytes, limits)?;
    while decoder.next_token()?.is_some() {}
    decoder.finish()?;
    Ok(())
}

#[test]
fn every_truncated_prefix_and_noncanonical_form_uses_the_existing_errors() -> TestResult {
    for hex in [
        "1bffffffffffffffff",
        "c249010000000000000000",
        "c3510100000000000000000000000000000000",
        "62c3a9",
        "a261628201616162616102",
        "830080a0",
    ] {
        let bytes = unhex(hex)?;
        consume(&bytes, limits()?)?;
        for end in 0..bytes.len() {
            let prefix = &bytes[..end];
            let error = RawCbor::parse(prefix, limits()?)
                .err()
                .ok_or("accepted truncation")?;
            assert_eq!(consume(prefix, limits()?), Err(DecodeError::Cbor(error)));
        }
    }
    for hex in [
        "1800",
        "5800",
        "9f00ff",
        "a10100",
        "a2616200616100",
        "a2616100616101",
        "c24101",
        "c249000000000000000001",
        "c200",
        "c000",
        "f7",
        "f90000",
        "61ff",
        "0000",
        "81",
        "a16161",
    ] {
        let bytes = unhex(hex)?;
        let error = RawCbor::parse(&bytes, limits()?)
            .err()
            .ok_or("accepted malformed input")?;
        assert_eq!(
            consume(&bytes, limits()?),
            Err(DecodeError::Cbor(error)),
            "{hex}"
        );
    }
    Ok(())
}

#[test]
fn wrong_types_poison_the_cursor() -> TestResult {
    let mut decoder = Decoder::new(&[0x81, 0xf6], limits()?)?;
    assert_eq!(decoder.array()?, 1);
    assert_eq!(
        decoder.text(),
        Err(DecodeError::Type {
            offset: 1,
            expected: "text",
            actual: Kind::Null,
        })
    );
    assert_eq!(decoder.next_token(), Err(DecodeError::Failed));
    assert!(matches!(decoder.finish(), Err(DecodeError::Failed)));
    Ok(())
}

#[test]
fn container_endings_and_unread_fields_are_required() -> TestResult {
    let mut decoder = Decoder::new(&[0x80], limits()?)?;
    assert_eq!(decoder.array()?, 0);
    assert!(matches!(
        decoder.finish(),
        Err(DecodeError::Unconsumed { offset: 0 })
    ));
    let mut decoder = Decoder::new(&[0x81, 0], limits()?)?;
    assert_eq!(decoder.array()?, 1);
    assert!(matches!(
        decoder.end_array(),
        Err(DecodeError::Type {
            actual: Kind::Integer,
            ..
        })
    ));
    let mut decoder = Decoder::new(&[0xa0], limits()?)?;
    assert_eq!(decoder.map()?, 0);
    assert!(matches!(
        decoder.end_array(),
        Err(DecodeError::Type {
            actual: Kind::EndMap,
            ..
        })
    ));
    Ok(())
}

#[test]
fn keys_integer_signs_and_end_of_input_have_distinct_types() -> TestResult {
    let mut decoder = Decoder::new(&[0xa1, 0x61, b'a', 0], limits()?)?;
    assert_eq!(decoder.map()?, 1);
    assert!(matches!(
        decoder.text(),
        Err(DecodeError::Type {
            actual: Kind::Key,
            ..
        })
    ));
    let mut decoder = Decoder::new(&[0x20], limits()?)?;
    assert!(matches!(
        decoder.unsigned(),
        Err(DecodeError::Type {
            actual: Kind::Integer,
            ..
        })
    ));
    let mut decoder = Decoder::new(&[0], limits()?)?;
    assert_eq!(decoder.unsigned()?, 0);
    assert!(matches!(
        decoder.null(),
        Err(DecodeError::EndOfInput { offset: 1, .. })
    ));
    assert!(matches!(decoder.finish(), Err(DecodeError::Failed)));
    Ok(())
}

#[test]
fn syntax_errors_poison_the_cursor_and_extra_bytes_are_not_silently_skipped() -> TestResult {
    let mut decoder = Decoder::new(&[0x82, 0, 0x18], limits()?)?;
    assert_eq!(decoder.array()?, 2);
    assert_eq!(decoder.unsigned()?, 0);
    assert!(matches!(
        decoder.unsigned(),
        Err(DecodeError::Cbor(Error::Syntax { offset: 3, .. }))
    ));
    assert!(matches!(decoder.finish(), Err(DecodeError::Failed)));
    let mut decoder = Decoder::new(&[0, 1], limits()?)?;
    assert_eq!(decoder.unsigned()?, 0);
    assert!(matches!(
        decoder.finish(),
        Err(DecodeError::Cbor(Error::TrailingInput { offset: 1 }))
    ));
    Ok(())
}

#[test]
fn streaming_budgets_match_the_canonical_parser_including_maps_and_bignums() -> TestResult {
    let bytes = [0x82, 0, 0];
    consume(&bytes, Limits::new(3, 1, 2, 3)?)?;
    for (limits, budget) in [
        (Limits::new(2, 1, 2, 3)?, Budget::Bytes),
        (Limits::new(3, 0, 2, 3)?, Budget::Depth),
        (Limits::new(3, 1, 1, 3)?, Budget::ContainerEntries),
        (Limits::new(3, 1, 2, 2)?, Budget::Nodes),
    ] {
        assert!(
            matches!(consume(&bytes, limits), Err(DecodeError::Cbor(Error::Limit { budget: actual, .. })) if actual == budget)
        );
    }
    // Map keys, bignum envelopes, and closing tokens do not add value nodes.
    consume(&unhex("a1636b657900")?, Limits::new(6, 1, 1, 2)?)?;
    consume(&unhex("c249010000000000000000")?, Limits::new(11, 0, 0, 1)?)?;
    Ok(())
}

#[test]
fn empty_and_deep_containers_enforce_depth_without_recursion() -> TestResult {
    consume(&[0x80], Limits::new(1, 1, 0, 1)?)?;
    assert!(matches!(
        consume(&[0x80], Limits::new(1, 0, 0, 1)?),
        Err(DecodeError::Cbor(Error::Limit {
            budget: Budget::Depth,
            ..
        }))
    ));
    let mut nested = vec![0x81; MAX_SUPPORTED_DEPTH];
    nested.push(0);
    consume(
        &nested,
        Limits::new(nested.len(), MAX_SUPPORTED_DEPTH, 1, nested.len())?,
    )?;
    assert!(matches!(
        consume(
            &nested,
            Limits::new(nested.len(), MAX_SUPPORTED_DEPTH - 1, 1, nested.len())?
        ),
        Err(DecodeError::Cbor(Error::Limit {
            budget: Budget::Depth,
            ..
        }))
    ));
    Ok(())
}
