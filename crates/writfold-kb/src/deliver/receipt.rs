//! Delivery check receipts: one JSON file per check evaluation,
//! `runs/<run>/deliveries/<candidate>/checks/<check>-<time>-<hex>.json`,
//! written create-new through the write protocol by `deliver check` and by
//! nothing else. A receipt binds the check, the digest of what the check
//! means (its configuration), and the digest of the candidate bytes it
//! read; it is about those bytes only. Like a gate receipt it ends in a
//! digest over its canonical form: it resists mistakes, not forgers.
use super::checks::{Evaluated, Finding, MAX_FINDINGS};
use crate::gate::receipt::{canonical, evaluator};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use writfold_core::gate::Outcome;

pub const SCHEMA: &str = "writfold-delivery-receipt/1";
/// The directory of a candidate that holds its check receipts.
pub const DIR: &str = "checks";

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// Everything one receipt records, before its digest.
pub struct Facts<'a> {
    pub check: &'a str,
    pub class: &'a str,
    pub definition: &'a str,
    pub run: &'a str,
    pub candidate: &'a str,
    /// The digest of the candidate bytes the check read.
    pub content: &'a str,
    pub evaluated: &'a Evaluated,
    pub recorded: &'a str,
}

/// The receipt's bytes: two-space JSON ending in a line break.
pub fn bytes(f: &Facts<'_>) -> Vec<u8> {
    let e = f.evaluated;
    let findings: Vec<Value> = e
        .findings
        .iter()
        .take(MAX_FINDINGS)
        .map(|x| json!({"line": x.line, "rule": x.rule, "detail": x.detail}))
        .collect();
    let mut value = json!({
        "schema": SCHEMA,
        "check": f.check,
        "class": f.class,
        "definition": f.definition,
        "run": f.run,
        "candidate": {"id": f.candidate, "content": f.content},
        "result": {
            "outcome": e.outcome.name(),
            "findings": findings,
            "findings_total": e.findings.len(),
            "note": e.note,
        },
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
    pub check: String,
    pub class: String,
    pub definition: String,
    pub candidate: String,
    pub content: String,
    pub outcome: Outcome,
    pub findings: Vec<Finding>,
    pub total: u64,
    pub note: Option<String>,
    pub recorded: String,
}

fn text(value: &Value, pointer: &str) -> Result<String, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{pointer} is missing or not a one-line string"))
}

fn findings(value: &Value) -> Result<Vec<Finding>, String> {
    let items = value
        .pointer("/result/findings")
        .and_then(Value::as_array)
        .ok_or("result.findings is not a list")?;
    items
        .iter()
        .map(|item| {
            Ok(Finding {
                line: item
                    .get("line")
                    .and_then(Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("a finding has no line")?,
                rule: text(item, "/rule")?,
                detail: text(item, "/detail")?,
            })
        })
        .collect()
}

/// Read one receipt of candidate `candidate` in `run`, filed as `name`.
pub fn parse(text_in: &str, run: &str, candidate: &str, name: &str) -> Result<Receipt, String> {
    let mut value: Value = serde_json::from_str(text_in).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value
        .as_object_mut()
        .ok_or("not a JSON object")?
        .remove("digest");
    if text(&value, "/schema")? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    if claimed.as_ref().and_then(Value::as_str) != Some(sha(canonical(&value).as_bytes()).as_str())
    {
        return Err(
            "not written by writfold deliver check (its digest does not match its content)".into(),
        );
    }
    if !text(&value, "/evaluator")?.starts_with("writfold ") {
        return Err("its evaluator is not writfold".into());
    }
    let check = text(&value, "/check")?;
    if text(&value, "/run")? != run
        || text(&value, "/candidate/id")? != candidate
        || !name.starts_with(&format!("{check}-"))
    {
        return Err("its run, candidate or check does not match where it is filed".into());
    }
    let outcome =
        Outcome::parse(&text(&value, "/result/outcome")?).ok_or("result.outcome is unknown")?;
    let findings = findings(&value)?;
    let total = value
        .pointer("/result/findings_total")
        .and_then(Value::as_u64)
        .ok_or("result.findings_total is not a count")?;
    let consistent = match outcome {
        Outcome::Pass => total == 0,
        Outcome::Fail => total > 0,
        Outcome::Unknown => true,
    };
    if !consistent || (findings.len() as u64) > total {
        return Err("its outcome disagrees with its findings".into());
    }
    Ok(Receipt {
        class: text(&value, "/class")?,
        definition: text(&value, "/definition")?,
        candidate: candidate.to_owned(),
        content: text(&value, "/candidate/content")?,
        outcome,
        findings,
        total,
        note: text(&value, "/result/note").ok(),
        recorded: text(&value, "/recorded")?,
        check,
    })
}

/// One receipt file of a candidate.
pub struct Filed {
    pub path: String,
    pub name: String,
    pub parsed: Result<Receipt, String>,
}

/// Every receipt file under `<candidate dir>/checks/`, by name.
pub fn filed(root: &std::path::Path, dir: &str, run: &str, candidate: &str) -> Vec<Filed> {
    let Ok(entries) = std::fs::read_dir(root.join(dir).join(DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json") && !n.starts_with('.'))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let path = format!("{dir}/{DIR}/{name}");
            let parsed = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|t| parse(&t, run, candidate, &name));
            Filed { path, name, parsed }
        })
        .collect()
}
