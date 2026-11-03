use writfold_wire::rpc::{
    Capability, Frame, FrameError, Initialize, InitializeParams, MAX_REQUEST_BYTES, ParamsError,
    Request, StorageProfileVersion,
};
use writfold_wire::{Error, Limits};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn frame(params: &str) -> TestResult<Frame> {
    let input = format!(
        r#"{{"jsonrpc":"2.0","id":"storage-init","method":"initialize","params":{params}}}"#
    );
    Ok(Frame::parse(
        input.as_bytes(),
        Limits::new(MAX_REQUEST_BYTES, 32, 256, 512)?,
    )?)
}

fn request(frame: &Frame) -> TestResult<Request<'_>> {
    let request = frame.request().map_err(|error| format!("{error:?}"))?;
    assert_eq!(request.method(), "initialize");
    Ok(request)
}

#[test]
fn old_clients_initialize_without_implicitly_negotiating_storage() -> TestResult {
    for params in [
        r#"{"protocol_version":"2"}"#,
        r#"{"protocol_version":"2","capabilities":[]}"#,
        r#"{"protocol_version":"2","capabilities":["citations","graph","knowledge","authored","notifications"]}"#,
    ] {
        let frame = frame(params)?;
        let request = request(&frame)?;
        let initialize = InitializeParams::parse(request.params())?.admit()?;
        assert_eq!(initialize.protocol_version(), "2");
        assert_eq!(
            initialize.require_storage_profile(),
            Err(ParamsError::UnsupportedStorageProfile)
        );
        assert_eq!(
            Initialize::parse(request.params())?.require_storage_profile(),
            Err(ParamsError::UnsupportedStorageProfile)
        );
    }
    assert_eq!(ParamsError::UnsupportedStorageProfile.code(), -32000);
    Ok(())
}

#[test]
fn explicit_storage_profile_preserves_protocol_and_capability_order() -> TestResult {
    let frame = frame(
        r#"{"protocol_version":"2","capabilities":["notifications","writfold.storage/1","graph","graph","citations","writfold.storage/1","knowledge","authored"]}"#,
    )?;
    let request = request(&frame)?;
    let initialize = InitializeParams::parse(request.params())?.admit()?;
    assert_eq!(initialize.protocol_version(), "2");
    assert_eq!(
        initialize.require_storage_profile(),
        Ok(StorageProfileVersion::V1)
    );
    assert_eq!(
        Initialize::parse(request.params())?.require_storage_profile(),
        Ok(StorageProfileVersion::V1)
    );
    assert_eq!(
        initialize.capabilities().collect::<Vec<_>>(),
        [
            Capability::Notifications,
            Capability::StorageV1,
            Capability::Graph,
            Capability::Graph,
            Capability::Citations,
            Capability::StorageV1,
            Capability::Knowledge,
            Capability::Authored,
        ]
    );
    assert_eq!(
        initialize
            .capabilities()
            .map(Capability::as_str)
            .collect::<Vec<_>>(),
        [
            "notifications",
            "writfold.storage/1",
            "graph",
            "graph",
            "citations",
            "writfold.storage/1",
            "knowledge",
            "authored",
        ]
    );
    assert_eq!(StorageProfileVersion::PIN_SCHEMA, 1);
    assert_eq!(StorageProfileVersion::OPERATION_SCHEMA, 1);
    assert_eq!(StorageProfileVersion::BACKEND, "omnigraph");
    Ok(())
}

#[test]
fn reserved_versions_are_refused_without_falling_back_to_supported_profile() -> TestResult {
    for version in ["", "0", "2", "01", "1.0", "1/extra", "unknown"] {
        for capabilities in [
            format!(r#"["writfold.storage/{version}"]"#),
            format!(r#"["writfold.storage/1","writfold.storage/{version}"]"#),
            format!(r#"["writfold.storage/{version}","writfold.storage/1"]"#),
        ] {
            let frame = frame(&format!(
                r#"{{"protocol_version":"2","capabilities":{capabilities}}}"#
            ))?;
            let params = InitializeParams::parse(request(&frame)?.params())?;
            assert_eq!(
                params.admit().err(),
                Some(ParamsError::UnsupportedStorageProfile)
            );
        }
    }
    Ok(())
}

#[test]
fn support_errors_follow_input_order_and_only_exact_family_is_reserved() -> TestResult {
    for (capabilities, expected) in [
        (
            r#"["unknown","writfold.storage/2"]"#,
            ParamsError::UnsupportedCapability,
        ),
        (
            r#"["writfold.storage/2","unknown"]"#,
            ParamsError::UnsupportedStorageProfile,
        ),
        (
            r#"["writfold.storage/1","unknown"]"#,
            ParamsError::UnsupportedCapability,
        ),
        (
            r#"["writfold.storage"]"#,
            ParamsError::UnsupportedCapability,
        ),
        (
            r#"["Writfold.storage/1"]"#,
            ParamsError::UnsupportedCapability,
        ),
        (
            r#"["writfold.storage-other/1"]"#,
            ParamsError::UnsupportedCapability,
        ),
    ] {
        let frame = frame(&format!(
            r#"{{"protocol_version":"2","capabilities":{capabilities}}}"#
        ))?;
        let params = InitializeParams::parse(request(&frame)?.params())?;
        assert_eq!(params.admit().err(), Some(expected));
        assert_eq!(expected.code(), -32000);
    }
    Ok(())
}

#[test]
fn all_schema_checks_precede_storage_support() -> TestResult {
    for (params, expected) in [
        (
            r#"{"protocol_version":2,"capabilities":["writfold.storage/1"]}"#,
            ParamsError::InvalidProtocolVersion,
        ),
        (
            r#"{"protocol_version":"3","capabilities":["writfold.storage/2"]}"#,
            ParamsError::InvalidProtocolVersion,
        ),
        (
            r#"{"protocol_version":"2","capabilities":"writfold.storage/1"}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":["writfold.storage/2",null]}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":["unknown","writfold.storage/2",1]}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":[false,"writfold.storage/1"]}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":["writfold.storage/1",{}]}"#,
            ParamsError::InvalidCapabilities,
        ),
        (
            r#"{"protocol_version":"2","capabilities":["writfold.storage/2"],"storage_profile":1}"#,
            ParamsError::UnknownField,
        ),
    ] {
        let frame = frame(params)?;
        let request = request(&frame)?;
        assert_eq!(
            InitializeParams::parse(request.params()).err(),
            Some(expected)
        );
        assert_eq!(Initialize::parse(request.params()).err(), Some(expected));
        assert_eq!(expected.code(), -32602);
    }
    Ok(())
}

#[test]
fn duplicate_parameter_fields_fail_at_the_frame_boundary() -> TestResult {
    let input = br#"{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2","capabilities":["writfold.storage/1"],"capabilities":[]}}"#;
    assert!(matches!(
        Frame::parse(input, Limits::new(MAX_REQUEST_BYTES, 32, 256, 512)?),
        Err(FrameError::Json(Error::DuplicateKey { .. }))
    ));
    Ok(())
}
