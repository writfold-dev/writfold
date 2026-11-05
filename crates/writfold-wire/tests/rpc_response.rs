use serde_json::{Value, json};
use writfold_domain::{ContentDigest, InstanceId, RequestGeneration, SessionIncarnation};
use writfold_wire::{
    Limits,
    rpc::{
        Frame, MAX_REQUEST_BYTES, RequestId,
        response::{
            Fault, MAX_RESPONSE_BYTES, METHOD_SCHEMAS, PROFILE, ResponseError, ServerMetadata,
            Success, encode_error, encode_success,
        },
    },
};

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

struct Metadata {
    instance: InstanceId,
    incarnation: SessionIncarnation,
    digest: ContentDigest,
}

impl Metadata {
    fn new() -> TestResult<Self> {
        Ok(Self {
            instance: "stdio-instance".parse()?,
            incarnation: "stdio-session".parse()?,
            digest: "a".repeat(64).parse()?,
        })
    }

    fn borrowed<'a>(&'a self, version: &'a str) -> ServerMetadata<'a> {
        ServerMetadata::new(version, &self.instance, &self.incarnation, &self.digest)
    }

    fn session(&self) -> Value {
        json!({
            "instance_id": self.instance.as_str(),
            "incarnation": self.incarnation.as_str(),
        })
    }
}

fn object(bytes: &[u8]) -> TestResult<Value> {
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
    assert!(bytes.len() <= MAX_RESPONSE_BYTES);
    let value: Value = serde_json::from_slice(bytes)?;
    assert!(value.is_object());
    Ok(value)
}

