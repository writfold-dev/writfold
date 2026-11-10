//! `writfold.trial/1` negotiation and every refusal path before the trial
//! functions run; the real-process flow is in `crates/writfold/tests`.
use super::{Engine, InputFrame, InvocationIdentity, ServeError, TestResult};
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::BTreeMap;
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

// Typed observations of the reply fields these tests check; exact wire bytes
// are covered at the wire and real-process boundaries.
#[derive(Default, Deserialize)]
struct Reply {
    #[serde(default)]
    result: Option<Success>,
    #[serde(default)]
    error: Option<Failure>,
}
#[derive(Deserialize)]
struct Success {
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    implementation_scope: String,
    #[serde(default)]
    methods: BTreeMap<String, IgnoredAny>,
}
#[derive(Deserialize)]
struct Failure {
    code: i64,
    data: Data,
}
#[derive(Deserialize)]
struct Data {
    #[serde(default)]
    category: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    outcome: String,
    exit_code: Option<i64>,
}

fn method_names(schemas: &str) -> TestResult<Vec<String>> {
    let parsed: BTreeMap<String, IgnoredAny> = serde_json::from_str(schemas)?;
    Ok(parsed.into_keys().collect())
}

fn send(engine: &mut Engine<'_>, frame: &str) -> Result<Option<Reply>, ServeError> {
    let dispatch = engine.handle(InputFrame::Bytes(frame.as_bytes()))?;
    Ok(dispatch
        .response
        .map(|bytes| serde_json::from_slice(&bytes).unwrap_or_default()))
}

fn call(method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":7,"method":"{method}","params":{params}}}"#)
}

/// (code, category, data.message) of a refusal.
fn fault(value: Option<Reply>) -> TestResult<(i64, String, String)> {
    let error = value
        .ok_or("expected a correlated reply")?
        .error
        .ok_or("expected an error")?;
    Ok((error.code, error.data.category, error.data.message))
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
    let result = reply.result.ok_or("expected success")?;
    assert_eq!(result.capabilities, ["notifications", "writfold.trial/1"]);
    assert_eq!(
        result.implementation_scope,
        "transport-control+trial-worktree"
    );
    let advertised: Vec<String> = result.methods.into_keys().collect();
    assert_eq!(advertised, method_names(TRIAL_METHOD_SCHEMAS)?);
    // Exactly these methods (sorted keys): no approve, apply or recover.
    assert_eq!(
        advertised,
        [
            "initialize",
            "shutdown",
            "trial/evidence-list",
            "trial/evidence-show",
            "trial/prepare",
            "trial/retain",
            "trial/run",
            "trial/show"
        ]
    );
    // Without the request, the advertisement is the unchanged control one.
    let mut plain = engine(&identity, &contract, Some(&state))?;
    let reply = send(&mut plain, PLAIN)?.ok_or("initialize reply")?;
    let result = reply.result.ok_or("expected success")?;
    assert_eq!(result.capabilities, ["notifications"]);
    let advertised: Vec<String> = result.methods.into_keys().collect();
    assert_eq!(advertised, method_names(METHOD_SCHEMAS)?);
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
    for method in ["trial/run", "trial/evidence-list", "trial/evidence-show"] {
        let refused = fault(send(&mut engine, &call(method, "{}"))?)?;
        assert_eq!(refused.0, -32000, "{method}");
        assert_eq!(refused.2, "unsupported requested capability", "{method}");
    }
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
        (
            "trial/run",
            r#"{"repo":"/r","file":"f","recipe":"/r.spl","weft":"/w"}"#.to_owned(),
        ),
        ("trial/evidence-list", r#"{"out":"/f"}"#.to_owned()),
        (
            "trial/evidence-show",
            r#"{"binding":"b","repo":"r"}"#.to_owned(),
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
    let data = reply.ok_or("reply")?.error.ok_or("expected an error")?.data;
    assert_eq!(data.category, "not-found");
    assert_eq!(data.outcome, "rejected");
    assert_eq!(data.exit_code, Some(1));
    assert_eq!(data.message, "No such file or directory (os error 2)");
    Ok(())
}
