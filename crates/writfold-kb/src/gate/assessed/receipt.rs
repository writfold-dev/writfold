//! Assessed receipts: one JSON file per Weft assessment,
//! `runs/<run>/assessed/<gate>-<UTC compact time>-<6 hex>.json`, written
//! create-new through the write protocol by `gate run` and by nothing
//! else. A receipt binds the gate's definition digest, the Weft source's
//! digest, the pinned and the actual binary digests, the report's digest,
//! each assumption's citations with what they stood on when it was
//! written, and each check's Weft result, Weft's own basis, Writfold's
//! basis and whether it was met. It ends with `digest`, the SHA-256 of the
//! rest in canonical form.
//!
//! Reading a receipt re-derives every assumption's grounding, every
//! check's basis and met flag, and the outcome from the recorded facts
//! with the kernel ([`writfold_core::assess`]); a receipt whose stated
//! verdicts disagree reads as invalid, so a hypothetical result cannot be
//! relabelled grounded by editing the file.
use super::super::receipt::canonical;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use writfold_core::assess::{self, Basis, Grounding, Required, Verdict};
use writfold_core::gate::Outcome;

pub const SCHEMA: &str = "writfold-assessed-receipt/1";
/// The directory of a run that holds assessed receipts.
pub const DIR: &str = "assessed";

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// One citation, as recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cited {
    pub binding: String,
    pub reference: String,
    pub grounding: Grounding,
    pub standing: String,
    pub path: Option<String>,
}

/// One assumption, as recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assumed {
    pub name: String,
    pub grounding: Grounding,
    pub citations: Vec<Cited>,
}

/// One check, as recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Judged {
    pub name: String,
    pub required: bool,
    pub verdict: Verdict,
    pub weft_basis: String,
    pub basis: Basis,
    pub premises: Vec<String>,
    pub met: bool,
}

/// A receipt's content, written or read back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assessed {
    pub gate: String,
    pub version: i64,
    pub definition: String,
    pub origin: String,
    pub path: String,
    pub run: String,
    pub repo: String,
    pub commit: String,
    /// As written in the definition (`{gate}/…` or `{run}/…`).
    pub weft: String,
    pub source: String,
    pub pin: String,
    pub binary: Option<String>,
    pub report: Option<String>,
    pub assumptions: Vec<Assumed>,
    pub checks: Vec<Judged>,
    /// The required checks the report did not have.
    pub missing: Vec<String>,
    pub outcome: Outcome,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub evaluator: String,
    pub recorded: String,
}

impl Assessed {
    /// The outcome the kernel derives from the recorded facts.
    pub fn derived(&self) -> Outcome {
        let mut required: Vec<Required> = self
            .checks
            .iter()
            .filter(|c| c.required)
            .map(|c| Required::Found(c.verdict, c.basis))
            .collect();
        required.extend(self.missing.iter().map(|_| Required::Missing));
        assess::outcome(self.error.is_none(), &required)
    }
}

fn citation(c: &Cited) -> Value {
    json!({"binding": c.binding, "reference": c.reference, "grounding": c.grounding.name(),
           "standing": c.standing, "path": c.path})
}

/// The receipt's bytes: two-space JSON ending in a line break.
pub fn bytes(a: &Assessed) -> Vec<u8> {
    let assumptions: Vec<Value> = a
        .assumptions
        .iter()
        .map(|s| {
            json!({"name": s.name, "grounding": s.grounding.name(),
                        "citations": s.citations.iter().map(citation).collect::<Vec<_>>()})
        })
        .collect();
    let checks: Vec<Value> = a
        .checks
        .iter()
        .map(|c| {
            json!({"name": c.name, "required": c.required, "result": c.verdict.name(),
                        "weft_basis": c.weft_basis, "basis": c.basis.name(),
                        "premises": c.premises, "met": c.met})
        })
        .collect();
    let mut value = json!({
        "schema": SCHEMA,
        "gate": a.gate, "version": a.version, "class": "assessed",
        "definition": {"digest": a.definition, "origin": a.origin, "path": a.path},
        "run": a.run,
        "subject": {"repo": a.repo, "commit": a.commit},
        "weft": {"source": {"weft": a.weft, "digest": a.source},
                 "binary": {"pin": a.pin, "digest": a.binary},
                 "report": a.report},
        "assumptions": assumptions,
        "checks": checks,
        "missing": a.missing,
        "result": {"outcome": a.outcome.name(), "error": a.error, "duration_ms": a.duration_ms},
        "evaluator": a.evaluator,
        "recorded": a.recorded,
    });
    let digest = sha(canonical(&value).as_bytes());
    if let Value::Object(map) = &mut value {
        map.insert("digest".into(), Value::String(digest));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |v, key| v.get(key))
}

fn s(value: &Value, path: &[&str]) -> Result<String, String> {
    at(value, path)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("{} is missing or not a string", path.join(".")))
}

fn maybe(value: &Value, path: &[&str]) -> Result<Option<String>, String> {
    match at(value, path) {
        Some(Value::Null) => Ok(None),
        _ => s(value, path).map(Some),
    }
}

