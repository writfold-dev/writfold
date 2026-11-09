//! `writfold.trial/1` negotiation and every refusal path before the trial
//! functions run; the real-process flow is in `crates/writfold/tests`.
use super::{Engine, InputFrame, InvocationIdentity, ServeError, TestResult};
use serde_json::Value;
use std::path::{Path, PathBuf};
use writfold_protocol::session::SessionState;
use writfold_wire::rpc::response::{METHOD_SCHEMAS, TRIAL_METHOD_SCHEMAS};

const INIT: &str = r#"{"jsonrpc":"2.0","id":"i","method":"initialize","params":{"protocol_version":"2","capabilities":["writfold.trial/1"]}}"#;
const PLAIN: &str =
    r#"{"jsonrpc":"2.0","id":"i","method":"initialize","params":{"protocol_version":"2"}}"#;
const CLI_ONLY: [&str; 3] = ["trial/approve", "trial/apply", "trial/recover"];

fn missing_state() -> PathBuf {
    std::env::temp_dir().join(format!(".writfold-serve-absent-{}", std::process::id()))
}

fn send(engine: &mut Engine<'_>, frame: &str) -> Result<Option<Value>, ServeError> {
    let dispatch = engine.handle(InputFrame::Bytes(frame.as_bytes()))?;
    Ok(dispatch
        .response
        .map(|bytes| serde_json::from_slice(&bytes).unwrap_or(Value::Null)))
}

fn call(method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":7,"method":"{method}","params":{params}}}"#)
}

/// (code, category, data.message) of a refusal.
fn fault(value: Option<Value>) -> TestResult<(i64, String, String)> {
    let value = value.ok_or("expected a correlated reply")?;
    let data = &value["error"]["data"];
    Ok((
        value["error"]["code"].as_i64().ok_or("expected an error")?,
        data["category"].as_str().unwrap_or_default().to_owned(),
        data["message"].as_str().unwrap_or_default().to_owned(),
    ))
}

fn engine<'a>(
    identity: &'a InvocationIdentity,
    contract: &'a writfold_domain::ContentDigest,
    state: Option<&'a Path>,
) -> TestResult<Engine<'a>> {
    Ok(Engine::new(identity, contract, state)?)
}

#[test]
fn trial_is_admitted_only_when_requested_and_a_state_is_configured() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    // No startup state: the capability is unsupported and nothing initializes.
    let mut bare = engine(&identity, &contract, None)?;
    let (code, category, _) = fault(send(&mut bare, INIT)?)?;
    assert_eq!((code, category.as_str()), (-32000, "malformed"));
    assert_eq!(bare.session.state(), SessionState::Uninitialized);
    // An unknown capability still refuses, even beside the trial one.
    let state = missing_state();
    let mut with = engine(&identity, &contract, Some(&state))?;
    let unknown = INIT.replace("\"writfold.trial/1\"", "\"writfold.trial/1\",\"knowledge\"");
    assert_eq!(fault(send(&mut with, &unknown)?)?.0, -32000);
    let reply = send(&mut with, INIT)?.ok_or("initialize reply")?;
    let result = &reply["result"];
    assert_eq!(
        result["capabilities"],
        serde_json::json!(["notifications", "writfold.trial/1"])
    );
    assert_eq!(
        result["implementation_scope"],
        "transport-control+trial-worktree"
    );
    let methods: Value = serde_json::from_str(TRIAL_METHOD_SCHEMAS)?;
    assert_eq!(result["methods"], methods);
    // Exactly these methods (sorted keys): no approve, apply or recover.
    let keys: Vec<&String> = methods.as_object().ok_or("methods")?.keys().collect();
    assert_eq!(
        keys,
        [
            "initialize",
            "shutdown",
            "trial/prepare",
            "trial/retain",
            "trial/show"
        ]
    );
    // Without the request, the advertisement is the unchanged control one.
    let mut plain = engine(&identity, &contract, Some(&state))?;
    let reply = send(&mut plain, PLAIN)?.ok_or("initialize reply")?;
    assert_eq!(
        reply["result"]["capabilities"],
        serde_json::json!(["notifications"])
    );
    assert_eq!(
        reply["result"]["methods"],
        serde_json::from_str::<Value>(METHOD_SCHEMAS)?
    );
    Ok(())
}

