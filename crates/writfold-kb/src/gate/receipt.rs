//! Gate receipts: one JSON file per evaluation,
//! `runs/<run>/gates/<gate>-<UTC compact time>-<6 hex>.json`, written
//! create-new through the write protocol by `gate run` and by nothing else.
//! A receipt binds the gate and its definition digest, the subject (repo,
//! commit, scope digest, dirty paths in scope), the command, how it ended,
//! digests of the retained output tails, the evaluator and the time, and
//! ends with `digest`, the SHA-256 of all of that in canonical form.
//!
//! The digest makes a hand-edited or hand-written receipt read as invalid
//! unless its author recomputes it: it resists mistakes, not adversaries.
//! What resists tampering is Git history (a receipt is a tracked file) and
//! `gate verify`, which re-runs the command at the recorded commit.
use super::def::Definition;
use super::exec::Evaluation;
use super::subject::Scoped;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use writfold_core::gate::{Ending, Outcome, outcome};

pub const SCHEMA: &str = "writfold-gate-receipt/1";
/// The directory of a run that holds its gate receipts.
pub const DIR: &str = "gates";

/// The evaluator every receipt names: this program.
pub fn evaluator() -> String {
    format!("writfold {}", env!("CARGO_PKG_VERSION"))
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// Objects with sorted keys, no spaces: the form the digest is taken over.
pub fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}:{}", Value::String(k.clone()), canonical(&map[k])))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", parts.join(","))
        }
        other => other.to_string(),
    }
}

/// Everything a receipt records, before its digest.
pub struct Facts<'a> {
    pub definition: &'a Definition,
    pub run: &'a str,
    pub repo: &'a str,
    pub commit: &'a str,
    pub scoped: &'a Scoped,
    pub evaluation: &'a Evaluation,
    pub subject_moved: bool,
    pub recorded: &'a str,
}

/// The receipt's bytes: two-space JSON ending in a line break.
pub fn bytes(f: &Facts<'_>) -> Vec<u8> {
    let d = f.definition;
    let e = f.evaluation;
    let stream = |tail: &writfold_runtime::process::Tail| json!({"bytes": tail.total, "tail_bytes": tail.bytes.len(), "tail": sha(&tail.bytes)});
    let mut value = json!({
        "schema": SCHEMA,
        "gate": d.id,
        "version": d.version,
        "class": d.class,
        "definition": {
            "digest": d.digest,
            "origin": d.origin.name(),
            "path": d.path,
            "scope": d.scope,
        },
        "run": f.run,
        "subject": {
            "repo": f.repo,
            "commit": f.commit,
            "scope": f.scoped.digest,
            "paths": f.scoped.paths,
            "dirty": f.scoped.dirty,
            "dirty_digest": f.scoped.dirty_digest,
        },
        "command": {"argv": d.argv, "workdir": d.workdir, "timeout_seconds": d.timeout_seconds},
        "result": {
            "outcome": outcome(e.ending, f.subject_moved).name(),
            "exit_code": e.exit_code,
            "signal": e.signal,
            "timed_out": e.timed_out,
            "error": e.error,
            "subject_moved": f.subject_moved,
            "duration_ms": e.duration_ms,
        },
        "output": {"stdout": stream(&e.stdout), "stderr": stream(&e.stderr)},
        "evaluator": evaluator(),
        "recorded": f.recorded,
    });
    let digest = sha(canonical(&value).as_bytes());
    if let Value::Object(map) = &mut value {
        map.insert("digest".into(), Value::String(digest));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

/// A valid receipt, as read back.
#[derive(Clone, Debug)]
pub struct Receipt {
    pub gate: String,
    pub class: String,
    pub definition: String,
    /// The definition's scope patterns when the receipt was written.
    pub patterns: Vec<String>,
    pub run: String,
    pub repo: String,
    pub commit: String,
    pub scope: String,
    pub paths: u64,
    pub dirty: u64,
    pub argv: Vec<String>,
    pub workdir: String,
    pub timeout_seconds: u64,
    pub outcome: Outcome,
    pub exit_code: Option<i64>,
    pub ending: Ending,
    pub duration_ms: u64,
    pub recorded: String,
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |v, key| v.get(key))
}

fn s(value: &Value, path: &[&str]) -> Result<String, String> {
    at(value, path)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{} is missing or not a one-line string", path.join(".")))
}

fn n(value: &Value, path: &[&str]) -> Result<u64, String> {
    at(value, path)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{} is missing or not a count", path.join(".")))
}

fn maybe_i(value: &Value, path: &[&str]) -> Result<Option<i64>, String> {
    match at(value, path) {
        Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("{} is not an integer", path.join("."))),
        None => Err(format!("{} is missing", path.join("."))),
    }
}

