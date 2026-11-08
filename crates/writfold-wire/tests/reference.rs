use serde_json::{Value, json};
use writfold_wire::reference::{evaluate_json, evaluate_request};
use writfold_wire::semantic::{dec_formula, dec_ttl, enc_formula};

#[test]
fn retained_source_examples_match_complete_responses() -> Result<(), Box<dyn std::error::Error>> {
    for source in [
        include_str!("fixtures/conformance/cases.jsonl"),
        include_str!("fixtures/conformance/workflow-v3.jsonl"),
        include_str!("fixtures/conformance/workflow-v3-boundaries.jsonl"),
    ] {
        for line in source.lines().filter(|line| !line.trim().is_empty()) {
            let case: Value = serde_json::from_str(line)?;
            let mut request = case.as_object().ok_or("fixture is not an object")?.clone();
            let expected = match case.get("expected_response") {
                Some(response) => response.clone(),
                None => {
                    json!({"id": case["id"], "ok": true, "result": case.get("expected").ok_or("fixture lacks expected result")?})
                }
            };
            let actual = if let Some(raw) = case.get("raw_request") {
                evaluate_json(
                    raw.as_str()
                        .ok_or("raw request is not a string")?
                        .as_bytes(),
                )
            } else {
                for field in [
                    "expected",
                    "expected_response",
                    "category",
                    "trusted_environment",
                    "raw_request",
                ] {
                    request.remove(field);
                }
                let transport = match case.get("trusted_environment") {
                    Some(environment) => {
                        json!({"request": request, "trusted_environment": environment})
                    }
                    None => Value::Object(request),
                };
                evaluate_json(&serde_json::to_vec(&transport)?)
            };
            assert_eq!(actual, expected, "fixture {}", case["id"]);
        }
    }
    Ok(())
}

