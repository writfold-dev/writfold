use std::cell::OnceCell;
use writfold_domain::Integer;
use writfold_wire::engineering::cbor::{Error, RawCbor, Value, encode};
use writfold_wire::{Budget, Limits, MAX_SUPPORTED_DEPTH};

#[path = "engineering_cbor/decode.rs"]
mod decode;
#[path = "engineering_cbor/decode_errors.rs"]
mod decode_errors;

#[path = "engineering_cbor/integers.rs"]
mod integers;

type TestResult<T = (), E = Box<dyn std::error::Error + Send + Sync>> = Result<T, E>;

fn limits() -> Result<Limits, writfold_wire::Error> {
    Limits::new(1_000_000, MAX_SUPPORTED_DEPTH, 10_000, 20_000)
}

fn unhex(text: &str) -> TestResult<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return Err("fixture hex must contain complete bytes".into());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}

fn golden(value: &Value<'_>, hex: &str) -> TestResult {
    let expected = unhex(hex)?;
    assert_eq!(encode(value, limits()?)?.as_bytes(), expected);
    let parsed = RawCbor::parse(&expected, limits()?)?;
    assert_eq!(parsed.into_bytes(), expected);
    Ok(())
}

#[test]
fn exact_eighteen_hand_encoded_planning_vectors() -> TestResult {
    // Independently transcribed from immutable tools/check_engineering_examples.py.
    let native_negative = Integer::from(-18_446_744_073_709_551_616_i128);
    let positive_big = Integer::from(18_446_744_073_709_551_616_u128);
    let negative_big = Integer::from(-18_446_744_073_709_551_617_i128);
    let array = [Value::Unsigned(1), Value::Text("a")];
    let map = [("aa", Value::Unsigned(2)), ("b", Value::Unsigned(1))];
    let vectors = [
        (Value::Null, "f6"),
        (Value::Bool(false), "f4"),
        (Value::Bool(true), "f5"),
        (Value::Unsigned(0), "00"),
        (Value::Unsigned(23), "17"),
        (Value::Unsigned(24), "1818"),
        (Value::Unsigned(256), "190100"),
        (Value::Signed(-1), "20"),
        (Value::Signed(-25), "3818"),
        (Value::Unsigned(u64::MAX), "1bffffffffffffffff"),
        (Value::Integer(&native_negative), "3bffffffffffffffff"),
        (Value::Integer(&positive_big), "c249010000000000000000"),
        (Value::Integer(&negative_big), "c349010000000000000000"),
        (Value::Bytes(b"ab"), "426162"),
        (Value::Text("a"), "6161"),
        (Value::Text("é"), "62c3a9"),
        (Value::Array(&array), "82016161"),
        (Value::Map(&map), "a261620162616102"),
    ];
    for (value, hex) in vectors {
        golden(&value, hex)?;
    }
    Ok(())
}

#[test]
fn refuses_unsupported_noncanonical_and_malformed_input() -> TestResult {
    let rejected = [
        "",
        "18",
        "1901",
        "1a000001",
        "1b0000000000000001",
        "1817",
        "1900ff",
        "1a0000ffff",
        "1b00000000ffffffff",
        "3800",
        "5800",
        "7800",
        "9800",
        "b800",
        "d802",
        "1c",
        "1d",
        "1e",
        "1f",
        "5f40ff",
        "7f60ff",
        "9fff",
        "bfff",
        "ff",
        "f0",
        "f7",
        "f818",
        "f820",
        "f90000",
        "fa00000000",
        "fb0000000000000000",
        "f97e00",
        "f97c00",
        "f9fc00",
        "c000",
        "c100",
        "c400",
        "d86400",
        "c200",
        "c380",
        "c240",
        "c340",
        "c24101",
        "c348ffffffffffffffff",
        "c249000100000000000000",
        "c34900ffffffffffffffff",
        "c2490100000000000000",
        "c2590009010000000000000000",
        "42ff",
        "62c3",
        "61ff",
        "62c080",
        "63eda080",
        "64f4908080",
        "a10000",
        "a1416100",
        "a16161",
        "a2616100616101",
        "a2616200616101",
        "a262616100616201",
        "8200",
        "a160",
        "5bffffffffffffffff",
        "7bffffffffffffffff",
        "9bffffffffffffffff",
        "bbffffffffffffffff",
        "00f6",
        "f600",
        "8080",
    ];
    for hex in rejected {
        assert!(
            RawCbor::parse(&unhex(hex)?, limits()?).is_err(),
            "accepted {hex}"
        );
    }
    Ok(())
}

