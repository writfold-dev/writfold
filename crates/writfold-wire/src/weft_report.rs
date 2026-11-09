//! The part of a Weft `weft.report/1` document Writfold reads: whether it is
//! a complete local report that claims no grounding or effect authority, and
//! its checks' names and results. The report's exact bytes stay retained and
//! bound by digest; this reading never upgrades a hypothetical result.
use crate::{Error, Limits, RawJson};
use serde_json::Value;
use writfold_kernel::trial::{Check, CheckResult};

/// A complete Weft report's checks, in report order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeftReport {
    pub kind: String,
    pub scope: String,
    pub checks: Vec<Check>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error(transparent)]
    Json(#[from] Error),
    #[error("not a weft.report/1 document: {0}")]
    Shape(&'static str),
    #[error("the Weft report's execution is {0:?}, not complete")]
    Incomplete(String),
    #[error("the Weft report claims {0}; local reports never do")]
    Claims(&'static str),
}

fn text<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, ReportError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(ReportError::Shape(field))
}

/// Parse and check a report: exact schema, `execution: complete`,
/// `writfold_grounded: false`, `effect_authority: false`, and every check a
/// named finding with a known result.
pub fn parse_report(bytes: &[u8], limits: Limits) -> Result<WeftReport, ReportError> {
    let raw = RawJson::parse(bytes, limits)?;
    let root = raw.value();
    if text(root, "schema")? != "weft.report/1" {
        return Err(ReportError::Shape("schema"));
    }
    let execution = text(root, "execution")?;
    if execution != "complete" {
        return Err(ReportError::Incomplete(execution.to_owned()));
    }
    if root.get("writfold_grounded") != Some(&Value::Bool(false)) {
        return Err(ReportError::Claims("Writfold grounding"));
    }
    if root.get("effect_authority") != Some(&Value::Bool(false)) {
        return Err(ReportError::Claims("effect authority"));
    }
    let checks = root
        .get("checks")
        .and_then(Value::as_array)
        .ok_or(ReportError::Shape("checks"))?
        .iter()
        .map(|check| {
            Ok(Check {
                name: text(check, "name")?.to_owned(),
                result: CheckResult::parse(text(check, "result")?)
                    .ok_or(ReportError::Shape("check result"))?,
            })
        })
        .collect::<Result<_, ReportError>>()?;
    Ok(WeftReport {
        kind: text(root, "kind")?.to_owned(),
        scope: text(root, "scope")?.to_owned(),
        checks,
    })
}