fn flag(value: &Value, path: &[&str]) -> Result<bool, String> {
    at(value, path)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{} is missing or not true/false", path.join(".")))
}

fn strings(value: &Value, path: &[&str]) -> Result<Vec<String>, String> {
    let wrong = || format!("{} is not a non-empty list of strings", path.join("."));
    match at(value, path) {
        Some(Value::Array(items)) if !items.is_empty() => items
            .iter()
            .map(|i| i.as_str().map(str::to_owned).ok_or_else(wrong))
            .collect(),
        _ => Err(wrong()),
    }
}

/// How the receipt says the command ended, if its fields agree.
fn ending(value: &Value) -> Result<Ending, String> {
    let error = !matches!(at(value, &["result", "error"]), Some(Value::Null) | None);
    let timed_out = flag(value, &["result", "timed_out"])?;
    let signal = maybe_i(value, &["result", "signal"])?;
    let code = maybe_i(value, &["result", "exit_code"])?;
    match (error, timed_out, signal, code) {
        (true, false, None, None) => Ok(Ending::Error),
        (false, true, None, None) => Ok(Ending::TimedOut),
        (false, false, Some(_), None) => Ok(Ending::Signaled),
        (false, false, None, Some(code)) => i32::try_from(code)
            .map(Ending::Exited)
            .map_err(|_| "exit_code is out of range".into()),
        _ => Err("result fields disagree about how the command ended".into()),
    }
}

/// Read one receipt of `run`, named `name` (`<gate>-….json`), checking its
/// schema, evaluator, fields, the agreement of its outcome with how the
/// command ended, and its digest. The error says why it is invalid.
pub fn parse(text: &str, run: &str, name: &str) -> Result<Receipt, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    let Some(map) = value.as_object_mut() else {
        return Err("not a JSON object".into());
    };
    let claimed = map.remove("digest");
    if s(&Value::Object(map.clone()), &["schema"])? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    let digest = sha(canonical(&value).as_bytes());
    if claimed.as_ref().and_then(Value::as_str) != Some(digest.as_str()) {
        return Err(
            "not written by writfold gate run (its digest does not match its content)".into(),
        );
    }
    let evaluator = s(&value, &["evaluator"])?;
    if !evaluator.starts_with("writfold ") {
        return Err(format!("evaluator {evaluator} is not writfold"));
    }
    let gate = s(&value, &["gate"])?;
    let receipt_run = s(&value, &["run"])?;
    if receipt_run != run || !name.starts_with(&format!("{gate}-")) {
        return Err("its run or gate does not match where it is filed".into());
    }
    let ending = ending(&value)?;
    let moved = flag(&value, &["result", "subject_moved"])?;
    let stated = Outcome::parse(&s(&value, &["result", "outcome"])?)
        .ok_or("result.outcome is not pass, fail or unknown")?;
    if stated != outcome(ending, moved) {
        return Err("its outcome disagrees with how the command ended".into());
    }
    let argv = strings(&value, &["command", "argv"])?;
    let patterns = strings(&value, &["definition", "scope"])?;
    Ok(Receipt {
        gate,
        class: s(&value, &["class"])?,
        definition: s(&value, &["definition", "digest"])?,
        patterns,
        run: receipt_run,
        repo: s(&value, &["subject", "repo"])?,
        commit: s(&value, &["subject", "commit"])?,
        scope: s(&value, &["subject", "scope"])?,
        paths: n(&value, &["subject", "paths"])?,
        dirty: n(&value, &["subject", "dirty"])?,
        argv,
        workdir: s(&value, &["command", "workdir"])?,
        timeout_seconds: n(&value, &["command", "timeout_seconds"])?,
        outcome: stated,
        exit_code: maybe_i(&value, &["result", "exit_code"])?,
        ending,
        duration_ms: n(&value, &["result", "duration_ms"])?,
        recorded: s(&value, &["recorded"])?,
    })
}

/// A short form of how it ended: `exit=N`, `signal=N`, `timeout` or `error`.
pub fn ended(r: &Receipt) -> String {
    match r.ending {
        Ending::Exited(code) => format!("exit={code}"),
        Ending::Signaled => "signal".into(),
        Ending::TimedOut => "timeout".into(),
        Ending::Error => "error".into(),
    }
}
