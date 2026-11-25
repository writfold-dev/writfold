//! Vale for documents (`prose.vale`, #117, #131, RFC
//! 2026-11-25-vale-for-documents): what one run of the user's `vale`
//! executable means for a delivery check.
//!
//! - [`weigh`] turns a Vale severity into advice or blocking under the
//!   mode's threshold (`vale_block`).
//! - [`usable`] says whether a run can be read at all: the pinned version
//!   ran, its output is whole, and its exit status agrees with its alerts.
//! - [`rules`] is the number of enabled checks, or 0 when the canary line
//!   drew no alert (a config that matched nothing).
//! - [`settle`] is `unknown` unless the run is usable, and otherwise
//!   [`crate::prose::verdict`].
//!
//! Probing the version, running the process, parsing its JSON and hashing
//! the profile tree are the caller's (tested). In the Charon `start_from`
//! list; `proofs/Writfold/Vale.lean` proves every function for every input
//! (docs/writfold/aeneas.md).
use crate::gate::{Ending, Outcome};
use crate::prose::{Weight, verdict};

/// A Vale alert's severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Level {
    Suggestion,
    Warning,
    Error,
}

/// The lowest severity that blocks (a mode's `vale_block`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockFrom {
    /// Nothing blocks; every alert is advice.
    Never,
    Error,
    Warning,
    Suggestion,
}

/// What the `vale --version` probe found, against the profile's pin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tool {
    /// No executable could be run, or it printed no version.
    Absent,
    /// Another version than the pinned one.
    Other,
    /// Exactly the pinned version.
    Pinned,
}

/// The facts of one run, gathered by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ran {
    pub tool: Tool,
    pub ending: Ending,
    /// The output was under the cap, parsed as Vale's JSON, every severity
    /// was known and every alert was about one of the two files given.
    pub whole: bool,
    /// Error-level alerts in the whole output (document and canary).
    pub errors: usize,
    /// Checks Vale reported as loaded (`counts`).
    pub enabled: usize,
    /// Alerts on the canary file.
    pub canary: usize,
}

/// Whether an alert of `level` blocks under `from`.
pub fn weigh(level: Level, from: BlockFrom) -> Weight {
    match from {
        BlockFrom::Never => Weight::Advice,
        BlockFrom::Error => match level {
            Level::Error => Weight::Blocking,
            Level::Warning | Level::Suggestion => Weight::Advice,
        },
        BlockFrom::Warning => match level {
            Level::Error | Level::Warning => Weight::Blocking,
            Level::Suggestion => Weight::Advice,
        },
        BlockFrom::Suggestion => Weight::Blocking,
    }
}

/// Whether a run can be read: the pinned version, whole output, and exit 0
/// with no error-level alert or exit 1 with at least one.
pub fn usable(ran: Ran) -> bool {
    match ran.tool {
        Tool::Pinned => {}
        Tool::Absent | Tool::Other => return false,
    }
    if !ran.whole {
        return false;
    }
    match ran.ending {
        Ending::Exited(0) => ran.errors == 0,
        Ending::Exited(1) => ran.errors > 0,
        Ending::Exited(_) | Ending::Signaled | Ending::TimedOut | Ending::Error => false,
    }
}

/// The rules a run applied: the enabled checks, or none when the canary
/// drew no alert.
pub fn rules(enabled: usize, canary: usize) -> usize {
    if canary == 0 { 0 } else { enabled }
}

/// The outcome of a `prose.vale` check: `unknown` unless the run is
/// usable, else the prose verdict over the weighed document alerts.
pub fn settle(ran: Ran, found: &[Weight]) -> Outcome {
    if !usable(ran) {
        return Outcome::Unknown;
    }
    verdict(rules(ran.enabled, ran.canary), found)
}

#[cfg(test)]
mod tests;
