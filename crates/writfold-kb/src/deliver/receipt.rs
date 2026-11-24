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
    /// One more than the highest `seq` among the candidate's receipts of
    /// this check: the judging order, with no clock in it.
    pub seq: u64,
    pub recorded: &'a str,
}

/// The receipt's bytes: two-space JSON ending in a line break.
fn listed(found: &[Finding]) -> Vec<Value> {
    (found.iter().take(MAX_FINDINGS))
        .map(|x| json!({"line": x.line, "rule": x.rule, "detail": x.detail}))
        .collect()
}

pub fn bytes(f: &Facts<'_>) -> Vec<u8> {
    let e = f.evaluated;
    let findings = listed(&e.findings);
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
        "seq": f.seq,
        "recorded": f.recorded,
    });
    // Advice only when there is some, so a receipt without it keeps the
    // bytes it always had.
    if let (false, Some(Value::Object(result))) = (e.advice.is_empty(), value.get_mut("result")) {
        result.insert("advice".into(), Value::Array(listed(&e.advice)));
        result.insert("advice_total".into(), json!(e.advice.len()));
    }
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
    /// Advice: shown and recorded, never deciding ([`writfold_core::prose`]).
    pub advice: Vec<Finding>,
    pub advice_total: u64,
    pub note: Option<String>,
    /// 0 for a receipt written before receipts carried `seq`.
    pub seq: u64,
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
    found(value, "/result/findings")
}

fn found(value: &Value, pointer: &str) -> Result<Vec<Finding>, String> {
    let items = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{pointer} is not a list"))?;
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
    let counted = usize::try_from(total).unwrap_or(usize::MAX);
    if !writfold_core::prose::agrees(outcome, counted) || (findings.len() as u64) > total {
        return Err("its outcome disagrees with its findings".into());
    }
    let (advice, advice_total) = match value.pointer("/result/advice") {
        None => (Vec::new(), 0),
        Some(_) => (
            found(&value, "/result/advice")?,
            (value
                .pointer("/result/advice_total")
                .and_then(Value::as_u64))
            .ok_or("result.advice_total is not a count")?,
        ),
    };
    if (advice.len() as u64) > advice_total {
        return Err("its advice disagrees with its advice count".into());
    }
    Ok(Receipt {
        class: text(&value, "/class")?,
        definition: text(&value, "/definition")?,
        candidate: candidate.to_owned(),
        content: text(&value, "/candidate/content")?,
        outcome,
        findings,
        total,
        advice,
        advice_total,
        note: text(&value, "/result/note").ok(),
        seq: match value.get("seq") {
            None => 0,
            Some(v) => v.as_u64().filter(|n| *n > 0).ok_or("seq is not a count")?,
        },
        recorded: text(&value, "/recorded")?,
        check,
    })
}

/// The order receipts are judged in, oldest first: by `seq`, then by file
/// name ([`writfold_core::order::cmp`]). No clock enters: `recorded` is
/// shown, never compared. A receipt written before `seq` existed reads as
/// 0, so it comes first, in name order (its name carries the time it was
/// written).
pub fn order(a: (&Filed, &Receipt), b: (&Filed, &Receipt)) -> std::cmp::Ordering {
    writfold_core::order::cmp((a.1.seq, &a.0.name), (b.1.seq, &b.0.name))
}

/// One more than the highest `seq` among `filed`'s valid receipts of
/// `check`.
pub fn next_seq(filed: &[Filed], check: &str) -> u64 {
    (filed.iter())
        .filter_map(|f| f.parsed.as_ref().ok())
        .filter(|r| r.check == check)
        .map(|r| r.seq)
        .max()
        .unwrap_or(0)
        + 1
}

/// One receipt file of a candidate.
pub struct Filed {
    pub path: String,
    pub name: String,
    pub parsed: Result<Receipt, String>,
}

/// Every receipt file under `<candidate dir>/checks/`, by name.
pub fn filed(root: &std::path::Path, dir: &str, run: &str, candidate: &str) -> Vec<Filed> {
    let (names, fault) =
        crate::gate::discover::evidence(&root.join(dir).join(DIR), crate::gate::discover::list);
    let mut filed: Vec<Filed> = names
        .into_iter()
        .map(|name| {
            let path = format!("{dir}/{DIR}/{name}");
            let parsed = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|t| parse(&t, run, candidate, &name));
            Filed { path, name, parsed }
        })
        .collect();
    filed.extend(fault.map(|why| Filed {
        path: format!("{dir}/{DIR}"),
        name: String::new(),
        parsed: Err(crate::gate::discover::unlisted_evidence(&why)),
    }));
    filed
}
