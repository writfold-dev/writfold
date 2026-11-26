//! Generated review units (#156, RFC 2026-11-25-kb-research-contracts):
//! a checklist whose items are made from a scope gate's manifest.
//!
//! - [`pairs`] is every unordered pair of `n` members, `(a, b)` with
//!   `a < b < n`, in lexicographic order, each once; refused over
//!   [`MAX_MEMBERS`].
//! - [`generated`] guards the shipped roll-up, which is covered over zero
//!   items: a checklist whose universe is absent is never covered, and an
//!   empty universe is covered only where the checklist allows it.
//!
//! Reading the manifest, naming units and the digest that joins a
//! checklist to its manifest are the caller's (tested). In the Charon
//! `start_from` list; `proofs/Writfold/Units.lean` proves every function
//! for every input (docs/writfold/aeneas.md).
use crate::checklist::Rollup;
use alloc::vec::Vec;

/// The most members a manifest may have for pair units: 64 members are
/// 2016 units.
pub const MAX_MEMBERS: usize = 64;

/// Every pair `(a, b)` with `a < b < n`, in lexicographic order; `None`
/// when `n` is over [`MAX_MEMBERS`].
pub fn pairs(n: usize) -> Option<Vec<(usize, usize)>> {
    if n > MAX_MEMBERS {
        return None;
    }
    let mut out = Vec::new();
    let mut a = 0;
    while a < n {
        let mut b = a + 1;
        while b < n {
            out.push((a, b));
            b += 1;
        }
        a += 1;
    }
    Some(out)
}

/// The roll-up of a generated checklist: covered exactly when its universe
/// is `present`, it has a unit (`count > 0`) or the checklist allows an
/// empty one, and every unit is covered.
pub fn generated(present: bool, count: usize, allow_empty: bool, rollup: Rollup) -> Rollup {
    let universe = present && (count > 0 || allow_empty);
    Rollup {
        covered: universe && rollup.covered,
        stale: rollup.stale,
        first_owed: rollup.first_owed,
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_MEMBERS, generated, pairs};
    use crate::checklist::Rollup;
    use alloc::vec;

    #[test]
    fn pairs_are_every_unordered_pair_in_order() {
        assert_eq!(pairs(0), Some(vec![]));
        assert_eq!(pairs(1), Some(vec![]));
        assert_eq!(pairs(3), Some(vec![(0, 1), (0, 2), (1, 2)]));
        assert_eq!(pairs(5).map(|p| p.len()), Some(10));
        assert_eq!(pairs(MAX_MEMBERS).map(|p| p.len()), Some(2016));
        assert_eq!(pairs(MAX_MEMBERS + 1), None);
    }

    #[test]
    fn an_absent_or_empty_universe_is_not_covered() {
        let all = Rollup {
            covered: true,
            stale: false,
            first_owed: None,
        };
        assert!(generated(true, 10, false, all).covered);
        assert!(!generated(false, 10, true, all).covered);
        assert!(!generated(true, 0, false, all).covered);
        assert!(generated(true, 0, true, all).covered);
        let owed = Rollup {
            covered: false,
            first_owed: Some(2),
            ..all
        };
        assert_eq!(generated(true, 10, false, owed), owed);
    }
}