#[test]
fn lifecycle_and_negotiation_refuse_before_any_trial_function() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let state = missing_state();
    let mut engine = engine(&identity, &contract, Some(&state))?;
    let show = call("trial/show", r#"{"run":"run.1.aaaaaa"}"#);
    // Before initialize: initialization required, as for shutdown.
    assert_eq!(fault(send(&mut engine, &show)?)?.0, -32002);
    // Initialized without the capability: unsupported requested capability.
    send(&mut engine, PLAIN)?;
    let (code, category, message) = fault(send(&mut engine, &show)?)?;
    assert_eq!((code, category.as_str()), (-32000, "malformed"));
    assert_eq!(message, "unsupported requested capability");
    // The capability cannot be negotiated late.
    let (code, category, _) = fault(send(&mut engine, INIT)?)?;
    assert_eq!((code, category.as_str()), (-32000, "conflict"));
    let stop = send(
        &mut engine,
        r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#,
    )?;
    assert!(stop.is_some());
    // After shutdown: shut down.
    let (code, _, message) = fault(send(&mut engine, &show)?)?;
    assert_eq!((code, message.as_str()), (-32002, "engine is shut down"));
    Ok(())
}

#[test]
fn approve_apply_and_recover_are_method_not_found_in_every_state() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let state = missing_state();
    let mut engine = engine(&identity, &contract, Some(&state))?;
    for stage in 0..2 {
        for method in CLI_ONLY {
            let (code, _, message) = fault(send(&mut engine, &call(method, r#"{"run":"r"}"#))?)?;
            assert_eq!(
                (code, message.as_str()),
                (-32601, "Method not found"),
                "{method}"
            );
        }
        if stage == 0 {
            send(&mut engine, INIT)?;
        }
    }
    Ok(())
}

#[test]
fn parameters_are_strict_and_notifications_do_not_run() -> TestResult {
    let identity = InvocationIdentity::new()?;
    let contract = crate::contract::digest()?;
    let state = missing_state();
    let mut engine = engine(&identity, &contract, Some(&state))?;
    send(&mut engine, INIT)?;
    let repo = r#""repo":"/r","commit":"c","file":"f""#;
    for (method, params) in [
        ("trial/show", r#"{}"#.to_owned()),
        ("trial/show", r#"{"run":"r","extra":1}"#.to_owned()),
        ("trial/show", r#"{"run":7}"#.to_owned()),
        ("trial/show", r#"{"run":""}"#.to_owned()),
        ("trial/show", r#"null"#.to_owned()),
        ("trial/retain", format!(r#"{{{repo},"start":0,"end":1.0}}"#)),
        ("trial/retain", format!(r#"{{{repo},"start":-1,"end":1}}"#)),
        (
            "trial/retain",
            format!(r#"{{{repo},"start":0,"end":1,"state":"/x"}}"#),
        ),
        (
            "trial/retain",
            r#"{"repo":"rel","commit":"c","file":"f","start":0,"end":1}"#.to_owned(),
        ),
        ("trial/prepare", r#"{"repo":"/r","file":"f"}"#.to_owned()),
        (
            "trial/prepare",
            r#"{"repo":"/r","file":"f","recipe":"r.spl"}"#.to_owned(),
        ),
        (
            "trial/prepare",
            r#"{"repo":"/r","file":"f","recipe":"/r.spl","state":"/s"}"#.to_owned(),
        ),
    ] {
        let (code, _, _) = fault(send(&mut engine, &call(method, &params))?)?;
        assert_eq!(code, -32602, "{method} {params}");
    }
    let before = engine.generation.get();
    let note = r#"{"jsonrpc":"2.0","method":"trial/show","params":{"run":"run.1.aaaaaa"}}"#;
    assert!(send(&mut engine, note)?.is_none());
    assert_eq!(engine.generation.get(), before + 1);
    // A missing state refuses through the same function as the CLI, read-only.
    let reply = send(
        &mut engine,
        &call("trial/show", r#"{"run":"run.1.aaaaaa"}"#),
    )?;
    let data = reply.ok_or("reply")?["error"]["data"].clone();
    assert_eq!(data["category"], "not-found");
    assert_eq!(data["outcome"], "rejected");
    assert_eq!(data["exit_code"], 1);
    assert_eq!(data["message"], "No such file or directory (os error 2)");
    Ok(())
}
