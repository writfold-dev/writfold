//! Development-only conformance host. Model assumptions here are not authentication.
use crate::semantic::*;
use crate::{Limits, RawJson};
use serde_json::{Value, json};
use writfold_domain::*;

pub const MAX_LINE_BYTES: usize = 16 * 1024 * 1024;

pub fn invalid_response(id: &str) -> Value {
    json!({"id": id, "ok": false, "error": "invalid_request"})
}

/// Evaluate ordinary requests without a trusted fixture environment.
pub fn evaluate_request(request: &Value) -> Value {
    let result = if request.get("schema_version") == Some(&json!(3)) {
        request3::handle(request, &[], &[], &[])
    } else {
        request::handle(request, &[], &[], &[])
    };
    envelope(request, result)
}
fn envelope(request: &Value, result: Result<Value>) -> Value {
    let id = request.get("id").and_then(Value::as_str).unwrap_or("");
    match result {
        Ok(result) => json!({"id": id, "ok": true, "result": result}),
        Err(_) => invalid_response(id),
    }
}

/// The outer fixture envelope is intentionally separate from ordinary request fields.
/// This API is exclusively for development conformance, never production authorization.
pub fn evaluate_transport(transport: &Value) -> Value {
    let Some(request) = transport.get("request") else {
        return evaluate_request(transport);
    };
    match trusted_result(transport, request) {
        Ok(result) => envelope(request, result),
        Err(_) => invalid_response(""),
    }
}
fn trusted_result(transport: &Value, request: &Value) -> Result<Result<Value>> {
    let outer = obj(transport, "request trusted_environment")?;
    let environment = obj(
        &outer["trusted_environment"],
        "approvals selections retained",
    )?;
    if request.get("schema_version") == Some(&json!(3)) {
        let approvals = items(&environment["approvals"], dec_verified_portfolio_approval)?;
        let selections = items(
            &environment["selections"],
            dec_verified_portfolio_selection_approval,
        )?;
        let retained = items(&environment["retained"], dec_portfolio_state)?;
        if !unique(
            &retained
                .iter()
                .map(|state| state.kb().snapshot())
                .collect::<Vec<_>>(),
        ) {
            return Err(InvalidRequest);
        }
        Ok(request3::handle(
            request,
            &approvals,
            &selections,
            &retained,
        ))
    } else {
        let approvals = items(&environment["approvals"], |value| {
            Ok(ModelApproval::new(dec_plan(&obj(value, "plan")?["plan"])?))
        })?;
        let selections = items(&environment["selections"], |value| {
            Ok(ModelSelectionApproval::new(dec_selection_plan(
                &obj(value, "plan")?["plan"],
            )?))
        })?;
        let retained = items(&environment["retained"], dec_kb)?;
        if !unique(&retained.iter().map(KB::snapshot).collect::<Vec<_>>()) {
            return Err(InvalidRequest);
        }
        Ok(request::handle(request, &approvals, &selections, &retained))
    }
}

/// Parse one bounded JSON document, rejecting duplicate keys, nonintegers and invalid Unicode.
pub fn evaluate_json(bytes: &[u8]) -> Value {
    let limits = match Limits::new(
        MAX_LINE_BYTES,
        crate::MAX_SUPPORTED_DEPTH,
        250_000,
        1_000_000,
    ) {
        Ok(limits) => limits,
        Err(_) => return invalid_response(""),
    };
    match RawJson::parse(bytes, limits) {
        Ok(raw) => evaluate_transport(raw.value()),
        Err(_) => invalid_response(""),
    }
}
