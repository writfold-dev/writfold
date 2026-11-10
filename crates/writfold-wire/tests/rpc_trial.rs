//! `writfold.trial/1`: closed parameter schemas, the negotiated advertisement,
//! bounded results and refusals, and the published contract's file pins.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use writfold_domain::{ContentDigest, InstanceId, RequestGeneration, SessionIncarnation};
use writfold_wire::rpc::response::{
    CONTRACT, MAX_RESPONSE_BYTES, METHOD_SCHEMAS, ResponseError, ServerMetadata, ShowPayload,
    Success, TRIAL_METHOD_SCHEMAS, TrialCategory, TrialOutcome, TrialRefusal, encode_success,
    encode_trial_refusal, encode_trial_success,
};
use writfold_wire::rpc::trial::{
    EvidenceListParams, EvidenceShowParams, PrepareParams, RetainParams, RunParams, ShowParams,
    TrialMethod,
};
use writfold_wire::rpc::{Capability, Initialize, ParamsError, RequestId};

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

fn value(text: &str) -> TestResult<Value> {
    Ok(serde_json::from_str(text)?)
}

#[test]
fn only_the_six_trial_methods_are_registered() {
    for (name, method) in [
        ("trial/show", Some(TrialMethod::Show)),
        ("trial/retain", Some(TrialMethod::Retain)),
        ("trial/prepare", Some(TrialMethod::Prepare)),
        ("trial/run", Some(TrialMethod::Run)),
        ("trial/evidence-list", Some(TrialMethod::EvidenceList)),
        ("trial/evidence-show", Some(TrialMethod::EvidenceShow)),
        ("trial/evidence-labels", None),
        ("trial/evidence", None),
        ("trial/approve", None),
        ("trial/apply", None),
        ("trial/recover", None),
        ("trial/Show", None),
        ("trial/show ", None),
    ] {
        assert_eq!(TrialMethod::lookup(name), method, "{name}");
        if let Some(method) = method {
            assert_eq!(method.as_str(), name);
        }
    }
    assert!(!TrialMethod::Show.writes());
    assert!(!TrialMethod::EvidenceList.writes() && !TrialMethod::EvidenceShow.writes());
    assert!(TrialMethod::Retain.writes() && TrialMethod::Prepare.writes());
    assert!(TrialMethod::Run.writes());
}

