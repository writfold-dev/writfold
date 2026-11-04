use std::fmt::Display;
use writfold_domain::engineering::{
    AttemptInputDigest, DependencyLockDigest, EngineeringDigest, LearningCandidateDigest,
    ProcedureRevision, RepoVectorDigest, SkillRevision,
};
use writfold_wire::{
    Limits,
    engineering::{
        cbor::{self, Value},
        fingerprint,
    },
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn limits() -> Result<Limits, writfold_wire::Error> {
    Limits::new(8192, 8, 64, 128)
}

fn hex_bytes(text: &str) -> TestResult<Vec<u8>> {
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| -> TestResult<u8> {
            let high = char::from(pair[0])
                .to_digit(16)
                .ok_or("invalid fixture hex")? as u8;
            let low = char::from(pair[1])
                .to_digit(16)
                .ok_or("invalid fixture hex")? as u8;
            Ok((high << 4) | low)
        })
        .collect()
}

fn golden<D: EngineeringDigest + Display>(
    row: &serde_json::Value,
    value: &Value<'_>,
) -> TestResult {
    let encoded = cbor::encode(value, limits()?)?;
    assert_eq!(
        encoded.as_bytes(),
        hex_bytes(
            row["canonical_hex"]
                .as_str()
                .ok_or("missing canonical bytes")?
        )?
    );
    assert_eq!(
        fingerprint::<D>(&encoded).to_string(),
        row["fingerprint"].as_str().ok_or("missing fingerprint")?
    );
    Ok(())
}

fn dependency_lock_golden(rows: &serde_json::Value) -> TestResult {
    golden::<DependencyLockDigest>(
        &rows[0],
        &Value::Map(&[
            ("schema_version", Value::Unsigned(1)),
            (
                "packages",
                Value::Array(&[
                    Value::Map(&[
                        ("id", Value::Text("demo.a")),
                        ("payload", Value::Text("synthetic-payload-a")),
                    ]),
                    Value::Map(&[
                        ("id", Value::Text("demo.b")),
                        ("payload", Value::Text("synthetic-payload-b")),
                    ]),
                ]),
            ),
            (
                "edges",
                Value::Array(&[
                    Value::Array(&[
                        Value::Text("demo.a"),
                        Value::Text("read"),
                        Value::Text("demo.b"),
                    ]),
                    Value::Array(&[
                        Value::Text("demo.b"),
                        Value::Text("read"),
                        Value::Text("demo.a"),
                    ]),
                ]),
            ),
        ]),
    )?;
    Ok(())
}

#[test]
fn frozen_handover_payloads_keep_exact_bytes_and_domains() -> TestResult {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../handovers/revision-5/writfold/examples/engineering/fingerprints.json"
    ))?;
    dependency_lock_golden(&rows)?;
    golden::<RepoVectorDigest>(
        &rows[1],
        &Value::Map(&[
            ("schema_version", Value::Unsigned(1)),
            (
                "repositories",
                Value::Array(&[Value::Map(&[
                    ("repo", Value::Text("demo.api")),
                    ("base", Value::Text("synthetic-base")),
                    ("target", Value::Text("synthetic-target")),
                    ("content", Value::Text("synthetic-content")),
                ])]),
            ),
        ]),
    )?;
    golden::<AttemptInputDigest>(
        &rows[2],
        &Value::Map(&[
            ("schema_version", Value::Unsigned(1)),
            ("procedure", Value::Text("synthetic-procedure-1")),
            ("repository_vector", Value::Text("synthetic-vector-1")),
            ("gate", Value::Text("compatibility")),
            ("environment", Value::Text("synthetic-env-1")),
        ]),
    )?;
    golden::<LearningCandidateDigest>(
        &rows[3],
        &Value::Map(&[
            ("schema_version", Value::Unsigned(1)),
            ("scope", Value::Text("API response extensions")),
            ("delta", Value::Text("check strict old consumers")),
            (
                "evidence",
                Value::Array(&[Value::Text("synthetic-failure-1")]),
            ),
            (
                "evaluation_protocol",
                Value::Text("synthetic-frozen-eval-1"),
            ),
        ]),
    )?;
    Ok(())
}

#[test]
fn matching_payload_bytes_do_not_allow_domain_substitution() -> TestResult {
    let payload = cbor::encode(
        &Value::Map(&[
            ("schema_version", Value::Unsigned(1)),
            ("name", Value::Text("demo")),
        ]),
        limits()?,
    )?;
    let skill = fingerprint::<SkillRevision>(&payload);
    let procedure = fingerprint::<ProcedureRevision>(&payload);
    assert_ne!(skill.as_digest(), procedure.as_digest());
    assert!(procedure.to_string().parse::<SkillRevision>().is_err());
    assert!(skill.to_string().parse::<ProcedureRevision>().is_err());
    Ok(())
}

#[test]
fn map_order_is_canonical_but_array_order_and_exact_text_remain_material() -> TestResult {
    let first = cbor::encode(
        &Value::Map(&[
            ("z", Value::Text("é\n")),
            (
                "aa",
                Value::Array(&[Value::Unsigned(1), Value::Unsigned(2)]),
            ),
        ]),
        limits()?,
    )?;
    let reordered = cbor::encode(
        &Value::Map(&[
            (
                "aa",
                Value::Array(&[Value::Unsigned(1), Value::Unsigned(2)]),
            ),
            ("z", Value::Text("é\n")),
        ]),
        limits()?,
    )?;
    let different_order = cbor::encode(
        &Value::Map(&[
            ("z", Value::Text("é\n")),
            (
                "aa",
                Value::Array(&[Value::Unsigned(2), Value::Unsigned(1)]),
            ),
        ]),
        limits()?,
    )?;
    let decomposed = cbor::encode(
        &Value::Map(&[
            ("z", Value::Text("e\u{301}\n")),
            (
                "aa",
                Value::Array(&[Value::Unsigned(1), Value::Unsigned(2)]),
            ),
        ]),
        limits()?,
    )?;
    let different_lines = cbor::encode(
        &Value::Map(&[
            ("z", Value::Text("é\r\n")),
            (
                "aa",
                Value::Array(&[Value::Unsigned(1), Value::Unsigned(2)]),
            ),
        ]),
        limits()?,
    )?;
    let expected = fingerprint::<SkillRevision>(&first);
    assert_eq!(expected, fingerprint::<SkillRevision>(&reordered));
    assert_ne!(expected, fingerprint::<SkillRevision>(&different_order));
    assert_ne!(expected, fingerprint::<SkillRevision>(&decomposed));
    assert_ne!(expected, fingerprint::<SkillRevision>(&different_lines));
    Ok(())
}
