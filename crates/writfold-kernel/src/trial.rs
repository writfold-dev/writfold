//! Trial edit-flow decisions: the typed adapter over `writfold_core::trial`,
//! whose Aeneas translation is proved in `proofs/Writfold/Trial.lean`
//! (`changes_spec`, `recovery_spec`, `recoveryOutcome_unambiguous`,
//! `required_entailed_spec`, `required_ok_iff`), and over
//! `writfold_core::flow`, the edit-flow validator (`proofs/Writfold/Flow.lean`,
//! `validate_spec`).
use writfold_core::flow;
use writfold_core::trial as core;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};

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

/// A retained object: its digest and exact byte length.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blob {
    pub digest: BlobDigest,
    pub bytes: u64,
}

/// One edit-flow record as the validator reads it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step<'a> {
    Prepared {
        path: &'a str,
        source: Blob,
        recipe: Blob,
        candidate: Blob,
        report: Blob,
    },
    Approved {
        grant: Blob,
        approval: Blob,
    },
    Claimed {
        candidate: Blob,
    },
    Applied {
        candidate: Blob,
    },
    Abandoned {
        reason: &'a str,
    },
}

fn core_blob(blob: &Blob) -> flow::Blob {
    flow::Blob {
        digest: *blob.digest.as_digest(),
        bytes: blob.bytes,
    }
}

fn core_step(step: &Step<'_>) -> flow::Event {
    match step {
        Step::Prepared {
            path,
            source,
            recipe,
            candidate,
            report,
        } => flow::Event::Prepared {
            path: path.as_bytes().to_vec(),
            source: core_blob(source),
            recipe: core_blob(recipe),
            candidate: core_blob(candidate),
            report: core_blob(report),
        },
        Step::Approved { grant, approval } => flow::Event::Approved {
            grant: core_blob(grant),
            approval: core_blob(approval),
        },
        Step::Claimed { candidate } => flow::Event::Claimed {
            candidate: core_blob(candidate),
        },
        Step::Applied { candidate } => flow::Event::Applied {
            candidate: core_blob(candidate),
        },
        Step::Abandoned { reason } => flow::Event::Abandoned {
            reason: reason.as_bytes().to_vec(),
        },
    }
}

/// The edit-flow rule for one run (`validate_spec`): `step` carries every
/// blob it names in `closure` and is the next record after `prefix`, exactly
/// `Prepared` → `Approved` → `Claimed` → `Applied` or `Abandoned`, with the
/// claim and `Applied` naming the prepared candidate and a non-empty reason.
pub fn edit_flow(prefix: &[Step<'_>], step: &Step<'_>, closure: &[BlobDigest]) -> bool {
    let prefix: Vec<flow::Event> = prefix.iter().map(core_step).collect();
    let closure: Vec<[u8; 32]> = closure.iter().map(|d| *d.as_digest()).collect();
    flow::validate(&prefix, &core_step(step), &closure)
}
