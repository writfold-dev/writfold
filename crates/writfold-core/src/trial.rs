//! Pure decisions of the `writfold-trial-worktree/1` edit flow: whether a
//! prepared candidate changes anything, how an interrupted apply is
//! reconciled from the observed bytes, and whether the checks an operator
//! requires are all `entailed` in the retained Weft report.
use crate::bytes::same;
use alloc::vec::Vec;

/// A prepared candidate must differ from its source. Recovery tells "installed"
/// from "never installed" by comparing bytes, which needs the two to differ.
pub fn changes(source: &[u8], candidate: &[u8]) -> bool {
    !same(source, candidate)
}

/// The reconciled outcome of a claim with no recorded result.
pub enum Outcome {
    /// The file holds the candidate's exact bytes: the install happened.
    Applied,
    /// The file holds the source's exact bytes: the install never happened.
    Abandoned,
    /// Anything else: a person decides; nothing is re-dispatched.
    Unknown,
}

/// Decide from the observed file bytes alone; never rewrites anything.
pub fn recovery(observed: &[u8], source: &[u8], candidate: &[u8]) -> Outcome {
    if same(observed, candidate) {
        Outcome::Applied
    } else if same(observed, source) {
        Outcome::Abandoned
    } else {
        Outcome::Unknown
    }
}

/// Weft check results, as `weft.report/1` names them.
pub const ENTAILED: u8 = 0;
pub const REFUTED: u8 = 1;
pub const UNDETERMINED: u8 = 2;
pub const INCONSISTENT_PREMISES: u8 = 3;
pub const UNKNOWN: u8 = 4;
pub const NOT_EVALUATED: u8 = 5;

/// One check of a retained Weft report: its name and result.
pub struct Check {
    pub name: Vec<u8>,
    pub result: u8,
}

/// Why a required check is not satisfied; the index is into the requirements.
pub enum Refusal {
    /// No check of the report has this name.
    Missing(usize),
    /// Some check with this name is not `entailed`.
    NotEntailed(usize),
}

/// One check folded into a name's status: 0 none seen, 1 all entailed so
/// far, 2 at least one not entailed (final).
pub fn fold_status(status: u8, matched: bool, result: u8) -> u8 {
    if !matched || status == 2 {
        status
    } else if result == ENTAILED {
        1
    } else {
        2
    }
}

/// How the report's checks named `name` stand: 0 none, 1 all entailed,
/// 2 at least one not entailed.
pub fn name_status(checks: &[Check], name: &[u8]) -> u8 {
    let mut status = 0;
    let mut i = 0;
    while i < checks.len() {
        status = fold_status(status, same(&checks[i].name, name), checks[i].result);
        i += 1;
    }
    status
}

/// Every required check is present and `entailed`; the first requirement that
/// is not names the refusal. An empty requirement list is satisfied.
pub fn required_entailed(checks: &[Check], required: &[Vec<u8>]) -> Result<(), Refusal> {
    let mut i = 0;
    while i < required.len() {
        let status = name_status(checks, &required[i]);
        if status == 0 {
            return Err(Refusal::Missing(i));
        }
        if status == 2 {
            return Err(Refusal::NotEntailed(i));
        }
        i += 1;
    }
    Ok(())
}