fn items<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} is not a list"))
}

fn names(value: &Value, key: &str) -> Result<Vec<String>, String> {
    items(value, key)?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or(format!("{key} holds a non-string"))
        })
        .collect()
}

fn parse_assumption(v: &Value) -> Result<Assumed, String> {
    let citations = items(v, "citations")?
        .iter()
        .map(|c| {
            Ok(Cited {
                binding: s(c, &["binding"])?,
                reference: s(c, &["reference"])?,
                grounding: Grounding::parse(&s(c, &["grounding"])?).ok_or("unknown grounding")?,
                standing: s(c, &["standing"])?,
                path: maybe(c, &["path"])?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let groundings: Vec<Grounding> = citations.iter().map(|c| c.grounding).collect();
    let stated = Grounding::parse(&s(v, &["grounding"])?).ok_or("unknown grounding")?;
    if stated != assess::assumption(&groundings) {
        return Err("an assumption's grounding disagrees with its citations".into());
    }
    Ok(Assumed {
        name: s(v, &["name"])?,
        grounding: stated,
        citations,
    })
}

fn parse_check(v: &Value, assumptions: &[Assumed]) -> Result<Judged, String> {
    let premises = names(v, "premises")?;
    let groundings = premises
        .iter()
        .map(|p| {
            assumptions
                .iter()
                .find(|a| &a.name == p)
                .map(|a| a.grounding)
                .ok_or(format!("premise {p} is not a recorded assumption"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let verdict = Verdict::parse(&s(v, &["result"])?).ok_or("unknown check result")?;
    let basis = Basis::parse(&s(v, &["basis"])?).ok_or("unknown basis")?;
    let met = at(v, &["met"])
        .and_then(Value::as_bool)
        .ok_or("met is missing")?;
    if basis != assess::basis(&groundings) || met != assess::met(verdict, basis) {
        return Err("a check's basis or met flag disagrees with its premises".into());
    }
    Ok(Judged {
        name: s(v, &["name"])?,
        required: at(v, &["required"])
            .and_then(Value::as_bool)
            .ok_or("required is missing")?,
        verdict,
        weft_basis: s(v, &["weft_basis"])?,
        basis,
        premises,
        met,
    })
}

/// Read one receipt of `run` named `name`, checking its schema, digest,
/// evaluator, filing, and that every stated verdict is the one the kernel
/// derives. The error says why it is invalid.
pub fn parse(text: &str, run: &str, name: &str) -> Result<Assessed, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value
        .as_object_mut()
        .ok_or("not a JSON object")?
        .remove("digest");
    if s(&value, &["schema"])? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    if claimed.as_ref().and_then(Value::as_str) != Some(sha(canonical(&value).as_bytes()).as_str())
    {
        return Err(
            "not written by writfold gate run (its digest does not match its content)".into(),
        );
    }
    let evaluator = s(&value, &["evaluator"])?;
    let gate = s(&value, &["gate"])?;
    if !evaluator.starts_with("writfold ")
        || s(&value, &["run"])? != run
        || !name.starts_with(&format!("{gate}-"))
    {
        return Err("its evaluator, run or gate does not match where it is filed".into());
    }
    let assumptions = items(&value, "assumptions")?
        .iter()
        .map(parse_assumption)
        .collect::<Result<Vec<_>, String>>()?;
    let checks = items(&value, "checks")?
        .iter()
        .map(|c| parse_check(c, &assumptions))
        .collect::<Result<Vec<_>, String>>()?;
    let a = Assessed {
        gate,
        version: at(&value, &["version"])
            .and_then(Value::as_i64)
            .ok_or("version is missing")?,
        definition: s(&value, &["definition", "digest"])?,
        origin: s(&value, &["definition", "origin"])?,
        path: s(&value, &["definition", "path"])?,
        run: run.to_owned(),
        repo: s(&value, &["subject", "repo"])?,
        commit: s(&value, &["subject", "commit"])?,
        weft: s(&value, &["weft", "source", "weft"])?,
        source: s(&value, &["weft", "source", "digest"])?,
        pin: s(&value, &["weft", "binary", "pin"])?,
        binary: maybe(&value, &["weft", "binary", "digest"])?,
        report: maybe(&value, &["weft", "report"])?,
        assumptions,
        checks,
        missing: names(&value, "missing")?,
        outcome: Outcome::parse(&s(&value, &["result", "outcome"])?).ok_or("unknown outcome")?,
        error: maybe(&value, &["result", "error"])?,
        duration_ms: at(&value, &["result", "duration_ms"])
            .and_then(Value::as_u64)
            .ok_or("duration_ms is missing")?,
        evaluator,
        recorded: s(&value, &["recorded"])?,
    };
    if a.outcome != a.derived() {
        return Err("its outcome disagrees with its checks".into());
    }
    if a.outcome == Outcome::Pass && a.binary.as_deref() != Some(a.pin.as_str()) {
        return Err("a pass from a binary other than the pinned one".into());
    }
    Ok(a)
}
