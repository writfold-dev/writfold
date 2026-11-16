//! Reading the hook's answer and comparing it with `expected.json`.
use super::support::TestResult;
use serde_json::Value;

/// The rule IDs a reason names, sorted.
fn deciders(reason: &str) -> Vec<String> {
    let mut ids: Vec<String> = reason
        .lines()
        .filter_map(|l| l.strip_prefix("writfold guard rule "))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_owned)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// The decision and the reason (or the nudge's text) the hook printed.
pub fn observed(name: &str, stdout: &str) -> TestResult<(String, String)> {
    if stdout.is_empty() {
        let quiet = if name == "stop" { "quiet" } else { "pass" };
        return Ok((quiet.to_owned(), String::new()));
    }
    let value: Value = serde_json::from_str(stdout)?;
    let specific = &value["hookSpecificOutput"];
    let text = |k: &str| specific[k].as_str().unwrap_or("").to_owned();
    Ok(match name {
        "stop" => ("nudge".to_owned(), text("additionalContext")),
        _ => (text("permissionDecision"), text("permissionDecisionReason")),
    })
}

/// What differs from `expected.json`.
pub fn compare(expected: &Value, decision: &str, reason: &str) -> TestResult<Vec<String>> {
    let mut wrong = Vec::new();
    if expected["decision"] != decision {
        wrong.push(format!(
            "decision {decision}, expected {}",
            expected["decision"]
        ));
    }
    if let Some(ids) = expected.get("decided_by") {
        let ids: Vec<String> = serde_json::from_value(ids.clone())?;
        if ids != deciders(reason) {
            wrong.push(format!(
                "decided by {:?}, expected {ids:?}",
                deciders(reason)
            ));
        }
    }
    for piece in expected["reason_has"].as_array().into_iter().flatten() {
        let piece = piece.as_str().ok_or("reason_has holds strings")?;
        if !reason.contains(piece) {
            wrong.push(format!("reason lacks {piece:?}"));
        }
    }
    Ok(wrong)
}
