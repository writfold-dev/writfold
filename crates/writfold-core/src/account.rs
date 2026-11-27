//! An accounted universe (#181, RFC 2026-11-27-referents-remainders-sites,
//! law U): each member of something a universal word is said of (the
//! files of a run, the inputs of a packet) is judged, declared out, or
//! left over. A word that claims all of it holds only when nothing is
//! left over ([`whole`]).
//!
//! Listing and classing the members is the caller's (tested). In the
//! Charon `start_from` list; `proofs/Writfold/Account.lean` proves both
//! functions and links them to `Calculus.remainder` and `Calculus.whole`.

/// How one member of a universe was accounted for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Member {
    /// Read and judged.
    Judged,
    /// Out of the universe by a declared rule (a dot-name).
    DeclaredOut,
    /// Neither: left over.
    Remainder,
}

/// Whether a member is left over.
pub fn left(m: Member) -> bool {
    match m {
        Member::Remainder => true,
        Member::Judged | Member::DeclaredOut => false,
    }
}

/// How many members are left over.
pub fn remainder(members: &[Member]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < members.len() {
        if left(members[index]) {
            count += 1;
        }
        index += 1;
    }
    count
}

/// Nothing is left over.
pub fn whole(members: &[Member]) -> bool {
    remainder(members) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_remainder_counts_only_what_is_left_over() {
        use Member::*;
        assert_eq!(remainder(&[]), 0);
        assert!(whole(&[Judged, DeclaredOut]));
        assert_eq!(remainder(&[Remainder, Judged, Remainder, DeclaredOut]), 2);
        assert!(!whole(&[Judged, Remainder]));
    }
}
