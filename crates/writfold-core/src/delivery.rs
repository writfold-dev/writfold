//! A delivery owed by a plan step (`deliver:<mode>`): which candidate
//! decides an open step, one standing and one readiness for the whole
//! delivery from its obligations (each with the standings its policy
//! accepts), what a post record makes of a `posted:<mode>` gate, and the
//! delivery guard's decision over every candidate of the run. Nothing here
//! reads a candidate, a mode, a receipt or a command; the caller judges each
//! obligation with [`crate::gate`] and passes the standings in the mode's
//! order.
//!
//! Every function here is in the Charon `start_from` list;
//! `proofs/Writfold/Delivery.lean` proves them for every input
//! (docs/writfold/aeneas.md). The exhaustive tests in `delivery/tests.rs`
//! state the same properties for small inputs.

use crate::gate::{Standing, State};
use crate::plan::GateFact;
use crate::policy::Accept;

/// One obligation of a delivery: its standing as the gate kernel gives it,
/// and the standings its policy takes (a mode's `waivable`, or a mode
/// gate's own `accept`).
pub struct Owed {
    pub standing: Standing,
    pub accept: Accept,
}

/// Where one candidate of a mode stands for an open step, oldest first by
/// `(seq, id)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Place {
    /// Live and named by no done record of the run.
    Open,
    /// Named by a done record of the run.
    Bound,
    /// Withdrawn by a statement: it decides nothing.
    Withdrawn,
}

/// Whether a place is `Open`.
pub fn is_open(place: &Place) -> bool {
    matches!(place, Place::Open)
}

/// Whether a place is `Bound`.
pub fn is_bound(place: &Place) -> bool {
    matches!(place, Place::Bound)
}

/// The candidate that decides an open step: scanning from the newest,
/// withdrawn candidates are passed over; the first other one decides if it
/// is open, and nothing does if it is bound (an older draft never decides
/// once a newer candidate of its mode is bound).
pub fn deciding(places: &[Place]) -> Option<usize> {
    let mut index = places.len();
    while index > 0 {
        index -= 1;
        if is_open(&places[index]) {
            return Some(index);
        }
        if is_bound(&places[index]) {
            return None;
        }
    }
    None
}

/// Whether an obligation is not met under its policy
/// ([`crate::policy::met`]).
pub fn unmet(owed: &Owed) -> bool {
    !crate::policy::met(owed.standing, &owed.accept)
}

/// Whether a standing is a waiver.
pub fn is_waived(standing: Standing) -> bool {
    matches!(standing, Standing::Waived)
}

/// Whether a standing is an attestation.
pub fn is_attested(standing: Standing) -> bool {
    matches!(standing, Standing::Attested)
}

/// The first obligation not met under its policy, in the mode's order.
pub fn first_unmet(owed: &[Owed]) -> Option<usize> {
    let mut index = 0;
    while index < owed.len() {
        if unmet(&owed[index]) {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// One standing for a delivery from its obligations', in the mode's order:
/// the weakest word that is true. Empty: `Checked(Open)` (an empty set is
/// not met). Else the first unmet obligation's standing (a waiver its
/// policy rejects reads `Waived` here, and is not met); else `Waived` if any
/// is waived; else `Attested` if any is attested; else `Checked(Satisfied)`.
/// A statement is never read as a passing check.
pub fn standing(owed: &[Owed]) -> Standing {
    if owed.is_empty() {
        return Standing::Checked(State::Open);
    }
    let mut waived = false;
    let mut attested = false;
    let mut index = 0;
    while index < owed.len() {
        if unmet(&owed[index]) {
            return owed[index].standing;
        }
        if is_waived(owed[index].standing) {
            waived = true;
        }
        if is_attested(owed[index].standing) {
            attested = true;
        }
        index += 1;
    }
    if waived {
        Standing::Waived
    } else if attested {
        Standing::Attested
    } else {
        Standing::Checked(State::Satisfied)
    }
}

/// Whether a candidate is ready: it owes at least one obligation and every
/// one is met under its policy. A mode that declares nothing is never
/// ready; a waiver the policy rejects never makes one ready.
pub fn ready(owed: &[Owed]) -> bool {
    if owed.is_empty() {
        return false;
    }
    match first_unmet(owed) {
        None => true,
        Some(_) => false,
    }
}

/// Whether the delivery is held by a statement its policy rejects: the
/// first unmet obligation stands on a met word (a waiver, or an
/// attestation, the policy does not take).
pub fn refused(owed: &[Owed]) -> bool {
    match first_unmet(owed) {
        None => false,
        Some(index) => owed[index].standing.met(),
    }
}

/// What a plan step's `deliver:<mode>` gate is given: the delivery's
/// [`standing`], with every word accepted when it is [`ready`] and none
/// otherwise, so the proved `plan::fact_met` is exactly readiness.
pub fn fact(owed: &[Owed]) -> GateFact {
    let taken = ready(owed);
    GateFact::Stands(
        standing(owed),
        Accept {
            satisfied: taken,
            attested: taken,
            waived: taken,
        },
    )
}

/// A `posted:<mode>` gate: `Attested` when a post record about the
/// candidate's present bytes exists, else `Checked(Open)`. A post record is
/// a statement; it is never a passing check.
pub fn posted(recorded: bool) -> Standing {
    if recorded {
        Standing::Attested
    } else {
        Standing::Checked(State::Open)
    }
}

/// A passed check whose cited evidence moved since (`moved` citations no
/// longer fresh, or the re-check could not run) is stale; any other state
/// stands as it is.
pub fn rechecked(checked: State, moved: usize) -> State {
    if matches!(checked, State::Satisfied) && moved > 0 {
        State::Stale
    } else {
        checked
    }
}

/// What a guarded posting command would post, as far as the guard can read.
pub enum Body {
    /// The bytes are read from the command.
    Known,
    /// They cannot be read from the command.
    Unknown,
}

/// What unread bytes get: a listed command or tool keeps the lenient rule
/// (every live candidate of the run ready); a route denies them.
pub enum Unread {
    AllReady,
    Deny,
}

/// One candidate of the run as the guard sees it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Seen {
    /// Its payload is the bytes being posted.
    pub same: bool,
    /// It is ready ([`ready`]).
    pub ready: bool,
    /// It is not withdrawn.
    pub live: bool,
    /// It is of the route's mode (true for a plain list entry).
    pub routed: bool,
    /// A post record about its present bytes exists.
    pub spent: bool,
}

/// Whether the guard may pass a candidate's bytes.
pub fn postable(seen: &Seen) -> bool {
    seen.same && seen.ready && seen.live && seen.routed && !seen.spent
}

/// Whether some candidate is postable.
pub fn guard_any(seen: &[Seen]) -> bool {
    let mut index = 0;
    while index < seen.len() {
        if postable(&seen[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether at least one candidate is live and every live one is ready.
pub fn guard_all(seen: &[Seen]) -> bool {
    let mut live = false;
    let mut index = 0;
    while index < seen.len() {
        if seen[index].live && !seen[index].ready {
            return false;
        }
        if seen[index].live {
            live = true;
        }
        index += 1;
    }
    live
}

/// The delivery guard's decision. Known bytes pass exactly when some
/// candidate is postable. Unknown bytes never pass under `Deny`; under
/// `AllReady` they pass when at least one candidate is live and every live
/// candidate is ready.
pub fn guard(body: Body, unread: Unread, seen: &[Seen]) -> bool {
    match body {
        Body::Known => guard_any(seen),
        Body::Unknown => match unread {
            Unread::AllReady => guard_all(seen),
            Unread::Deny => false,
        },
    }
}

#[cfg(test)]
mod tests;
