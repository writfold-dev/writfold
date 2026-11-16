//! Gate obligations behind `writfold kb gate`: what one evaluation's ending
//! means, and the obligation state derived from a gate's receipts and
//! statements. Nothing here reads a repository or a receipt; the caller
//! gathers the facts. State is never stored: it is this function of the
//! definition and the receipts, recomputed on every read. Which paths a
//! scope covers is [`crate::scope`].
//!
//! The decisions `outcome`, `bound`, `current`, `newest_bound`, `about`,
//! `evidence`, `judge`, `state`, `stands`, `statements`, `standing` and
//! `Standing::met` are in the Charon
//! `start_from` list (named one by one, so the derived trait
//! impls and the `name`/`parse` text conveniences are never translated);
//! `proofs/Writfold/Gate.lean` proves them for every input
//! (docs/writfold/aeneas.md). The property tests below state the same
//! properties for small inputs.

use alloc::vec::Vec;

/// How one evaluation ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ending {
    /// The command exited with this status.
    Exited(i32),
    /// It was killed by a signal it did not ask for (a crash).
    Signaled,
    /// It ran past its timeout and was stopped.
    TimedOut,
    /// It could not be started or supervised.
    Error,
}

/// The outcome a receipt records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pass,
    Fail,
    /// A timeout, crash or error: never evidence of success.
    Unknown,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Unknown => "unknown",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pass" => Some(Outcome::Pass),
            "fail" => Some(Outcome::Fail),
            "unknown" => Some(Outcome::Unknown),
            _ => None,
        }
    }
}

/// Only exit status 0 is a pass; any other exit status is a failure; every
/// other ending is unknown. `subject_moved` (an in-scope path changed while
/// the command ran) makes any ending unknown: the result is about no single
/// subject.
pub fn outcome(ending: Ending, subject_moved: bool) -> Outcome {
    if subject_moved {
        return Outcome::Unknown;
    }
    match ending {
        Ending::Exited(0) => Outcome::Pass,
        Ending::Exited(_) => Outcome::Fail,
        Ending::Signaled | Ending::TimedOut | Ending::Error => Outcome::Unknown,
    }
}

/// A gate's obligation on one subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// No receipt.
    Open,
    /// The newest receipt about the current subject passed.
    Satisfied,
    /// Receipts exist, but none is about the current definition and the
    /// current content of the paths in scope.
    Stale,
    /// The newest receipt about the current subject failed.
    Failed,
    /// The newest receipt about the current subject is unknown.
    Unknown,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Open => "open",
            State::Satisfied => "satisfied",
            State::Stale => "stale",
            State::Failed => "failed",
            State::Unknown => "unknown",
        }
    }
}

/// What one valid receipt says, compared with the present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub outcome: Outcome,
    /// The receipt's definition digest equals the current definition's.
    pub same_definition: bool,
    /// The receipt's scope digest equals the present content in scope.
    pub same_scope: bool,
    /// The receipt is this run's, about this subject, and intact
    /// ([`bound`]). A receipt that is not bound counts as absent.
    pub bound: bool,
}

/// Whether a record is bound to the run and subject it is judged for: it
/// is filed in this run, names this repository or candidate, and its
/// digest is intact. All three, or it counts for nothing.
pub fn bound(same_run: bool, same_subject: bool, intact: bool) -> bool {
    same_run && same_subject && intact
}

/// Whether a receipt decides: bound, and about the current definition and
/// the current content in scope.
pub fn current(evidence: &Evidence) -> bool {
    evidence.bound && evidence.same_definition && evidence.same_scope
}

/// The newest bound receipt, if any.
pub fn newest_bound(receipts: &[Evidence]) -> Option<usize> {
    let mut index = receipts.len();
    while index > 0 {
        index -= 1;
        if receipts[index].bound {
            return Some(index);
        }
    }
    None
}

/// The state from a gate's receipts, oldest first, and the index of the
/// receipt that decides it: the newest [`current`] receipt, else the
/// newest bound receipt (which is then stale), else none (`open`). A
/// receipt that is not [`bound`] is treated exactly as an absent one.
pub fn state(receipts: &[Evidence]) -> (State, Option<usize>) {
    let mut index = receipts.len();
    while index > 0 {
        index -= 1;
        let evidence = receipts[index];
        if current(&evidence) {
            let state = match evidence.outcome {
                Outcome::Pass => State::Satisfied,
                Outcome::Fail => State::Failed,
                Outcome::Unknown => State::Unknown,
            };
            return (state, Some(index));
        }
    }
    match newest_bound(receipts) {
        None => (State::Open, None),
        Some(newest) => (State::Stale, Some(newest)),
    }
}

/// Whether a record (a receipt, a statement, a cover row) is about the
/// present: its recorded definition digest and content digest are the
/// present ones, byte for byte.
pub fn about(
    recorded_definition: &[u8],
    recorded_content: &[u8],
    definition: &[u8],
    content: &[u8],
) -> bool {
    crate::bytes::same(recorded_definition, definition)
        && crate::bytes::same(recorded_content, content)
}