#[test]
fn run_and_evidence_parameters_are_closed_typed_and_absolute() -> TestResult {
    let run = value(
        r#"{"repo":"/r","file":"f","recipe":"/x.spl","tuft":"/q.tuft","weft":"/w.weft","weft_bin":"/bin/weft"}"#,
    )?;
    let parsed = RunParams::parse(Some(&run))?;
    assert_eq!(parsed.weft, Some(("/w.weft", "/bin/weft")));
    assert_eq!(parsed.tuft, Some("/q.tuft"));
    let bare = value(r#"{"repo":"/r","file":"f","recipe":"/x"}"#)?;
    let bare = RunParams::parse(Some(&bare))?;
    assert_eq!((bare.tuft, bare.weft), (None, None));
    for (text, error) in [
        (
            r#"{"repo":"/r","file":"f","recipe":"/x","weft":"/w"}"#,
            ParamsError::MissingField,
        ),
        (
            r#"{"repo":"/r","file":"f","recipe":"/x","weft_bin":"/b"}"#,
            ParamsError::MissingField,
        ),
        (
            r#"{"repo":"/r","file":"f","recipe":"/x","tuft":"q.tuft"}"#,
            ParamsError::InvalidField,
        ),
        (
            r#"{"repo":"/r","file":"f","recipe":"/x","approve":true}"#,
            ParamsError::UnknownField,
        ),
        (
            r#"{"repo":"/r","file":"f","recipe":"/x","state":"/s"}"#,
            ParamsError::UnknownField,
        ),
    ] {
        assert_eq!(RunParams::parse(Some(&value(text)?)), Err(error), "{text}");
    }
    let empty = value(r#"{}"#)?;
    let list = EvidenceListParams::parse(Some(&empty))?;
    assert_eq!((list.path, list.commit, list.limit), (None, None, None));
    let list = value(r#"{"path":"src/","commit":"81071d8","limit":3}"#)?;
    assert_eq!(EvidenceListParams::parse(Some(&list))?.limit, Some(3));
    for text in [
        r#"{"limit":-1}"#,
        r#"{"limit":1.0}"#,
        r#"{"out":"/f"}"#,
        r#"{"path":""}"#,
    ] {
        assert!(
            EvidenceListParams::parse(Some(&value(text)?)).is_err(),
            "{text}"
        );
    }
    let show = value(r#"{"binding":"entry","context_lines":2,"repo":"/r"}"#)?;
    let parsed = EvidenceShowParams::parse(Some(&show))?;
    assert_eq!((parsed.context_lines, parsed.repo), (Some(2), Some("/r")));
    for text in [
        r#"{}"#,
        r#"{"binding":"b","repo":"r"}"#,
        r#"{"binding":"b","x":1}"#,
    ] {
        assert!(
            EvidenceShowParams::parse(Some(&value(text)?)).is_err(),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn the_capability_is_admitted_by_exact_spelling() -> TestResult {
    let params = value(r#"{"protocol_version":"2","capabilities":["writfold.trial/1"]}"#)?;
    let admitted = Initialize::parse(Some(&params)).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        admitted.capabilities().collect::<Vec<_>>(),
        [Capability::TrialV1]
    );
    assert_eq!(Capability::TrialV1.as_str(), "writfold.trial/1");
    for spelling in ["writfold.trial/2", "writfold.trial", "trial"] {
        let params = json!({"protocol_version": "2", "capabilities": [spelling]});
        assert_eq!(
            Initialize::parse(Some(&params)).err(),
            Some(ParamsError::UnsupportedCapability)
        );
    }
    Ok(())
}

#[test]
fn parameters_are_closed_typed_and_absolute() -> TestResult {
    let show = value(r#"{"run":"run.1.aaaaaa"}"#)?;
    assert_eq!(ShowParams::parse(Some(&show))?.run, "run.1.aaaaaa");
    let retain =
        value(r#"{"repo":"/r","commit":"c","file":"f","start":0,"end":18446744073709551615}"#)?;
    assert_eq!(RetainParams::parse(Some(&retain))?.end, u64::MAX);
    let prepare = value(r#"{"repo":"/r","file":"f","recipe":"/x.spl","report":"/w.json"}"#)?;
    assert_eq!(
        PrepareParams::parse(Some(&prepare))?.report,
        Some("/w.json")
    );
    for (text, error) in [
        (r#"{"run":"r","state":"/s"}"#, ParamsError::UnknownField),
        (r#"{}"#, ParamsError::MissingField),
        (r#"{"run":null}"#, ParamsError::InvalidField),
        (r#"{"run":"a\u0000b"}"#, ParamsError::InvalidField),
        (r#"[]"#, ParamsError::ExpectedObject),
    ] {
        assert_eq!(ShowParams::parse(Some(&value(text)?)), Err(error), "{text}");
    }
    assert_eq!(ShowParams::parse(None), Err(ParamsError::ExpectedObject));
    for text in [
        r#"{"repo":"/r","commit":"c","file":"f","start":0,"end":18446744073709551616}"#,
        r#"{"repo":"/r","commit":"c","file":"f","start":0,"end":1e3}"#,
        r#"{"repo":"/r","commit":"c","file":"f","start":-1,"end":1}"#,
        r#"{"repo":"/r","commit":"c","file":"f","start":"0","end":1}"#,
        r#"{"repo":"./r","commit":"c","file":"f","start":0,"end":1}"#,
    ] {
        assert!(RetainParams::parse(Some(&value(text)?)).is_err(), "{text}");
    }
    let relative = value(r#"{"repo":"/r","file":"f","recipe":"/x","report":"w.json"}"#)?;
    assert_eq!(
        PrepareParams::parse(Some(&relative)),
        Err(ParamsError::InvalidField)
    );
    Ok(())
}

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
            digest: "b".repeat(64).parse()?,
        })
    }
    fn borrowed(&self) -> ServerMetadata<'_> {
        ServerMetadata::new("1", &self.instance, &self.incarnation, &self.digest)
    }
}

#[test]
fn the_trial_advertisement_extends_the_control_one() -> TestResult {
    let control: Value = serde_json::from_str(METHOD_SCHEMAS)?;
    let trial: Value = serde_json::from_str(TRIAL_METHOD_SCHEMAS)?;
    assert_eq!(trial["initialize"], control["initialize"]);
    assert_eq!(trial["shutdown"], control["shutdown"]);
    assert_eq!(trial.as_object().ok_or("methods")?.len(), 8);
    let metadata = Metadata::new()?;
    let bytes = encode_success(
        RequestId::Null,
        metadata.borrowed(),
        RequestGeneration::from_u64(1),
        Success::InitializeTrial,
        MAX_RESPONSE_BYTES,
    )?;
    assert_eq!(bytes.iter().filter(|&&b| b == b'\n').count(), 1);
    let result = &serde_json::from_slice::<Value>(&bytes)?["result"];
    assert_eq!(
        result["capabilities"],
        json!(["notifications", "writfold.trial/1"])
    );
    assert_eq!(
        result["implementation_scope"],
        "transport-control+trial-worktree"
    );
    assert_eq!(result["methods"], trial);
    assert!(
        result["trust_boundary"]
            .as_str()
            .unwrap_or_default()
            .contains("CLI-only")
    );
    Ok(())
}

#[test]
fn results_are_bounded_and_refusals_carry_typed_outcomes() -> TestResult {
    let metadata = Metadata::new()?;
    let generation = RequestGeneration::from_u64(9);
    let payload = ShowPayload {
        run: "r",
        text: "line\n\"quoted\"",
    };
    let bytes = encode_trial_success(
        RequestId::String("s"),
        metadata.borrowed(),
        generation,
        &payload,
        MAX_RESPONSE_BYTES,
    )?;
    let reply: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(reply["result"]["payload"]["text"], "line\n\"quoted\"");
    assert_eq!(reply["result"]["generation"], 9);
    let big = "x".repeat(MAX_RESPONSE_BYTES);
    let payload = ShowPayload {
        run: "r",
        text: &big,
    };
    assert_eq!(
        encode_trial_success(
            RequestId::Null,
            metadata.borrowed(),
            generation,
            &payload,
            MAX_RESPONSE_BYTES
        ),
        Err(ResponseError::Limit)
    );
    let refusal = TrialRefusal {
        category: TrialCategory::Incomplete,
        outcome: TrialOutcome::Unknown,
        message: "prepared run r, but ...",
    };
    let bytes = encode_trial_refusal(
        RequestId::Null,
        metadata.borrowed(),
        generation,
        refusal,
        MAX_RESPONSE_BYTES,
    )?;
    let data = &serde_json::from_slice::<Value>(&bytes)?["error"]["data"];
    assert_eq!(data["outcome"], "unknown");
    assert_eq!(data["exit_code"], Value::Null);
    assert_eq!(data["diagnostic"]["code"], "E105");
    assert_eq!(data["generation"], 9);
    Ok(())
}

#[test]
fn the_contract_pins_every_published_file() -> TestResult {
    let contract: Value = serde_json::from_slice(CONTRACT)?;
    let files = contract["files"].as_array().ok_or("files")?;
    let published: [(&str, &[u8]); 2] = [
        ("methods.json", METHOD_SCHEMAS.as_bytes()),
        ("trial-methods.json", TRIAL_METHOD_SCHEMAS.as_bytes()),
    ];
    for (name, bytes) in published {
        let entry = files.iter().find(|f| f["path"] == name).ok_or(name)?;
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(entry["sha256"], digest, "{name}");
        assert_eq!(entry["bytes"], bytes.len(), "{name}");
    }
    assert_eq!(
        contract["capabilities"]["writfold.trial/1"]["cli_only"]["methods"],
        json!(["trial/approve", "trial/apply", "trial/recover"])
    );
    Ok(())
}