#[test]
fn malformed_versions_types_and_safe_ids() {
    let cases = [
        (br#"{"id":"safe","schema_version":true,"semantic_version":"2.0.0","op":"semantic_version"}"#.as_slice(), "safe"),
        (br#"{"id":7,"schema_version":2,"semantic_version":"2.0.0","op":"semantic_version"}"#.as_slice(), ""),
        (br#"{"id":"safe","schema_version":2,"semantic_version":"2.1.0","op":"semantic_version"}"#.as_slice(), "safe"),
        (br#"{"id":"safe","id":"other"}"#.as_slice(), ""),
        (br#"{"id":"\ud800"}"#.as_slice(), ""),
        (br#"[]"#.as_slice(), ""),
        (br#"{"id":"safe","schema_version":2,"semantic_version":"2.0.0","op":"semantic_version","trusted_environment":{}}"#.as_slice(), "safe"),
        (br#"{"schema_version":2,"semantic_version":"2.0.0","id":"fraction","op":"semantic_version","unexpected":1.5}"#.as_slice(), "fraction"),
    ];
    for (raw, id) in cases {
        assert_eq!(
            evaluate_json(raw),
            json!({"id": id, "ok": false, "error": "invalid_request"})
        );
    }
}

#[test]
fn arbitrary_integer_freshness_and_boolean_rejection() {
    assert_eq!(evaluate_json(br#"{"id":"large","schema_version":2,"semantic_version":"2.0.0","op":"fresh","now":100000000000000000000000000000001,"observed_at":100000000000000000000000000000000,"ttl":1}"#), json!({"id":"large","ok":true,"result":"fresh"}));
    assert!(dec_ttl(&json!(true)).is_err());
    assert!(dec_ttl(&json!(-1)).is_err());
    assert_eq!(
        evaluate_request(
            &json!({"schema_version":2,"semantic_version":"2.0.0","id":"n","op":"fresh","now":false,"observed_at":0,"ttl":0})
        ),
        json!({"id":"n","ok":false,"error":"invalid_request"})
    );
}

#[test]
fn formula_round_trip_preserves_argument_order_and_rejects_extra_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let formula = json!({"tag":"atom","atom":{"subject":"s","predicate":"p","args":["z","a","z"]}});
    assert_eq!(enc_formula(&dec_formula(&formula)?), formula);
    assert!(dec_formula(&json!({"tag":"truth","atom":{}})).is_err());
    assert!(dec_formula(&json!({"tag":"invented"})).is_err());
    Ok(())
}

#[test]
fn signed_integer_codec_does_not_narrow_to_machine_ranges() -> Result<(), Box<dyn std::error::Error>>
{
    let bytes = b"-999999999999999999999999999999999999999999999999999999";
    let limits = writfold_wire::Limits::new(1024, 8, 8, 8)?;
    let raw = writfold_wire::RawJson::parse(bytes, limits)?;
    let integer = writfold_wire::semantic::integer(raw.value())?;
    assert!(integer.is_negative());
    assert_eq!(writfold_wire::semantic::enc_integer(&integer), *raw.value());
    assert!(writfold_wire::semantic::nat(raw.value()).is_err());
    Ok(())
}

#[test]
fn malformed_environment_has_empty_id_but_request_errors_keep_safe_id() {
    let request =
        json!({"schema_version":2,"semantic_version":"2.0.0","id":"fixture","op":"unknown"});
    let valid_environment = json!({"approvals":[],"selections":[],"retained":[]});
    let malformed = json!({"request":request,"trusted_environment":{"approvals":true,"selections":[],"retained":[]}});
    assert_eq!(
        writfold_wire::reference::evaluate_transport(&malformed),
        json!({"id":"","ok":false,"error":"invalid_request"})
    );
    assert_eq!(
        writfold_wire::reference::evaluate_transport(
            &json!({"request":request,"trusted_environment":valid_environment})
        ),
        json!({"id":"fixture","ok":false,"error":"invalid_request"})
    );
}

#[test]
fn executable_stream_recovers_after_a_malformed_line() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_writfold-ref"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("missing child stdin")?;
    input.write_all(b"\nnot-json\n{\"schema_version\":2,\"semantic_version\":\"2.0.0\",\"id\":\"next\",\"op\":\"semantic_version\"}\n")?;
    drop(input);
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    let responses = std::str::from_utf8(&output.stdout)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        responses,
        vec![
            json!({"id":"","ok":false,"error":"invalid_request"}),
            json!({"id":"next","ok":true,"result":"2.0.0"})
        ]
    );
    Ok(())
}

#[test]
fn full_request_admits_baseline_formula_depth_and_rejects_next_level()
-> Result<(), Box<dyn std::error::Error>> {
    let mut formula = json!({"tag":"truth"});
    for _ in 1..256 {
        formula = json!({"tag":"and","left":{"tag":"truth"},"right":formula});
    }
    let scope = json!({"project":"project-a","selector":""});
    let request = json!({
        "schema_version":2,"semantic_version":"2.0.0","id":"deep","op":"validate",
        "kb":{"snapshot":"s1","observations":[],"claims":[{
            "id":"claim","revision":"r1","statement":{"formula":formula,"scope":scope,"polarity":"positive"}
        }],"attestations":[],"enumerations":[],"derivations":[],"corrections":[],"receipts":[]}
    });
    assert_eq!(
        evaluate_json(&serde_json::to_vec(&request)?),
        json!({"id":"deep","ok":true,"result":true})
    );
    let accepted = &request["kb"]["claims"][0]["statement"]["formula"];
    assert!(dec_formula(accepted).is_ok());
    assert!(dec_formula(&json!({"tag":"and","left":{"tag":"truth"},"right":accepted})).is_err());
    Ok(())
}

#[test]
fn fractional_and_exponent_tokens_are_not_semantic_integers()
-> Result<(), Box<dyn std::error::Error>> {
    let limits = writfold_wire::Limits::new(1024, 8, 8, 8)?;
    for token in [b"1.0".as_slice(), b"1e0".as_slice()] {
        let raw = writfold_wire::RawJson::parse(token, limits)?;
        assert!(writfold_wire::semantic::integer(raw.value()).is_err());
        assert!(writfold_wire::semantic::nat(raw.value()).is_err());
    }
    Ok(())
}