/// One valid receipt with the digests it recorded (oldest first, as the
/// caller orders them).
pub struct Recorded {
    pub outcome: Outcome,
    /// The definition digest the receipt was made under.
    pub definition: Vec<u8>,
    /// The content digest (scope or run record) it was made about.
    pub content: Vec<u8>,
    /// Whether it is bound to this run and subject ([`bound`]).
    pub bound: bool,
}

/// A receipt compared with the present: the same definition, and the same
/// content in scope. Empty present content (it could not be read) is
/// never the same content.
pub fn evidence(receipt: &Recorded, definition: &[u8], content: &[u8]) -> Evidence {
    Evidence {
        outcome: receipt.outcome,
        same_definition: crate::bytes::same(&receipt.definition, definition),
        same_scope: !content.is_empty() && crate::bytes::same(&receipt.content, content),
        bound: receipt.bound,
    }
}

/// [`state`] of a gate's receipts, each compared with the present
/// definition and content digests by [`evidence`].
pub fn judge(receipts: &[Recorded], definition: &[u8], content: &[u8]) -> (State, Option<usize>) {
    let mut all: Vec<Evidence> = Vec::new();
    let mut index = 0;
    while index < receipts.len() {
        all.push(evidence(&receipts[index], definition, content));
        index += 1;
    }
    state(&all)
}

/// What stands for an obligation once statements are counted: a checked
/// state, or one of the two statement classes. A statement is never a
/// passing check: `Attested` and `Waived` are their own standings, and no
/// statement ever yields `Checked(Satisfied)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Standing {
    Checked(State),
    /// An attested obligation with a statement about the current subject.
    Attested,
    /// A waiver, with a reason, about the current subject.
    Waived,
}

impl Standing {
    pub fn name(self) -> &'static str {
        match self {
            Standing::Checked(state) => state.name(),
            Standing::Attested => "attested",
            Standing::Waived => "waived",
        }
    }

    /// Whether nothing more is owed: satisfied by a check, attested where
    /// only an attestation is asked for, or waived with a reason.
    pub fn met(self) -> bool {
        !matches!(
            self,
            Standing::Checked(State::Open | State::Stale | State::Failed | State::Unknown)
        )
    }
}

/// The statements recorded for one obligation, compared with the present
/// (the same definition and the same subject content).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Statements {
    /// An attestation about the current subject exists.
    pub attested_current: bool,
    /// Some attestation exists (current or not).
    pub attested_any: bool,
    /// A waiver about the current subject exists.
    pub waived_current: bool,
}

/// One recorded statement: an attestation (`attested`) or a waiver, with
/// the digests it was made about.
pub struct Said {
    pub attested: bool,
    pub definition: Vec<u8>,
    pub content: Vec<u8>,
    /// Whether it is bound to this run and subject ([`bound`]).
    pub bound: bool,
}

/// Whether `said` is an attestation (`attested`) or a waiver (not
/// `attested`), bound to this run and subject, and [`about`] the present
/// definition and content. Its own function so the loop in [`statements`]
/// stays flat for the translation.
pub fn stands(said: &Said, attested: bool, definition: &[u8], content: &[u8]) -> bool {
    if said.attested != attested {
        return false;
    }
    if !said.bound {
        return false;
    }
    about(&said.definition, &said.content, definition, content)
}

/// Whether `said` is a bound attestation, current or not. Its own function
/// so the loop in [`statements`] stays flat for the translation.
pub fn attests(said: &Said) -> bool {
    said.attested && said.bound
}

/// The statements of one obligation compared with the present: an
/// attestation or a waiver stands only when it is bound and [`about`] the
/// present definition and content; `attested_any` counts every bound
/// attestation.
pub fn statements(said: &[Said], definition: &[u8], content: &[u8]) -> Statements {
    let mut attested_current = false;
    let mut attested_any = false;
    let mut waived_current = false;
    let mut index = 0;
    while index < said.len() {
        if attests(&said[index]) {
            attested_any = true;
        }
        if stands(&said[index], true, definition, content) {
            attested_current = true;
        }
        if stands(&said[index], false, definition, content) {
            waived_current = true;
        }
        index += 1;
    }
    Statements {
        attested_current,
        attested_any,
        waived_current,
    }
}

/// The standing of one obligation. An `attested_class` obligation has no
/// check and is met only by a current attestation; every other obligation
/// stands on its checked `state`. A current waiver stands for any
/// obligation that is not already satisfied or attested.
pub fn standing(attested_class: bool, state: State, statements: Statements) -> Standing {
    if attested_class {
        if statements.attested_current {
            return Standing::Attested;
        }
    } else if matches!(state, State::Satisfied) {
        return Standing::Checked(State::Satisfied);
    }
    if statements.waived_current {
        return Standing::Waived;
    }
    match (attested_class, statements.attested_any) {
        (true, true) => Standing::Checked(State::Stale),
        (true, false) => Standing::Checked(State::Open),
        (false, _) => Standing::Checked(state),
    }
}

#[cfg(test)]
mod tests;
