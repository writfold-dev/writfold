//! Trial edit-flow decisions: the typed adapter over `writfold_core::trial`,
//! whose Aeneas translation is proved in `proofs/Writfold/Trial.lean`
//! (`changes_spec`, `recovery_spec`, `recoveryOutcome_unambiguous`,
//! `required_entailed_spec`, `required_ok_iff`).
use writfold_core::trial as core;

/// Whether a prepared candidate changes its source. `prepare` refuses a
/// candidate that does not: recovery could not tell installed from not.
pub fn changes(source: &[u8], candidate: &[u8]) -> bool {
    core::changes(source, candidate)
}

/// How a claim with no recorded result is reconciled from the file's bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Applied,
    Abandoned,
    Unknown,
}

pub fn recovery(observed: &[u8], source: &[u8], candidate: &[u8]) -> Outcome {
    match core::recovery(observed, source, candidate) {
        core::Outcome::Applied => Outcome::Applied,
        core::Outcome::Abandoned => Outcome::Abandoned,
        core::Outcome::Unknown => Outcome::Unknown,
    }
}

/// A Weft check result, as `weft.report/1` names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckResult {
    Entailed,
    Refuted,
    Undetermined,
    InconsistentPremises,
    Unknown,
    NotEvaluated,
}

impl CheckResult {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "entailed" => Self::Entailed,
            "refuted" => Self::Refuted,
            "undetermined" => Self::Undetermined,
            "inconsistent-premises" => Self::InconsistentPremises,
            "unknown" => Self::Unknown,
            "not-evaluated" => Self::NotEvaluated,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entailed => "entailed",
            Self::Refuted => "refuted",
            Self::Undetermined => "undetermined",
            Self::InconsistentPremises => "inconsistent-premises",
            Self::Unknown => "unknown",
            Self::NotEvaluated => "not-evaluated",
        }
    }
    fn code(self) -> u8 {
        match self {
            Self::Entailed => core::ENTAILED,
            Self::Refuted => core::REFUTED,
            Self::Undetermined => core::UNDETERMINED,
            Self::InconsistentPremises => core::INCONSISTENT_PREMISES,
            Self::Unknown => core::UNKNOWN,
            Self::NotEvaluated => core::NOT_EVALUATED,
        }
    }
}

/// One check of a retained Weft report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Check {
    pub name: String,
    pub result: CheckResult,
}

/// The first required check that is not satisfied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Unsatisfied {
    /// No check of the report has this name.
    Missing(String),
    /// Some check with this name is not `entailed`.
    NotEntailed(String),
}

/// Every required check is present and every check of that name `entailed`.
pub fn required_entailed(checks: &[Check], required: &[String]) -> Result<(), Unsatisfied> {
    let core_checks: Vec<core::Check> = checks
        .iter()
        .map(|c| core::Check {
            name: c.name.as_bytes().to_vec(),
            result: c.result.code(),
        })
        .collect();
    let names: Vec<Vec<u8>> = required.iter().map(|r| r.as_bytes().to_vec()).collect();
    core::required_entailed(&core_checks, &names).map_err(|refusal| match refusal {
        core::Refusal::Missing(i) => {
            Unsatisfied::Missing(required.get(i).cloned().unwrap_or_default())
        }
        core::Refusal::NotEntailed(i) => {
            Unsatisfied::NotEntailed(required.get(i).cloned().unwrap_or_default())
        }
    })
}