#[test]
fn replies_preserve_integer_string_and_null_id_kinds() -> TestResult {
    let metadata = Metadata::new()?;
    let generation = RequestGeneration::from_u64(1);
    for token in [
        "0".to_owned(),
        "-1".to_owned(),
        "9".repeat(5000),
        format!("-{}", "8".repeat(5000)),
        "\"123\"".to_owned(),
        "\"\"".to_owned(),
        "null".to_owned(),
        "-0".to_owned(),
    ] {
        let frame = Frame::parse(
            format!(r#"{{"jsonrpc":"2.0","method":"unknown","id":{token}}}"#).as_bytes(),
            Limits::new(MAX_REQUEST_BYTES, 8, 32, 64)?,
        )?;
        let request = frame.request().map_err(|error| format!("{error:?}"))?;
        let expected_token = if token == "-0" { "0" } else { &token };
        for response in [
            encode_success(
                request.id(),
                metadata.borrowed("1"),
                generation,
                Success::Initialize,
                MAX_RESPONSE_BYTES,
            )?,
            encode_success(
                request.id(),
                metadata.borrowed("1"),
                generation,
                Success::Shutdown,
                MAX_RESPONSE_BYTES,
            )?,
            encode_error(
                request.id(),
                metadata.borrowed("1"),
                Some(generation),
                Fault::MethodNotFound,
                MAX_RESPONSE_BYTES,
            )?,
        ] {
            let value = object(&response)?;
            assert!(
                response
                    .starts_with(format!(r#"{{"jsonrpc":"2.0","id":{expected_token},"#).as_bytes())
            );
            match request.id() {
                RequestId::Integer(integer) => {
                    let Value::Number(number) = &value["id"] else {
                        return Err("response integer ID changed kind".into());
                    };
                    assert_eq!(number.as_str(), integer.as_decimal());
                }
                RequestId::String(string) => assert_eq!(value["id"].as_str(), Some(string)),
                RequestId::Null => assert!(value["id"].is_null()),
                RequestId::Absent => return Err("fixture lost its ID".into()),
            }
        }
    }
    Ok(())
}

#[test]
fn strings_are_escaped_without_creating_extra_frames_or_fields() -> TestResult {
    let metadata = Metadata::new()?;
    let id = "\"},\"injected\":true,\"id\":\"\\\n\r\t\0\u{8}\u{c}雪\u{1d11e}";
    let version = "v\"\\\n\r\t\0é";
    let bytes = encode_success(
        RequestId::String(id),
        metadata.borrowed(version),
        RequestGeneration::from_u64(1),
        Success::Initialize,
        MAX_RESPONSE_BYTES,
    )?;
    let value = object(&bytes)?;
    assert_eq!(value["id"], id);
    assert_eq!(value["result"]["server"]["version"], version);
    assert_eq!(value.as_object().ok_or("response object")?.len(), 3);
    let bytes = encode_error(
        RequestId::String(id),
        metadata.borrowed(version),
        None,
        Fault::InvalidParams,
        MAX_RESPONSE_BYTES,
    )?;
    let value = object(&bytes)?;
    assert_eq!(value["id"], id);
    assert_eq!(value["error"]["data"]["message"], "Invalid params");
    assert!(!serde_json::to_string(&value["error"])?.contains("雪"));
    Ok(())
}

#[test]
fn initialize_advertises_only_closed_transport_control_contract() -> TestResult {
    let metadata = Metadata::new()?;
    let methods: Value = serde_json::from_str(METHOD_SCHEMAS)?;
    assert_eq!(
        methods,
        json!({
            "initialize": {
                "additionalProperties": false,
                "properties": {
                    "protocol_version": {"const": "2", "title": "Protocol Version", "type": "string"},
                    "capabilities": {"items": {"type": "string"}, "title": "Capabilities", "type": "array"},
                },
                "required": ["protocol_version"], "title": "Initialize", "type": "object",
            },
            "shutdown": {"additionalProperties": false, "properties": {}, "title": "Params", "type": "object"},
        })
    );
    let bytes = encode_success(
        RequestId::Null,
        metadata.borrowed("test-version"),
        RequestGeneration::from_u64(u64::MAX),
        Success::Initialize,
        MAX_RESPONSE_BYTES,
    )?;
    assert_eq!(
        object(&bytes)?,
        json!({
            "jsonrpc": "2.0", "id": null,
            "result": {
                "protocol_version": "2", "profile": PROFILE,
                "implementation_scope": "transport-control-only",
                "server": {"name": "writfold", "version": "test-version"},
                "session": metadata.session(), "generation": u64::MAX,
                "capabilities": ["notifications"], "batch": false, "notifications": true,
                "max_request_bytes": 65536, "max_response_bytes": 65536,
                "methods": methods,
                "contract": {"namespace": "writfold-agent-contract", "schema": 1, "sha256": "a".repeat(64)},
                "trust_boundary": "Transport/session control only; no authorized corpus, retained release, grounding or effect service is available.",
            },
        })
    );
    Ok(())
}

#[test]
fn shutdown_is_complete_and_retains_session_and_generation() -> TestResult {
    let metadata = Metadata::new()?;
    let bytes = encode_success(
        RequestId::String("shutdown"),
        metadata.borrowed("1"),
        RequestGeneration::from_u64(42),
        Success::Shutdown,
        MAX_RESPONSE_BYTES,
    )?;
    assert_eq!(
        object(&bytes)?,
        json!({
            "jsonrpc": "2.0", "id": "shutdown",
            "result": {
                "exit_code": 0, "complete": true, "generation": 42,
                "session": metadata.session(), "payload": {"shutdown": true},
            },
        })
    );
    Ok(())
}

#[test]
fn faults_preserve_classification_correlation_and_rejected_outcome() -> TestResult {
    let metadata = Metadata::new()?;
    for (fault, code, category) in [
        (Fault::Parse, -32700, "malformed"),
        (Fault::Oversized, -32600, "malformed"),
        (Fault::InvalidRequest, -32600, "malformed"),
        (Fault::InvalidParams, -32602, "malformed"),
        (Fault::MethodNotFound, -32601, "malformed"),
        (Fault::InitializationRequired, -32002, "conflict"),
        (Fault::AlreadyInitialized, -32000, "conflict"),
        (Fault::ShutDown, -32002, "conflict"),
        (Fault::UnsupportedCapability, -32000, "malformed"),
        (Fault::GenerationExhausted, -32000, "conflict"),
    ] {
        for generation in [None, Some(RequestGeneration::from_u64(u64::MAX))] {
            let encoded = encode_error(
                RequestId::Null,
                metadata.borrowed("1"),
                generation,
                fault,
                MAX_RESPONSE_BYTES,
            )?;
            let value = object(&encoded)?;
            assert_eq!(value["id"], Value::Null);
            assert!(value.get("result").is_none());
            assert_eq!(value["error"]["code"], code);
            let data = &value["error"]["data"];
            assert_eq!(data["category"], category);
            assert_eq!(data["complete"], false);
            assert_eq!(data["outcome"], "rejected");
            assert_eq!(data["exit_code"], 1);
            assert_eq!(
                data["generation"],
                json!(generation.map(RequestGeneration::get))
            );
            assert_eq!(data["session"], metadata.session());
        }
    }
    Ok(())
}

#[test]
fn absent_ids_refuse_even_before_output_budget_is_considered() -> TestResult {
    let metadata = Metadata::new()?;
    for maximum in [0, MAX_RESPONSE_BYTES, usize::MAX] {
        for success in [Success::Initialize, Success::Shutdown] {
            assert_eq!(
                encode_success(
                    RequestId::Absent,
                    metadata.borrowed("1"),
                    RequestGeneration::from_u64(1),
                    success,
                    maximum,
                ),
                Err(ResponseError::AbsentId)
            );
        }
        assert_eq!(
            encode_error(
                RequestId::Absent,
                metadata.borrowed("1"),
                None,
                Fault::InvalidRequest,
                maximum,
            ),
            Err(ResponseError::AbsentId)
        );
    }
    Ok(())
}

#[test]
fn exact_budgets_include_lf_and_never_return_partial_or_substitute_ids() -> TestResult {
    let metadata = Metadata::new()?;
    let generation = RequestGeneration::from_u64(1);
    let id = RequestId::String("quoted\"\\\n");
    for success in [Success::Initialize, Success::Shutdown] {
        let full = encode_success(
            id,
            metadata.borrowed("1"),
            generation,
            success,
            MAX_RESPONSE_BYTES,
        )?;
        assert_eq!(
            encode_success(id, metadata.borrowed("1"), generation, success, full.len())?,
            full
        );
        for maximum in [0, 1, full.len() - 1] {
            assert_eq!(
                encode_success(id, metadata.borrowed("1"), generation, success, maximum),
                Err(ResponseError::Limit)
            );
        }
    }
    let full = encode_error(
        id,
        metadata.borrowed("1"),
        None,
        Fault::Parse,
        MAX_RESPONSE_BYTES,
    )?;
    assert_eq!(
        encode_error(id, metadata.borrowed("1"), None, Fault::Parse, full.len())?,
        full
    );
    for maximum in [0, 1, full.len() - 1] {
        assert_eq!(
            encode_error(id, metadata.borrowed("1"), None, Fault::Parse, maximum),
            Err(ResponseError::Limit)
        );
    }
    Ok(())
}

#[test]
fn escaping_and_caller_limits_cannot_exceed_the_protocol_ceiling() -> TestResult {
    let metadata = Metadata::new()?;
    let generation = RequestGeneration::from_u64(1);
    let empty = encode_success(
        RequestId::String(""),
        metadata.borrowed("1"),
        generation,
        Success::Shutdown,
        MAX_RESPONSE_BYTES,
    )?;
    let exact_id = "x".repeat(MAX_RESPONSE_BYTES - empty.len());
    let exact = encode_success(
        RequestId::String(&exact_id),
        metadata.borrowed("1"),
        generation,
        Success::Shutdown,
        usize::MAX,
    )?;
    assert_eq!(exact.len(), MAX_RESPONSE_BYTES);
    assert_eq!(object(&exact)?["id"], exact_id);
    let oversized_id = format!("{exact_id}x");
    assert_eq!(
        encode_success(
            RequestId::String(&oversized_id),
            metadata.borrowed("1"),
            generation,
            Success::Shutdown,
            usize::MAX,
        ),
        Err(ResponseError::Limit)
    );
    let escaped_id = "\n".repeat(MAX_RESPONSE_BYTES / 2);
    assert_eq!(
        encode_error(
            RequestId::String(&escaped_id),
            metadata.borrowed("1"),
            None,
            Fault::Parse,
            usize::MAX,
        ),
        Err(ResponseError::Limit)
    );
    let oversized_version = "v".repeat(MAX_RESPONSE_BYTES);
    assert_eq!(
        encode_success(
            RequestId::Null,
            metadata.borrowed(&oversized_version),
            generation,
            Success::Initialize,
            usize::MAX,
        ),
        Err(ResponseError::Limit)
    );
    Ok(())
}
