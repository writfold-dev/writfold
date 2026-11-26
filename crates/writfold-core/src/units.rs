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
//! - [`universe`] (#165) decides whether a generated checklist has a
//!   universe at all, from what the caller read of the scope gate and its
//!   kept manifest: the first fact that does not hold, in a fixed order.
//! - [`unit_moved`] (#166) is whether one unit's row is still about the
//!   present: its definition, the seed and its two members. No other
//!   member is an input, so no other member's edit can move it.
//!
//! Reading the manifest, matching member paths against scope patterns,
//! naming units and the digests are the caller's (tested). In the Charon
//! `start_from` list; `proofs/Writfold/Units.lean` proves every function
//! for every input (docs/writfold/aeneas.md).
use crate::checklist::Rollup;
use crate::gate::Standing;
use crate::grounding::Grounding;
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

/// What the caller found of a generated checklist's scope gate and the
/// manifest its deciding receipt kept (#165).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scope {
    /// The scope gate is defined.
    pub defined: bool,
    /// The scope gate's standing now.
    pub standing: Standing,
    /// The kept file reads.
    pub reads: bool,
    /// It is a `writfold-manifest/1` with a members list, each with an id.
    pub manifest: bool,
    /// Its `coverage` is `complete`.
    pub complete: bool,
    /// Every member's path is inside the scope gate's `scope`, so a change
    /// to it makes the scope stale.
    pub inside: bool,
    /// No two members make one unit name (no id holds `~`, none twice).
    pub distinct: bool,
    /// The number of members.
    pub members: usize,
}

/// Why a generated checklist has no universe: the first fact, in the fixed
/// order, that does not hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Absent {
    NoGate,
    NotChecked,
    Unreadable,
    NotManifest,
    Partial,
    OutsideScope,
    SameName,
    OverCap,
}

/// Whether a generated checklist has a universe. The scope gate must
/// ground as a passing check ([`crate::ground::of_gate`]: an attested,
/// waived, open, stale, failed or unknown gate does not), its kept file
/// must read as a complete manifest whose members the scope would notice
/// changing, with distinct unit names, within [`MAX_MEMBERS`].
pub fn universe(facts: &Scope) -> Result<(), Absent> {
    if !facts.defined {
        return Err(Absent::NoGate);
    }
    match crate::ground::of_gate(facts.defined, facts.standing) {
        Grounding::Grounded => {}
        Grounding::Attested | Grounding::Ungrounded => return Err(Absent::NotChecked),
    }
    if !facts.reads {
        return Err(Absent::Unreadable);
    }
    if !facts.manifest {
        return Err(Absent::NotManifest);
    }
    if !facts.complete {
        return Err(Absent::Partial);
    }
    if !facts.inside {
        return Err(Absent::OutsideScope);
    }
    if !facts.distinct {
        return Err(Absent::SameName);
    }
    if facts.members > MAX_MEMBERS {
        return Err(Absent::OverCap);
    }
    Ok(())
}

/// What moved under one unit's row, the first in the fixed order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Moved {
    Definition,
    Seed,
    First,
    Second,
}

/// Whether one unit's row is still about the present (#166): `None` when
/// the checklist's definition, the seed and both members are as the row
/// recorded; else the first that moved.
pub fn unit_moved(
    definition_same: bool,
    seed_same: bool,
    first_same: bool,
    second_same: bool,
) -> Option<Moved> {
    if !definition_same {
        return Some(Moved::Definition);
    }
    if !seed_same {
        return Some(Moved::Seed);
    }
    if !first_same {
        return Some(Moved::First);
    }
    if !second_same {
        return Some(Moved::Second);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Absent, MAX_MEMBERS, Moved, Scope, generated, pairs, unit_moved, universe};
    use crate::checklist::Rollup;
    use crate::gate::{Standing, State};
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

    const OK: Scope = Scope {
        defined: true,
        standing: Standing::Checked(State::Satisfied),
        reads: true,
        manifest: true,
        complete: true,
        inside: true,
        distinct: true,
        members: 5,
    };

    /// `OK` with one fact changed.
    fn with(change: fn(&mut Scope)) -> Scope {
        let mut facts = OK;
        change(&mut facts);
        facts
    }

    #[test]
    fn a_universe_needs_every_fact_in_order() {
        assert_eq!(universe(&OK), Ok(()));
        let cases: [(fn(&mut Scope), Absent); 10] = [
            (|f| f.defined = false, Absent::NoGate),
            (|f| f.standing = Standing::Attested, Absent::NotChecked),
            (|f| f.standing = Standing::Waived, Absent::NotChecked),
            (
                |f| f.standing = Standing::Checked(State::Stale),
                Absent::NotChecked,
            ),
            (|f| f.reads = false, Absent::Unreadable),
            (|f| f.manifest = false, Absent::NotManifest),
            (|f| f.complete = false, Absent::Partial),
            (|f| f.inside = false, Absent::OutsideScope),
            (|f| f.distinct = false, Absent::SameName),
            (|f| f.members = MAX_MEMBERS + 1, Absent::OverCap),
        ];
        for (change, want) in cases {
            assert_eq!(universe(&with(change)), Err(want));
        }
        let two = with(|f| {
            f.complete = false;
            f.members = 99;
        });
        assert_eq!(universe(&two), Err(Absent::Partial));
        assert_eq!(universe(&with(|f| f.members = MAX_MEMBERS)), Ok(()));
    }

    #[test]
    fn a_unit_moves_only_with_its_own_inputs() {
        assert_eq!(unit_moved(true, true, true, true), None);
        assert_eq!(
            unit_moved(false, false, false, false),
            Some(Moved::Definition)
        );
        assert_eq!(unit_moved(true, false, false, true), Some(Moved::Seed));
        assert_eq!(unit_moved(true, true, false, false), Some(Moved::First));
        assert_eq!(unit_moved(true, true, true, false), Some(Moved::Second));
    }
}
