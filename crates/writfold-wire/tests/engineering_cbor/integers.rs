use super::{
    Budget, Integer, Limits, RawCbor, TestResult, Value, assert_limit, encode, golden, unhex,
};

#[test]
fn native_and_unbounded_integer_normalization() -> TestResult {
    let vectors = [
        ("0", "00"),
        ("1", "01"),
        ("-1", "20"),
        ("-24", "37"),
        ("-25", "3818"),
        ("255", "18ff"),
        ("256", "190100"),
        ("65535", "19ffff"),
        ("65536", "1a00010000"),
        ("4294967295", "1affffffff"),
        ("4294967296", "1b0000000100000000"),
        ("9223372036854775807", "1b7fffffffffffffff"),
        ("-9223372036854775808", "3b7fffffffffffffff"),
        ("18446744073709551615", "1bffffffffffffffff"),
        ("-18446744073709551616", "3bffffffffffffffff"),
        ("18446744073709551616", "c249010000000000000000"),
        ("-18446744073709551617", "c349010000000000000000"),
        ("-18446744073709551618", "c349010000000000000001"),
        ("-18446744078004518912", "c3490100000000ffffffff"),
        ("-4722366482869645213696", "c349ffffffffffffffffff"),
        ("4722366482869645213696", "c24a01000000000000000000"),
        (
            "-340282366920938463463374607431768211456",
            "c350ffffffffffffffffffffffffffffffff",
        ),
        (
            "340282366920938463463374607431768211456",
            "c2510100000000000000000000000000000000",
        ),
        (
            "-340282366920938463463374607431768211457",
            "c3510100000000000000000000000000000000",
        ),
    ];
    for (decimal, hex) in vectors {
        golden(&Value::Integer(&decimal.parse::<Integer>()?), hex)?;
    }
    golden(&Value::Signed(i64::MIN), "3b7fffffffffffffff")?;
    golden(&Value::Signed(i64::MAX), "1b7fffffffffffffff")?;
    golden(&Value::Signed(24), "1818")?;
    Ok(())
}

#[test]
fn streamed_bignums_cross_payload_header_boundaries() -> TestResult {
    for (length, header) in [
        (9, "49"),
        (23, "57"),
        (24, "5818"),
        (255, "58ff"),
        (256, "590100"),
    ] {
        let positive = Integer::from_bigint(Integer::from(2_u8).as_bigint().pow((length - 1) * 8));
        let negative = Integer::from_bigint(-positive.as_bigint() - 1_u8);
        for (integer, tag) in [(&positive, 0xc2), (&negative, 0xc3)] {
            let mut expected = vec![tag];
            expected.extend(unhex(header)?);
            expected.push(1);
            expected.resize(expected.len() + length as usize - 1, 0);
            let exact = Limits::new(expected.len(), 0, 0, 1)?;
            assert_eq!(
                encode(&Value::Integer(integer), exact)?.as_bytes(),
                expected
            );
            assert!(RawCbor::parse(&expected, exact).is_ok());
            assert_limit(
                encode(
                    &Value::Integer(integer),
                    Limits::new(expected.len() - 1, 0, 0, 1)?,
                ),
                Budget::Bytes,
            );
        }
    }
    Ok(())
}
