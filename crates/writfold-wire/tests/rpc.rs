use writfold_wire::rpc::{
    Capability, EnvelopeError, Frame, FrameError, Initialize, InitializeParams, MAX_REQUEST_BYTES,
    ParamsError, Request, RequestId,
};
use writfold_wire::{Budget, Error, Limits};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn limits() -> Result<Limits, Error> {
    Limits::new(MAX_REQUEST_BYTES, 1024, 8192, 16384)
}

fn parsed(input: &str) -> TestResult<Frame> {
    Ok(Frame::parse(input.as_bytes(), limits()?)?)
}

fn valid_request(frame: &Frame) -> TestResult<Request<'_>> {
    frame.request().map_err(|error| format!("{error:?}").into())
}

#[test]
fn ids_retain_absence_null_strings_and_unbounded_signed_integers() -> TestResult {
    for (field, notification) in [("", true), (",\"id\":null", false), (",\"id\":\"\"", false)] {
        let frame = parsed(&format!(r#"{{"jsonrpc":"2.0","method":"unknown"{field}}}"#))?;
        let request = valid_request(&frame)?;
        assert_eq!(request.is_notification(), notification);
        assert_eq!(request.method(), "unknown");
        assert_eq!(request.params(), None);
        match field {
            "" => assert_eq!(request.id(), RequestId::Absent),
            ",\"id\":null" => assert_eq!(request.id(), RequestId::Null),
            _ => assert_eq!(request.id(), RequestId::String("")),
        }
    }
    for integer in [
        "0".to_owned(),
        "-1".to_owned(),
        "9".repeat(5000),
        format!("-{}", "9".repeat(5000)),
    ] {
        let frame = parsed(&format!(
            r#"{{"jsonrpc":"2.0","method":"","id":{integer}}}"#
        ))?;
        let request = valid_request(&frame)?;
        let RequestId::Integer(value) = request.id() else {
            return Err("integer ID changed type".into());
        };
        assert_eq!(value.as_decimal(), integer);
    }
    let frame = parsed(r#"{"jsonrpc":"2.0","method":"initialize","id":-0}"#)?;
    assert!(
        matches!(valid_request(&frame)?.id(), RequestId::Integer(id) if id.as_decimal() == "0")
    );
    Ok(())
}

#[test]
fn invalid_ids_do_not_leak_into_error_response_ids() -> TestResult {
    for id in ["true", "false", "1.5", "1.0", "1e0", "-0.0", "[]", "{}"] {
        let frame = parsed(&format!(
            r#"{{"jsonrpc":"2.0","method":"initialize","id":{id}}}"#
        ))?;
        let error = frame.request().err().ok_or("invalid ID admitted")?;
        assert_eq!(error.reason(), EnvelopeError::InvalidId);
        assert_eq!(error.response_id(), RequestId::Null);
        assert_eq!(error.code(), -32600);
    }
    Ok(())
}

#[test]
fn malformed_envelopes_are_not_notifications_or_unknown_methods() -> TestResult {
    for input in ["[]", "[{}]", "null", "true", "0", "\"method\""] {
        assert_eq!(
            parsed(input)?
                .request()
                .err()
                .ok_or("non-object admitted")?
                .reason(),
            EnvelopeError::ExpectedObject
        );
    }
    for (input, reason) in [
        (r#"{"method":"x"}"#, EnvelopeError::InvalidVersion),
        (
            r#"{"jsonrpc":2.0,"method":"x"}"#,
            EnvelopeError::InvalidVersion,
        ),
        (r#"{"jsonrpc":"2.0"}"#, EnvelopeError::InvalidMethod),
        (
            r#"{"jsonrpc":"2.0","method":null}"#,
            EnvelopeError::InvalidMethod,
        ),
        (
            r#"{"jsonrpc":"2.0","method":7}"#,
            EnvelopeError::InvalidMethod,
        ),
        (
            r#"{"jsonrpc":"2.0","method":"x","extra":0}"#,
            EnvelopeError::UnknownField,
        ),
    ] {
        let frame = parsed(input)?;
        let error = frame.request().err().ok_or("malformed envelope admitted")?;
        assert_eq!(error.reason(), reason);
        assert_eq!(error.response_id(), RequestId::Null);
    }
    let frame = parsed(r#"{"jsonrpc":"wrong","method":"x","id":"kept"}"#)?;
    assert_eq!(
        frame
            .request()
            .err()
            .ok_or("bad version admitted")?
            .response_id(),
        RequestId::String("kept")
    );
    let frame = parsed(r#"{"jsonrpc":"2.0","method":"unknown","params":false}"#)?;
    let request = valid_request(&frame)?;
    assert!(request.is_notification());
    assert_eq!(request.method(), "unknown");
    Ok(())
}

#[test]
fn initialization_preserves_capability_order_duplicates_and_absence() -> TestResult {
    let frame = parsed(
        r#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2","capabilities":["notifications","graph","graph","citations","knowledge","authored"]}}"#,
    )?;
    let request = valid_request(&frame)?;
    assert!(request.is_notification());
    let initialize = Initialize::parse(request.params())?;
    assert_eq!(initialize.protocol_version(), "2");
    assert_eq!(
        initialize.capabilities().collect::<Vec<_>>(),
        [
            Capability::Notifications,
            Capability::Graph,
            Capability::Graph,
            Capability::Citations,
            Capability::Knowledge,
            Capability::Authored
        ]
    );
    let frame =
        parsed(r#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2"}}"#)?;
    assert_eq!(
        Initialize::parse(valid_request(&frame)?.params())?
            .capabilities()
            .next(),
        None
    );
    Ok(())
}

#[test]
fn initialization_rejects_wrong_types_versions_and_capabilities() -> TestResult {
    for (params, expected) in [
        ("null", ParamsError::ExpectedObject),
        ("[]", ParamsError::ExpectedObject),
        ("{}", ParamsError::InvalidProtocolVersion),
        (
            r#"{"protocol_version":2}"#,
            ParamsError::InvalidProtocolVersion,
        ),
        (
            r#"{"protocol_version":true}"#,
            ParamsError::InvalidProtocolVersion,
        ),
        (
            r#"{"protocol_version":"02"}"#,
            ParamsError::InvalidProtocolVersion,
        ),
        (
            r#"{"protocol_version":"3"}"#,
            ParamsError::InvalidProtocolVersion,
        ),
        (
            r#"{"protocol_version":"2","other":0}"#,
            ParamsError::UnknownField,
        ),
        (
            r#"{"protocol_version":"2","capabilities":null}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":"graph"}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":[true]}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":["unknown",1]}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":["Graph"]}"#,
            ParamsError::UnsupportedCapability,
        ),
    ] {
        let frame = parsed(&format!(
            r#"{{"jsonrpc":"2.0","method":"initialize","params":{params}}}"#
        ))?;
        let request = valid_request(&frame)?;
        assert_eq!(Initialize::parse(request.params()).err(), Some(expected));
        assert_eq!(
            expected.code(),
            if expected == ParamsError::UnsupportedCapability {
                -32000
            } else {
                -32602
            }
        );
    }
    assert_eq!(
        Initialize::parse(None).err(),
        Some(ParamsError::InvalidProtocolVersion)
    );
    Ok(())
}

#[test]
fn schema_validation_precedes_state_and_capability_admission() -> TestResult {
    let frame = parsed(
        r#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2","capabilities":["unknown"]}}"#,
    )?;
    let params = InitializeParams::parse(valid_request(&frame)?.params())?;
    assert_eq!(
        params.admit().err(),
        Some(ParamsError::UnsupportedCapability)
    );
    Ok(())
}

#[test]
fn shutdown_params_are_closed_and_missing_differs_from_null() -> TestResult {
    for suffix in ["", ",\"params\":{}"] {
        let frame = parsed(&format!(
            r#"{{"jsonrpc":"2.0","method":"shutdown"{suffix}}}"#
        ))?;
        assert_eq!(valid_request(&frame)?.shutdown_params(), Ok(()));
    }
    for params in ["null", "[]", "true", r#"{"extra":1}"#] {
        let frame = parsed(&format!(
            r#"{{"jsonrpc":"2.0","method":"shutdown","params":{params}}}"#
        ))?;
        assert!(valid_request(&frame)?.shutdown_params().is_err());
    }
    Ok(())
}

#[test]
fn utf8_duplicates_and_nonfinite_numbers_are_refused() -> TestResult {
    assert!(matches!(
        Frame::parse(b"\xff", limits()?),
        Err(FrameError::Json(Error::Unicode { .. }))
    ));
    for input in [
        r#"{"method":"x","\u006dethod":"y"}"#,
        r#"{"params":{"x":1,"x":2}}"#,
    ] {
        assert!(matches!(
            Frame::parse(input.as_bytes(), limits()?),
            Err(FrameError::Json(Error::DuplicateKey { .. }))
        ));
    }
    assert_eq!(
        Frame::parse(br#"{"params":[1e9999]}"#, limits()?).err(),
        Some(FrameError::NonfiniteNumber)
    );
    let underflow = parsed(r#"{"jsonrpc":"2.0","method":"x","params":[1e-9999]}"#)?;
    let params = valid_request(&underflow)?
        .params()
        .ok_or("missing params")?;
    assert_eq!(params[0].as_f64(), Some(0.0));
    assert!(matches!(
        Frame::parse(b"{}{}", limits()?),
        Err(FrameError::Json(Error::TrailingInput { .. }))
    ));
    Ok(())
}

#[test]
fn caller_budgets_and_transport_ceiling_are_independent() -> TestResult {
    for (input, limit, budget) in [
        (b"{}".as_slice(), Limits::new(1, 8, 8, 8)?, Budget::Bytes),
        (b"[[]]", Limits::new(64, 1, 8, 8)?, Budget::Depth),
        (
            b"[0,1]",
            Limits::new(64, 8, 1, 8)?,
            Budget::ContainerEntries,
        ),
        (b"[0]", Limits::new(64, 8, 8, 1)?, Budget::Nodes),
    ] {
        assert!(
            matches!(Frame::parse(input, limit), Err(FrameError::Json(Error::Limit { budget: actual, .. })) if actual == budget)
        );
    }
    let oversized = vec![b' '; MAX_REQUEST_BYTES + 1];
    assert!(matches!(
        Frame::parse(&oversized, Limits::new(usize::MAX, 8, 8, 8)?),
        Err(FrameError::Json(Error::Limit {
            budget: Budget::Bytes,
            ..
        }))
    ));
    let request = r#"{"jsonrpc":"2.0","method":"x"}"#;
    let exact = parsed(&format!(
        "{}{request}",
        " ".repeat(MAX_REQUEST_BYTES - request.len())
    ))?;
    assert_eq!(valid_request(&exact)?.method(), "x");
    Ok(())
}

fn deep_frames() -> TestResult {
    let nested = format!("{}0{}", "[".repeat(1000), "]".repeat(1000));
    let frame = parsed(&nested)?;
    assert_eq!(
        frame.request().err().ok_or("batch admitted")?.reason(),
        EnvelopeError::ExpectedObject
    );
    drop(frame);
    let overflow = format!("{}1e9999{}", "[".repeat(1000), "]".repeat(1000));
    assert_eq!(
        Frame::parse(overflow.as_bytes(), limits()?).err(),
        Some(FrameError::NonfiniteNumber)
    );
    Ok(())
}

#[test]
fn rejected_deep_tree_and_nested_float_drop_without_recursive_ownership() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| deep_frames().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boundary consumer thread panicked")?
        .map_err(Into::into)
}
