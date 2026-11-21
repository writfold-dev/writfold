//! Whether a policy inventory is complete (#56): definitions are read from
//! several roots, and a root that could not be listed makes the inventory
//! incomplete. No aggregate answer (`gates.met`, `gate lines`, an unplanned
//! run's "all met") is positive over an incomplete inventory; over a
//! complete one each keeps its existing answer. Translated and proved
//! (`proofs/Writfold/Inventory.lean`); classifying a root from the
//! filesystem is tested IO in `writfold-kb`.

use crate::policy::Summary;
use crate::policy::lines::{Listed, lines_clean};

/// One root a policy directory is read from.
pub enum Root {
    /// The directory does not exist: nothing to read, nothing missed.
    Absent,
    /// Every entry was listed.
    Listed,
    /// The directory exists (or a link to it does) but could not be
    /// listed, in whole or part-way.
    Unreadable,
}

/// Whether `root` was read in full or is legitimately absent.
pub fn readable(root: &Root) -> bool {
    !matches!(root, Root::Unreadable)
}

/// Whether no root is unreadable.
pub fn complete(roots: &[Root]) -> bool {
    let mut index = 0;
    while index < roots.len() {
        if !readable(&roots[index]) {
            return false;
        }
        index += 1;
    }
    true
}

/// How many roots are unreadable: each is one counted problem, so every
/// answer that needs no invalid definition (`gates.met`, `gate status`)
/// is not met while one remains.
pub fn unreadable(roots: &[Root]) -> usize {
    let mut index = 0;
    let mut count = 0;
    while index < roots.len() {
        if !readable(&roots[index]) {
            count += 1;
        }
        index += 1;
    }
    count
}

/// `gate lines` for a planned run over an inventory: never clean when it
/// is incomplete, else [`lines_clean`].
pub fn lines_clean_over(complete: bool, summary: &Summary, listed: &[Listed], valid: bool) -> bool {
    complete && lines_clean(summary, listed, valid)
}

/// Whether every element of `met` is true.
pub fn all_true(met: &[bool]) -> bool {
    let mut index = 0;
    while index < met.len() {
        if !met[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// `gate lines` and `gate status` for an unplanned run: clean only over a
/// complete inventory with at least one gate, every gate met and nothing
/// invalid.
pub fn unplanned_clean(complete: bool, met: &[bool], valid: bool) -> bool {
    complete && !met.is_empty() && all_true(met) && valid
}

#[cfg(test)]
mod tests {
    use super::{Root, complete, unplanned_clean, unreadable};

    #[test]
    fn an_unreadable_root_makes_the_inventory_incomplete() {
        assert!(complete(&[]));
        assert!(complete(&[Root::Absent, Root::Listed]));
        assert!(!complete(&[Root::Listed, Root::Unreadable]));
    }

    #[test]
    fn no_aggregate_is_positive_over_an_incomplete_inventory() {
        assert_eq!(
            unreadable(&[Root::Unreadable, Root::Absent, Root::Unreadable]),
            2
        );
        assert!(unplanned_clean(true, &[true, true], true));
        assert!(!unplanned_clean(false, &[true], true));
        assert!(!unplanned_clean(true, &[], true));
        assert!(!unplanned_clean(true, &[true, false], true));
        assert!(!unplanned_clean(true, &[true], false));
    }
}
