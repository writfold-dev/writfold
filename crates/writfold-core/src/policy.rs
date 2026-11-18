//! A team's policy over the gate kernel's standings: which standings meet
//! a gate (`accept`), whether a run's policy moved since it was pinned,
//! how each step of a plan was closed, and one word for how a plan ended.
//! Nothing here reads a definition, a pin or a record; the caller gathers
//! the facts. The standing itself is still [`crate::gate::standing`], shown
//! as it is; this module only decides what counts.
//!
//! The whole module is in the Charon `start_from` list (it carries no
//! derives); `proofs/Writfold/Policy.lean` proves it for every input
//! (docs/writfold/aeneas.md). The exhaustive tests below state the same
//! properties for small inputs.

use crate::gate::{Standing, State};
use alloc::vec::Vec;

/// The standings that meet a gate. A definition without `accept` is all
/// three, which is exactly [`Standing::met`].
pub struct Accept {
    pub satisfied: bool,
    pub attested: bool,
    pub waived: bool,
}

/// Whether a standing meets a gate under its policy: satisfied by a check,
/// attested or waived, each only when the policy accepts it.
pub fn met(standing: Standing, accept: &Accept) -> bool {
    match standing {
        Standing::Checked(state) => matches!(state, State::Satisfied) && accept.satisfied,
        Standing::Attested => accept.attested,
        Standing::Waived => accept.waived,
    }
}

/// One pinned gate (or required file) compared with the present.
pub enum Pin {
    /// Nothing was pinned.
    Unpinned,
    /// The present digest is the pinned one, byte for byte.
    Same,
    /// The present digest differs, or nothing is there now.
    Moved,
}

/// One pinned digest against the present: no pin is `Unpinned`; a pin
/// with nothing now, or with other bytes, is `Moved`; equal bytes `Same`.
pub fn pin_one(pinned: &Option<Vec<u8>>, now: &Option<Vec<u8>>) -> Pin {
    match pinned {
        None => Pin::Unpinned,
        Some(pinned) => match now {
            None => Pin::Moved,
            Some(now) => {
                if crate::bytes::same(pinned, now) {
                    Pin::Same
                } else {
                    Pin::Moved
                }
            }
        },
    }
}

/// Whether a pin is `Moved`.
pub fn is_moved(pin: &Pin) -> bool {
    matches!(pin, Pin::Moved)
}

/// Whether a pin is `Unpinned`.
pub fn is_unpinned(pin: &Pin) -> bool {
    matches!(pin, Pin::Unpinned)
}

/// Whether some pin is `Moved`.
pub fn any_moved(pins: &[Pin]) -> bool {
    let mut index = 0;
    while index < pins.len() {
        if is_moved(&pins[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether some pin is `Unpinned`.
pub fn any_unpinned(pins: &[Pin]) -> bool {
    let mut index = 0;
    while index < pins.len() {
        if is_unpinned(&pins[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// A run's policy against its pins: `Moved` when any pin moved; else
/// `Unpinned` when nothing was pinned or some entry was not; else `Same`.
pub fn pin_all(pins: &[Pin]) -> Pin {
    if any_moved(pins) {
        return Pin::Moved;
    }
    if pins.is_empty() || any_unpinned(pins) {
        return Pin::Unpinned;
    }
    Pin::Same
}

/// One gate's standing as a done record names it.
pub enum How {
    Passed,
    Attested,
    Waived,
}

/// How a step was closed.
pub enum Close {
    Open,
    /// Done, every gate passed a check (or the step had none).
    Passed,
    /// Done, some gate attested and none waived.
    Attested,
    /// Done, some gate waived.
    Waived,
    Skipped,
}

/// Whether a recorded standing is a waiver.
pub fn how_waived(how: &How) -> bool {
    matches!(how, How::Waived)
}

/// Whether a recorded standing is an attestation.
pub fn how_attested(how: &How) -> bool {
    matches!(how, How::Attested)
}

/// Whether some gate was met on a waiver.
pub fn any_waived(met: &[How]) -> bool {
    let mut index = 0;
    while index < met.len() {
        if how_waived(&met[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether some gate was met on an attestation.
pub fn any_attested(met: &[How]) -> bool {
    let mut index = 0;
    while index < met.len() {
        if how_attested(&met[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// How one step was closed, from whether a done or a skip record closes it
/// and the standings its done record names. Done wins over skipped; a done
/// step is `Waived` when any gate was waived, else `Attested` when any was
/// attested, else `Passed`.
pub fn close(done: bool, skipped: bool, met: &[How]) -> Close {
    if !done {
        if skipped {
            return Close::Skipped;
        }
        return Close::Open;
    }
    if any_waived(met) {
        return Close::Waived;
    }
    if any_attested(met) {
        return Close::Attested;
    }
    Close::Passed
}

/// The facts about a run, besides its steps, that decide its summary.
pub struct Facts {
    /// The run is closed.
    pub run_closed: bool,
    /// Every required template is in the plan.
    pub required_present: bool,
    /// The run's policy against its pins ([`pin_all`]).
    pub pin: Pin,
    /// The run adopted a changed policy (`plan repin`).
    pub repinned: bool,
    /// A done step's gate no longer holds.
    pub stale: bool,
}

/// One word for how a plan ended.
pub enum Summary {
    NoPlan,
    Incomplete,
    ClosedIncomplete,
    PolicyMoved,
    CompleteStale,
    CompleteWithExceptions,
    Complete,
}

/// Whether a step is still open.
pub fn is_open(close: &Close) -> bool {
    matches!(close, Close::Open)
}

/// Whether a step closed on passing checks only.
pub fn is_passed(close: &Close) -> bool {
    matches!(close, Close::Passed)
}

/// Whether some step is open.
pub fn any_open(closes: &[Close]) -> bool {
    let mut index = 0;
    while index < closes.len() {
        if is_open(&closes[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether every step closed on passing checks only.
pub fn all_passed(closes: &[Close]) -> bool {
    let mut index = 0;
    while index < closes.len() {
        if !is_passed(&closes[index]) {
            return false;
        }
        index += 1;
    }
    true
}

/// The plan's word, the first that applies: no step; a step open or a
/// required template missing (closed or not); the policy moved since the
/// pin; a done step's gate no longer holds; a step skipped, waived or
/// attested, or the run repinned; else complete.
pub fn summary(closes: &[Close], facts: &Facts) -> Summary {
    if closes.is_empty() {
        return Summary::NoPlan;
    }
    if any_open(closes) || !facts.required_present {
        if facts.run_closed {
            return Summary::ClosedIncomplete;
        }
        return Summary::Incomplete;
    }
    if is_moved(&facts.pin) {
        return Summary::PolicyMoved;
    }
    if facts.stale {
        return Summary::CompleteStale;
    }
    if facts.repinned || !all_passed(closes) {
        return Summary::CompleteWithExceptions;
    }
    Summary::Complete
}

pub mod lines;
pub mod repin;

#[cfg(test)]
mod tests;
