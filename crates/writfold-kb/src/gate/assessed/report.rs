//! The part of a Weft `weft.report/1` Writfold reads for an assessed gate:
//! that it is complete, about the exact source bytes, and claims neither
//! grounding nor effect authority; each assumption with the citations
//! written directly above it; and each check's result, Weft's basis, and
//! the assumptions Weft used for it.
//!
//! **Citations.** Weft's own construct for evidence is a binding,
//! `claim <name> = citation "<reference>";` (or `asset`, `rule`). Weft
//! records a binding's reference but gives an assumption no way to name
//! one, so Writfold reads the source order from the report: the bindings
//! declared directly above an `assume` (after the previous assumption or
//! check) are that assumption's citations. A binding followed by a check
//! or by the end of the review cites nothing and is refused.
//!
//! **Premises.** Each check lists the assumptions Weft used for it
//! (`hypotheses`: every preceding assumption, conservatively). Writfold
//! takes that list as the check's premises; nothing narrower is claimed.
use serde_json::Value;

/// One assumption and the references cited above it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assumption {
    pub name: String,
    /// `(binding name, reference)`, in source order.
    pub citations: Vec<(String, String)>,
}

/// One check as Weft reported it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Check {
    pub name: String,
    /// Weft's result, verbatim.
    pub result: String,
    /// Weft's own basis, verbatim (`hypothetical` for every local report).
    pub weft_basis: String,
    /// The assumptions Weft used, by name.
    pub premises: Vec<String>,
}

/// What Writfold takes from a report.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    pub assumptions: Vec<Assumption>,
    pub checks: Vec<Check>,
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("the Weft report has no {key}"))
}

fn list<'a>(value: Option<&'a Value>, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .and_then(|v| v.get(key))
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the Weft report has no {key} list"))
}

fn id(value: &Value) -> Result<u64, String> {
    value
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "a Weft declaration has no id".into())
}

/// Why the report cannot be used, naming Weft's first diagnostic.
fn incomplete(root: &Value, execution: &str) -> String {
    let first = root
        .get("diagnostics")
        .and_then(Value::as_array)
        .and_then(|d| d.first())
        .map(|d| {
            format!(
                ": {} {}",
                d.get("code").and_then(Value::as_str).unwrap_or("?"),
                d.get("message").and_then(Value::as_str).unwrap_or("")
            )
        })
        .unwrap_or_default();
    format!("Weft's execution is {execution}, not complete{first}")
}

/// Assumptions with the bindings declared directly above each.
fn assumptions(review: &Value) -> Result<Vec<Assumption>, String> {
    let references: Vec<(u64, String)> = list(Some(review), "bindings")?
        .iter()
        .map(|b| Ok((id(b)?, text(b, "reference")?.to_owned())))
        .collect::<Result<_, String>>()?;
    let mut out = Vec::new();
    let mut pending: Vec<(String, String)> = Vec::new();
    for d in list(Some(review), "declarations")? {
        let name = text(d, "name")?.to_owned();
        match text(d, "kind")? {
            "claim" | "asset" | "rule" => {
                let at = id(d)?;
                let reference = references
                    .iter()
                    .find(|(i, _)| *i == at)
                    .map(|(_, r)| r.clone())
                    .ok_or_else(|| format!("binding {name} has no reference"))?;
                pending.push((name, reference));
            }
            "hypothesis" => out.push(Assumption {
                name,
                citations: std::mem::take(&mut pending),
            }),
            "check" if !pending.is_empty() => break,
            _ => {}
        }
    }
    match pending.first() {
        Some((name, _)) => Err(format!(
            "citation {name} is not directly above an assumption: declare each citation just before the assume it supports"
        )),
        None => Ok(out),
    }
}

/// Read a report Weft printed for a source whose SHA-256 (bare hex) is
/// `source`.
pub fn parse(bytes: &[u8], source: &str) -> Result<Report, String> {
    let root: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("the Weft report is not JSON ({e})"))?;
    if text(&root, "schema")? != "weft.report/1" {
        return Err("not a weft.report/1 document".into());
    }
    for key in ["writfold_grounded", "effect_authority"] {
        if root.get(key) != Some(&Value::Bool(false)) {
            return Err(format!(
                "the Weft report claims {key}; local reports never do"
            ));
        }
    }
    let execution = text(&root, "execution")?;
    if execution != "complete" {
        return Err(incomplete(&root, execution));
    }
    let read = root.get("source").map(|s| text(s, "sha256"));
    if read.and_then(Result::ok) != Some(source) {
        return Err("the Weft report is about other source bytes".into());
    }
    let review = root.get("review").ok_or("the Weft report has no review")?;
    let names: Vec<(u64, String)> = list(Some(review), "hypotheses")?
        .iter()
        .map(|h| Ok((id(h)?, text(h, "name")?.to_owned())))
        .collect::<Result<_, String>>()?;
    let checks = list(Some(&root), "checks")?
        .iter()
        .map(|c| {
            let premises = c
                .get("hypotheses")
                .and_then(Value::as_array)
                .ok_or("a Weft check has no hypotheses list")?
                .iter()
                .map(|h| {
                    let at = h.as_u64().ok_or("a hypothesis id is not a number")?;
                    names
                        .iter()
                        .find(|(i, _)| *i == at)
                        .map(|(_, n)| n.clone())
                        .ok_or_else(|| format!("hypothesis {at} is not declared"))
                })
                .collect::<Result<_, String>>()?;
            Ok(Check {
                name: text(c, "name")?.to_owned(),
                result: text(c, "result")?.to_owned(),
                weft_basis: text(c, "basis")?.to_owned(),
                premises,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Report {
        assumptions: assumptions(review)?,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn report(declarations: Value, checks: Value) -> Vec<u8> {
        json!({
            "schema": "weft.report/1", "execution": "complete",
            "writfold_grounded": false, "effect_authority": false,
            "source": {"sha256": "ab"},
            "review": {
                "bindings": [{"id": 1, "name": "c", "kind": "claim", "reference": "gate:unit"}],
                "hypotheses": [{"id": 2, "name": "h"}],
                "declarations": declarations,
            },
            "checks": checks,
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn citations_attach_to_the_assumption_below_them() -> Result<(), String> {
        let decl = json!([
            {"id": 0, "name": "change", "kind": "target"},
            {"id": 1, "name": "c", "kind": "claim"},
            {"id": 2, "name": "h", "kind": "hypothesis"},
            {"id": 3, "name": "k", "kind": "check"},
        ]);
        let checks = json!([{"name": "k", "result": "entailed", "basis": "hypothetical", "hypotheses": [2]}]);
        let r = parse(&report(decl, checks.clone()), "ab")?;
        assert_eq!(
            r.assumptions[0].citations,
            vec![("c".into(), "gate:unit".into())]
        );
        assert_eq!(r.checks[0].premises, vec!["h".to_owned()]);
        assert!(parse(&report(json!([]), checks.clone()), "cd").is_err());
        // A citation above a check, or at the end, cites nothing.
        let dangling = json!([
            {"id": 2, "name": "h", "kind": "hypothesis"},
            {"id": 1, "name": "c", "kind": "claim"},
            {"id": 3, "name": "k", "kind": "check"},
        ]);
        let refused = parse(&report(dangling, checks), "ab");
        assert!(refused.is_err_and(|e| e.contains("citation c")));
        Ok(())
    }
}
