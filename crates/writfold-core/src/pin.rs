//! Pins: what a piece of evidence was taken against, compared with the
//! present. Evidence holds while every pin holds; one pin that moved, aged
//! or cannot be read is enough to stop it holding. No pins is today's
//! behaviour, and it holds.
//!
//! A pin is one of:
//! - the bytes a look searched (a negative look's `searched` citation),
//!   judged like any citation;
//! - a time with a lifetime from the kind catalog (`.writfold/sources/`),
//!   judged by [`timed`] with the same [`crate::verdict::expired`] rule
//!   citations age by;
//! - another repository's scope, read through an attached store.
//!
//! The caller reads the clock, the citations and the stores and turns each
//! into a [`Pin`]; the decision is here. Pins only take standing away: they
//! never make evidence hold that would not hold without them.
//!
//! `holds_from`, `holds`, `timed`, `nothing_found` and `current_under` are
//! in the Charon `start_from` list; `proofs/Writfold/Pin.lean` proves them
//! for every input (docs/writfold/aeneas.md).

use crate::observe::Status;
use crate::verdict::{Lifetime, expired};

/// One pin of a piece of evidence, compared with the present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pin {
    /// What it was taken against is unchanged.
    Held,
    /// What it was taken against changed.
    Moved,
    /// Its lifetime has passed.
    Aged,
    /// What it was taken against cannot be read here (no clone, a missing
    /// store, an unparsable token).
    Unreadable,
}

/// Whether every pin from `index` on holds.
pub fn holds_from(pins: &[Pin], index: usize) -> bool {
    let mut at = index;
    while at < pins.len() {
        match pins[at] {
            Pin::Held => {}
            Pin::Moved | Pin::Aged | Pin::Unreadable => return false,
        }
        at += 1;
    }
    true
}

/// Whether every pin holds (and so whether no pins at all hold: they do).
pub fn holds(pins: &[Pin]) -> bool {
    holds_from(pins, 0)
}

/// The time pin of something recorded at `seen` with a lifetime from the
/// catalog, at `now` (seconds): an immutable kind or no lifetime holds; a
/// lifetime of `ttl` seconds ages when `now - seen > ttl`.
pub fn timed(life: Lifetime<i64>, seen: i64, now: i64) -> Pin {
    match life {
        Lifetime::Immutable | Lifetime::Never => Pin::Held,
        Lifetime::Token(ttl) | Lifetime::Default(ttl) => {
            if expired(now, seen, ttl) {
                Pin::Aged
            } else {
                Pin::Held
            }
        }
    }
}

/// Whether a recorded look still reads as having found nothing: its status
/// is `no-data` or `not-found`, and every pin holds.
pub fn nothing_found(status: Status, pins: &[Pin]) -> bool {
    match status {
        Status::NoData | Status::NotFound => holds(pins),
        Status::Ok | Status::Partial | Status::Timeout | Status::Denied | Status::Error => false,
    }
}

/// Whether a receipt or row that is about the present bytes (`about`) is
/// current under its pins.
pub fn current_under(about: bool, pins: &[Pin]) -> bool {
    about && holds(pins)
}

#[cfg(test)]
mod tests {
    use super::{Pin, current_under, holds, nothing_found, timed};
    use crate::observe::{ALL, Status};
    use crate::verdict::Lifetime;
    use alloc::vec::Vec;

    const PINS: [Pin; 4] = [Pin::Held, Pin::Moved, Pin::Aged, Pin::Unreadable];

    fn lists() -> Vec<Vec<Pin>> {
        let mut out: Vec<Vec<Pin>> = Vec::new();
        out.push(Vec::new());
        for a in PINS {
            out.push([a].to_vec());
            for b in PINS {
                out.push([a, b].to_vec());
                for c in PINS {
                    out.push([a, b, c].to_vec());
                }
            }
        }
        out
    }

    #[test]
    fn holds_only_when_every_pin_holds() {
        for pins in lists() {
            let want = pins.iter().all(|p| *p == Pin::Held);
            assert_eq!(holds(&pins), want);
            for about in [false, true] {
                assert_eq!(current_under(about, &pins), about && want);
            }
            for status in ALL {
                let negative = matches!(status, Status::NoData | Status::NotFound);
                assert_eq!(nothing_found(status, &pins), negative && want);
            }
        }
    }

    #[test]
    fn timed_ages_past_its_lifetime() {
        assert_eq!(timed(Lifetime::Immutable, 0, i64::MAX), Pin::Held);
        assert_eq!(timed(Lifetime::Never, 0, i64::MAX), Pin::Held);
        for life in [Lifetime::Token(10), Lifetime::Default(10)] {
            assert_eq!(timed(life, 100, 110), Pin::Held);
            assert_eq!(timed(life, 100, 111), Pin::Aged);
            assert_eq!(timed(life, i64::MIN, i64::MAX), Pin::Aged);
        }
    }
}