#[test]
fn all_truncated_prefixes_of_valid_items_are_rejected() -> TestResult {
    for hex in [
        "1bffffffffffffffff",
        "c249010000000000000000",
        "c3510100000000000000000000000000000000",
        "62c3a9",
        "a261628201616162616102",
        "830080a0",
    ] {
        let bytes = unhex(hex)?;
        assert!(RawCbor::parse(&bytes, limits()?).is_ok());
        for end in 0..bytes.len() {
            assert!(
                RawCbor::parse(&bytes[..end], limits()?).is_err(),
                "{hex} at {end}"
            );
        }
    }
    Ok(())
}

#[test]
fn errors_expose_bounded_categories_and_offsets() -> TestResult {
    assert_eq!(
        RawCbor::parse(&[0, 0], limits()?)
            .err()
            .ok_or("expected trailing-input error")?,
        Error::TrailingInput { offset: 1 }
    );
    assert_eq!(
        RawCbor::parse(&[0x61, 0xff], limits()?)
            .err()
            .ok_or("expected Unicode error")?,
        Error::Unicode { offset: 1 }
    );
    assert_eq!(
        RawCbor::parse(&unhex("a2616100616101")?, limits()?)
            .err()
            .ok_or("expected duplicate-key error")?,
        Error::DuplicateKey { offset: 4 }
    );
    assert!(matches!(
        RawCbor::parse(&[0x18, 0], limits()?),
        Err(Error::NonCanonical { offset: 0, .. })
    ));
    assert!(matches!(
        RawCbor::parse(&[0x82, 0], limits()?),
        Err(Error::Syntax { offset: 2, .. })
    ));
    Ok(())
}

fn assert_limit(result: Result<RawCbor<'_>, Error>, expected: Budget) {
    assert!(matches!(result, Err(Error::Limit { budget, .. }) if budget == expected));
}

#[test]
fn exact_byte_entry_node_and_depth_limits() -> TestResult {
    let values = [Value::Unsigned(0), Value::Unsigned(1)];
    let array = Value::Array(&values);
    let bytes = [0x82, 0, 1];
    let exact = Limits::new(3, 1, 2, 3)?;
    assert_eq!(encode(&array, exact)?.as_bytes(), bytes);
    assert!(RawCbor::parse(&bytes, exact).is_ok());
    for (bounded, budget) in [
        (Limits::new(2, 1, 2, 3)?, Budget::Bytes),
        (Limits::new(3, 0, 2, 3)?, Budget::Depth),
        (Limits::new(3, 1, 1, 3)?, Budget::ContainerEntries),
        (Limits::new(3, 1, 2, 2)?, Budget::Nodes),
    ] {
        assert_limit(encode(&array, bounded), budget);
        assert_limit(RawCbor::parse(&bytes, bounded), budget);
    }
    for value in [Value::Null, Value::Text(""), Value::Bytes(b"")] {
        assert!(encode(&value, Limits::new(1, 0, 0, 1)?).is_ok());
        assert_limit(encode(&value, Limits::new(1, 0, 0, 0)?), Budget::Nodes);
        assert_limit(encode(&value, Limits::new(0, 0, 0, 1)?), Budget::Bytes);
    }
    for byte in [0xf6, 0x60, 0x40] {
        assert!(RawCbor::parse(&[byte], Limits::new(1, 0, 0, 1)?).is_ok());
        assert_limit(
            RawCbor::parse(&[byte], Limits::new(1, 0, 0, 0)?),
            Budget::Nodes,
        );
    }
    for (value, byte) in [(Value::Array(&[]), 0x80), (Value::Map(&[]), 0xa0)] {
        assert!(encode(&value, Limits::new(1, 1, 0, 1)?).is_ok());
        assert_limit(encode(&value, Limits::new(1, 0, 0, 1)?), Budget::Depth);
        assert_limit(
            RawCbor::parse(&[byte], Limits::new(1, 0, 0, 1)?),
            Budget::Depth,
        );
    }
    Ok(())
}

