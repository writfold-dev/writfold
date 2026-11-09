use super::*;
use serde::Deserialize;
use writfold_protocol::session::SessionState;
use writfold_wire::rpc::response::ResponseError;

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

// A typed observation of the fields needed by the lifecycle tests. Complete
// wire shape/identity behavior is checked at the wire and real-process boundary.
#[derive(Deserialize)]
struct Reply {
    id: Option<serde::de::IgnoredAny>,
    result: Option<SuccessFields>,
    error: Option<FailureFields>,
}
#[derive(Deserialize)]
struct SuccessFields {
    #[serde(default)]
    capabilities: Vec<String>,
    generation: u64,
}
#[derive(Deserialize)]
struct FailureFields {
    code: i32,
    data: FailureData,
}
#[derive(Deserialize)]
struct FailureData {
    category: String,
    generation: Option<u64>,
}
impl Reply {
    fn success(self) -> TestResult<SuccessFields> {
        self.result
            .ok_or_else(|| "expected successful control reply".into())
    }
    fn failure(self) -> TestResult<FailureFields> {
        self.error
            .ok_or_else(|| "expected rejected control reply".into())
    }
}

fn response(dispatch: Dispatch) -> TestResult<Reply> {
    let bytes = dispatch.response.ok_or("expected a correlated response")?;
    if bytes.last() != Some(&b'\n') || bytes[..bytes.len() - 1].contains(&b'\n') {
        return Err("response is not exactly one LF-terminated frame".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

#[test]
fn invalid_requests_and_capabilities_cannot_initialize_the_session() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let mut engine = Engine::new(&identity, &contract, None)?;
    for (request, code) in [
        (
            r#"{"jsonrpc":"2.0","id":1,"method":"shutdown","params":{"extra":true}}"#,
            -32002,
        ),
        (r#"{"jsonrpc":"2.0","id":2,"method":"unknown"}"#, -32601),
        (
            r#"{"jsonrpc":"2.0","id":3,"method":"initialize","params":{"protocol_version":2}}"#,
            -32602,
        ),
        (
            r#"{"jsonrpc":"2.0","id":4,"method":"initialize","params":{"protocol_version":"2","capabilities":["knowledge"]}}"#,
            -32000,
        ),
    ] {
        let value = response(engine.handle(InputFrame::Bytes(request.as_bytes()))?)?;
        assert_eq!(value.failure()?.code, code);
        assert_eq!(engine.session.state(), SessionState::Uninitialized);
    }
    let value = response(engine.handle(InputFrame::Bytes(br#"{"jsonrpc":"2.0","id":"ready","method":"initialize","params":{"protocol_version":"2"}}"#))?)?;
    let value = value.success()?;
    assert_eq!(value.capabilities, ["notifications"]);
    assert_eq!(value.generation, 5);
    assert_eq!(engine.session.state(), SessionState::Initialized);
    Ok(())
}

#[test]
fn double_initialization_and_shutdown_keep_reference_refusal_precedence() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let mut engine = Engine::new(&identity, &contract, None)?;
    engine.handle(InputFrame::Bytes(
        br#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2"}}"#,
    ))?;
    for (request, code, category) in [
        (
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":2}}"#,
            -32602,
            "malformed",
        ),
        (
            r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocol_version":"2","capabilities":["not-implemented"]}}"#,
            -32000,
            "conflict",
        ),
        (
            r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{"extra":true}}"#,
            -32602,
            "malformed",
        ),
    ] {
        let value = response(engine.handle(InputFrame::Bytes(request.as_bytes()))?)?;
        let failure = value.failure()?;
        assert_eq!(failure.code, code);
        assert_eq!(failure.data.category, category);
        assert_eq!(engine.session.state(), SessionState::Initialized);
    }
    let shutdown = engine.handle(InputFrame::Bytes(
        br#"{"jsonrpc":"2.0","id":null,"method":"shutdown"}"#,
    ))?;
    assert_eq!(shutdown.flow, Flow::Stop);
    assert!(response(shutdown)?.id.is_none());
    assert_eq!(engine.session.state(), SessionState::ShutDown);
    let unknown = response(engine.handle(InputFrame::Bytes(
        br#"{"jsonrpc":"2.0","id":4,"method":"unknown"}"#,
    ))?)?;
    assert_eq!(unknown.failure()?.code, -32601);
    let stopped = response(engine.handle(InputFrame::Bytes(
        br#"{"jsonrpc":"2.0","id":5,"method":"initialize","params":false}"#,
    ))?)?;
    assert_eq!(stopped.failure()?.code, -32002);
    Ok(())
}

#[test]
fn valid_notifications_are_silent_but_invalid_envelopes_are_not() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let mut engine = Engine::new(&identity, &contract, None)?;
    for request in [
        br#"{"jsonrpc":"2.0","method":"unknown"}"#.as_slice(),
        br#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":2}}"#,
        br#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2"}}"#,
    ] {
        assert!(
            engine
                .handle(InputFrame::Bytes(request))?
                .response
                .is_none()
        );
    }
    assert_eq!(engine.session.state(), SessionState::Initialized);
    let invalid =
        response(engine.handle(InputFrame::Bytes(br#"{"jsonrpc":"2.0","method":true}"#))?)?;
    assert!(invalid.id.is_none());
    let failure = invalid.failure()?;
    assert_eq!(failure.code, -32600);
    assert_eq!(failure.data.generation, None);
    let shutdown = engine.handle(InputFrame::Bytes(
        br#"{"jsonrpc":"2.0","method":"shutdown"}"#,
    ))?;
    assert!(shutdown.response.is_none());
    assert_eq!(shutdown.flow, Flow::Stop);
    assert_eq!(engine.generation.get(), 4);
    Ok(())
}

#[test]
fn bounded_reply_failure_does_not_commit_initialization_or_reuse_a_generation() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let mut engine = Engine::new(&identity, &contract, None)?;
    engine.maximum_response = 1;
    let request = br#"{"jsonrpc":"2.0","id":"request","method":"initialize","params":{"protocol_version":"2"}}"#;
    assert!(matches!(
        engine.handle(InputFrame::Bytes(request)),
        Err(ServeError::Response(ResponseError::Limit))
    ));
    assert_eq!(engine.session.state(), SessionState::Uninitialized);
    engine.maximum_response = MAX_RESPONSE_BYTES;
    let accepted = response(engine.handle(InputFrame::Bytes(request))?)?;
    assert_eq!(accepted.success()?.generation, 2);
    assert_eq!(engine.session.state(), SessionState::Initialized);
    Ok(())
}

#[test]
fn exhausted_generations_refuse_without_wrap_or_lifecycle_change() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let mut engine = Engine::new(&identity, &contract, None)?;
    engine.generation = RequestGeneration::from_u64(u64::MAX);
    let denied = engine.handle(InputFrame::Bytes(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2"}}"#,
    ))?;
    assert_eq!(denied.flow, Flow::Stop);
    let value = response(denied)?;
    let failure = value.failure()?;
    assert_eq!(failure.data.category, "conflict");
    assert_eq!(failure.data.generation, None);
    assert_eq!(engine.generation.get(), u64::MAX);
    assert_eq!(engine.session.state(), SessionState::Uninitialized);
    Ok(())
}

#[path = "engine_trial_tests.rs"]
mod trial;
