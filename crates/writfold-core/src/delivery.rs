//! A delivery owed by a plan step (`deliver:<mode>`): which candidate
//! decides an open step, and one standing for the whole delivery from its
//! obligations' standings. Nothing here reads a candidate, a mode or a
//! receipt; the caller judges each obligation with [`crate::gate`] and
//! passes the standings in the mode's order.
//!
//! `deciding`, `unmet`, `is_waived`, `is_attested`, `standing`, `ready`,
//! `rechecked` and `guard` are in the Charon `start_from` list;
//! `proofs/Writfold/Delivery.lean` proves them for every input
//! (docs/writfold/aeneas.md). The exhaustive tests below state the same
//! properties for small inputs.

use crate::gate::{Standing, State};

/// The candidate that decides an open step: the last unbound one.
/// `bound[i]` is whether candidate `i` (oldest first, by `(seq, id)`) is
/// named by a done record of the run.
pub fn deciding(bound: &[bool]) -> Option<usize> {
    let mut index = bound.len();
    while index > 0 {
        index -= 1;
        if !bound[index] {
            return Some(index);
        }
    }
    None
}

/// Whether an obligation's standing is not met. Its own function so the
/// loop in [`standing`] stays flat for the translation.
pub fn unmet(standing: Standing) -> bool {
    !standing.met()
}

/// Whether a standing is a waiver.
pub fn is_waived(standing: Standing) -> bool {
    matches!(standing, Standing::Waived)
}

/// Whether a standing is an attestation.
pub fn is_attested(standing: Standing) -> bool {
    matches!(standing, Standing::Attested)
}

/// One standing for a delivery from its obligations', in the mode's order:
/// the weakest word that is true. Empty: `Checked(Open)` (an empty set is
/// not met). Else the first unmet obligation's standing; else `Waived` if
/// any is waived; else `Attested` if any is attested; else
/// `Checked(Satisfied)`. A statement is never read as a passing check.
pub fn standing(obligations: &[Standing]) -> Standing {
    if obligations.is_empty() {
        return Standing::Checked(State::Open);
    }
    let mut waived = false;
    let mut attested = false;
    let mut index = 0;
    while index < obligations.len() {
        if unmet(obligations[index]) {
            return obligations[index];
        }
        if is_waived(obligations[index]) {
            waived = true;
        }
        if is_attested(obligations[index]) {
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

/// Whether a candidate is ready: its delivery's [`standing`] is met. A mode
/// that declares nothing is never ready (an empty set is not met).
pub fn ready(obligations: &[Standing]) -> bool {
    standing(obligations).met()
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

/// The delivery guard's decision: allow known bytes only when a ready
/// candidate of the run holds them (`same_ready`); allow unknown bytes only
/// when the run has at least one candidate and every candidate is ready.
pub fn guard(body: Body, same_ready: bool, candidates: usize, all_ready: bool) -> bool {
    match body {
        Body::Known => same_ready,
        Body::Unknown => candidates > 0 && all_ready,
    }
}

#[cfg(test)]
mod tests {
    use super::{Body, deciding, guard, ready, rechecked, standing};
    use crate::gate::{Standing, State};
    use alloc::vec::Vec;

    const ALL: [Standing; 7] = [
        Standing::Checked(State::Open),
        Standing::Checked(State::Satisfied),
        Standing::Checked(State::Stale),
        Standing::Checked(State::Failed),
        Standing::Checked(State::Unknown),
        Standing::Attested,
        Standing::Waived,
    ];

    /// Every list of up to `max` elements over `ALL`.
    fn lists(max: usize) -> Vec<Vec<Standing>> {
        let mut out: Vec<Vec<Standing>> = Vec::new();
        out.push(Vec::new());
        let mut layer: Vec<Vec<Standing>> = Vec::from([Vec::new()]);
        for _ in 0..max {
            let mut next = Vec::new();
            for list in &layer {
                for item in ALL {
                    let mut grown = list.clone();
                    grown.push(item);
                    next.push(grown);
                }
            }
            out.extend(next.iter().cloned());
            layer = next;
        }
        out
    }

    #[test]
    fn deciding_is_the_last_unbound_candidate() {
        for len in 0..=4usize {
            for mask in 0..(1u32 << len) {
                let bound: Vec<bool> = (0..len).map(|i| mask & (1 << i) != 0).collect();
                let want = (0..len).rev().find(|&i| !bound[i]);
                assert_eq!(deciding(&bound), want, "{bound:?}");
                if let Some(i) = deciding(&bound) {
                    assert!(!bound[i]);
                    assert!(bound[i + 1..].iter().all(|&b| b), "{bound:?}");
                }
            }
        }
    }

    #[test]
    fn standing_is_the_weakest_true_word() {
        let all = lists(3);
        assert_eq!(all.len(), 1 + 7 + 49 + 343);
        for list in &all {
            let got = standing(list);
            let every_satisfied = list
                .iter()
                .all(|s| *s == Standing::Checked(State::Satisfied));
            let every_met = list.iter().all(|s| s.met());
            let statement = list
                .iter()
                .any(|s| matches!(s, Standing::Attested | Standing::Waived));
            assert_eq!(
                got == Standing::Checked(State::Satisfied),
                !list.is_empty() && every_satisfied,
                "{list:?}"
            );
            assert_eq!(got.met(), !list.is_empty() && every_met, "{list:?}");
            if statement {
                assert_ne!(got, Standing::Checked(State::Satisfied), "{list:?}");
            }
            let want = match list.iter().find(|s| !s.met()) {
                Some(first) => *first,
                None if list.is_empty() => Standing::Checked(State::Open),
                None if list.contains(&Standing::Waived) => Standing::Waived,
                None if list.contains(&Standing::Attested) => Standing::Attested,
                None => Standing::Checked(State::Satisfied),
            };
            assert_eq!(got, want, "{list:?}");
        }
    }

    #[test]
    fn waived_is_weaker_than_attested() {
        let list = [Standing::Attested, Standing::Waived];
        assert_eq!(standing(&list), Standing::Waived);
        let list = [Standing::Checked(State::Satisfied), Standing::Attested];
        assert_eq!(standing(&list), Standing::Attested);
        assert_eq!(standing(&[]), Standing::Checked(State::Open));
    }

    /// A mode that declares nothing is never ready; otherwise ready is met.
    #[test]
    fn ready_is_standing_met() {
        assert!(!ready(&[]));
        for list in lists(2) {
            assert_eq!(ready(&list), standing(&list).met(), "{list:?}");
        }
    }

    #[test]
    fn rechecked_and_guard() {
        assert_eq!(rechecked(State::Satisfied, 1), State::Stale);
        assert_eq!(rechecked(State::Satisfied, 0), State::Satisfied);
        assert_eq!(rechecked(State::Failed, 3), State::Failed);
        for (same, n, all) in [(false, 0, true), (true, 0, false), (true, 2, true)] {
            assert_eq!(guard(Body::Known, same, n, all), same);
            assert_eq!(guard(Body::Unknown, same, n, all), n > 0 && all);
        }
    }
}