#[test]
fn map_keys_and_bignum_payloads_are_not_value_nodes() -> TestResult {
    let entries = [("key", Value::Unsigned(0))];
    let bounded = Limits::new(6, 1, 1, 2)?;
    assert_eq!(
        encode(&Value::Map(&entries), bounded)?.as_bytes(),
        unhex("a1636b657900")?
    );
    assert!(RawCbor::parse(&unhex("a1636b657900")?, bounded).is_ok());
    let integer = Integer::from(1_u128 << 64);
    let bounded = Limits::new(11, 0, 0, 1)?;
    assert!(encode(&Value::Integer(&integer), bounded).is_ok());
    assert!(RawCbor::parse(&unhex("c249010000000000000000")?, bounded).is_ok());
    assert_limit(
        encode(&Value::Integer(&integer), Limits::new(10, 0, 0, 1)?),
        Budget::Bytes,
    );
    Ok(())
}

#[test]
fn encoded_key_order_duplicate_refusal_and_exact_text() -> TestResult {
    let entries = [
        ("aa", Value::Null),
        ("é", Value::Null),
        ("b", Value::Null),
        ("", Value::Null),
    ];
    golden(&Value::Map(&entries), "a460f66162f6626161f662c3a9f6")?;
    let duplicate = [
        ("a", Value::Null),
        ("b", Value::Null),
        ("a", Value::Bool(true)),
    ];
    assert!(matches!(
        encode(&Value::Map(&duplicate), limits()?),
        Err(Error::DuplicateKey { .. })
    ));
    let long = "a".repeat(24);
    let short = "z".repeat(23);
    let boundary = [(&*long, Value::Null), (&*short, Value::Null)];
    let encoded = encode(&Value::Map(&boundary), limits()?)?;
    assert_eq!(&encoded.as_bytes()[..2], &[0xa2, 0x77]);
    assert!(RawCbor::parse(encoded.as_bytes(), limits()?).is_ok());
    golden(&Value::Text("e\u{301}"), "6365cc81")?;
    golden(&Value::Text("é"), "62c3a9")?;
    golden(&Value::Text("\r\n\0"), "630d0a00")?;
    golden(&Value::Text("😀"), "64f09f9880")?;
    let distinct = [("é", Value::Null), ("e\u{301}", Value::Null)];
    golden(&Value::Map(&distinct), "a262c3a9f66365cc81f6")?;
    let ordered = [Value::Unsigned(2), Value::Unsigned(1), Value::Unsigned(2)];
    golden(&Value::Array(&ordered), "83020102")?;
    Ok(())
}

#[test]
fn deep_traversal_and_drop_use_no_native_recursion() -> TestResult {
    let handle = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| -> TestResult {
            // OnceCell gives a nonrecursive borrowed arena without leaked allocations.
            let arena: Vec<OnceCell<Value<'_>>> =
                (0..=MAX_SUPPORTED_DEPTH).map(|_| OnceCell::new()).collect();
            arena[0]
                .set(Value::Null)
                .map_err(|_| "arena root already initialized")?;
            for depth in 1..=MAX_SUPPORTED_DEPTH {
                arena[depth]
                    .set(Value::Array(std::slice::from_ref(
                        arena[depth - 1].get().ok_or("arena predecessor missing")?,
                    )))
                    .map_err(|_| "arena depth already initialized")?;
            }
            let value = arena[MAX_SUPPORTED_DEPTH]
                .get()
                .ok_or("arena root missing")?;
            let bounded = Limits::new(
                MAX_SUPPORTED_DEPTH + 1,
                MAX_SUPPORTED_DEPTH,
                1,
                MAX_SUPPORTED_DEPTH + 1,
            )?;
            let owned = encode(value, bounded)?;
            let mut expected = vec![0x81; MAX_SUPPORTED_DEPTH];
            expected.push(0xf6);
            assert_eq!(owned.as_bytes(), expected);
            let raw = RawCbor::parse(&expected, bounded)?;
            drop(raw);
            drop(owned);
            let shallower = Limits::new(
                MAX_SUPPORTED_DEPTH + 1,
                MAX_SUPPORTED_DEPTH - 1,
                1,
                MAX_SUPPORTED_DEPTH + 1,
            )?;
            assert_limit(encode(value, shallower), Budget::Depth);
            assert_limit(RawCbor::parse(&expected, shallower), Budget::Depth);
            expected.insert(0, 0x81);
            assert_limit(RawCbor::parse(&expected, limits()?), Budget::Depth);
            Ok(())
        })?;
    match handle.join() {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
