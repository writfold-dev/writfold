//! Attached stores: companion Git repositories of disposable evidence
//! (probe scripts, data pulls, review notes) that a KB cites and never
//! curates. A store is declared in a committed `.writfold/stores/<name>.toml`.
//! The decisions: which rule judges a citation (the head of the store, as
//! for any repository, or the pinned revision the citation names); the
//! verdict under the pin; and where a declared store's clone is found on
//! this machine. Translated and proved (`proofs/Writfold/Store.lean`).
//! Reading the declaration, the clone's origin and the Git facts is
//! `writfold-kb`'s (`stores.rs`, `stale/pinned.rs`), tested there.

use crate::verdict::Verdict;

/// The rule a citation is judged by.
pub enum Basis {
    /// What the path holds at the repository's target commit (every
    /// repository that is not a pinned store).
    Head,
    /// What the citation's own revision holds.
    Pin,
}

/// The pin judges exactly when the repository is a declared store whose
/// freshness is `pinned` and the token names a revision.
pub fn basis(declared: bool, pinned: bool, has_rev: bool) -> Basis {
    if declared && pinned && has_rev {
        Basis::Pin
    } else {
        Basis::Head
    }
}

/// The cited commit in the local clone.
pub enum Commit {
    /// The clone does not have it (it may be behind).
    Absent,
    /// It is on disk but no ref reaches it: a fresh clone would not have
    /// it, and garbage collection may remove it.
    Unanchored,
    /// Some ref reaches it.
    Anchored,
}

/// What `rev:path` holds.
pub enum AtPin {
    /// The cited blob.
    Cited,
    /// Another blob.
    Other,
    /// Nothing.
    Nothing,
}

/// The verdict under the pin. Nothing about the store's head is an input:
/// a store that was reorganised since reads the same.
pub fn pinned(commit: &Commit, at: &AtPin) -> Verdict {
    match commit {
        Commit::Absent => Verdict::Unverifiable,
        Commit::Unanchored => Verdict::Gone,
        Commit::Anchored => match at {
            AtPin::Cited => Verdict::Fresh,
            AtPin::Other | AtPin::Nothing => Verdict::Gone,
        },
    }
}

/// One way of finding a declared store's clone on this machine.
pub enum Candidate {
    /// This way names no clone.
    None,
    /// It names a clone whose origin is (`true`) or is not the declared URL.
    Clone(bool),
}

/// Which way found the clone.
pub enum Via {
    /// The KB's local repository map.
    Map,
    /// A directory under a `$WRITFOLD_KB_REPOS` root.
    SearchRoot,
    /// A path hint in the committed declaration.
    Hint,
}

/// Where a declared store resolves.
pub enum Resolved {
    /// A clone to read; `flagged` when its origin is not the declared URL
    /// (possible only through the local map, which the machine's owner
    /// wrote).
    Found { via: Via, flagged: bool },
    /// A clone was found only where the declaration does not vouch for it:
    /// its origin is another repository. Nothing is read from it.
    Mismatch,
    /// No way names a clone.
    Missing,
}

/// The local map wins whenever it names a clone, whatever its origin.
/// Otherwise the first of the search roots and the committed hints whose
/// clone has the declared origin; a mismatch when one names a clone but
/// none of those has it; missing when none names a clone.
pub fn resolve(map: &Candidate, search: &Candidate, hint: &Candidate) -> Resolved {
    if let Candidate::Clone(origin) = map {
        return Resolved::Found {
            via: Via::Map,
            flagged: !*origin,
        };
    }
    if matches!(search, Candidate::Clone(true)) {
        return Resolved::Found {
            via: Via::SearchRoot,
            flagged: false,
        };
    }
    if matches!(hint, Candidate::Clone(true)) {
        return Resolved::Found {
            via: Via::Hint,
            flagged: false,
        };
    }
    if matches!(search, Candidate::Clone(_)) || matches!(hint, Candidate::Clone(_)) {
        return Resolved::Mismatch;
    }
    Resolved::Missing
}

#[cfg(test)]
mod tests {
    use super::{AtPin, Basis, Candidate, Commit, Resolved, Via, basis, pinned, resolve};
    use crate::verdict::Verdict;

    #[test]
    fn only_a_pinned_store_with_a_revision_is_judged_at_the_pin() {
        for bits in 0..8u8 {
            let (d, p, r) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
            assert_eq!(matches!(basis(d, p, r), Basis::Pin), d && p && r);
        }
    }

    #[test]
    fn the_pin_is_fresh_only_for_the_cited_bytes_at_an_anchored_commit() {
        let ats = || [AtPin::Cited, AtPin::Other, AtPin::Nothing];
        for at in ats() {
            assert_eq!(
                pinned(&Commit::Absent, &at),
                Verdict::Unverifiable,
                "a clone that is behind is not evidence of loss"
            );
            assert_eq!(pinned(&Commit::Unanchored, &at), Verdict::Gone);
        }
        assert_eq!(pinned(&Commit::Anchored, &AtPin::Cited), Verdict::Fresh);
        assert_eq!(pinned(&Commit::Anchored, &AtPin::Other), Verdict::Gone);
        assert_eq!(pinned(&Commit::Anchored, &AtPin::Nothing), Verdict::Gone);
    }

    fn of(code: u8) -> Candidate {
        match code {
            0 => Candidate::None,
            1 => Candidate::Clone(true),
            _ => Candidate::Clone(false),
        }
    }

    #[test]
    fn the_map_wins_and_committed_ways_need_the_declared_origin() {
        for m in 0..3u8 {
            for s in 0..3u8 {
                for h in 0..3u8 {
                    let got = resolve(&of(m), &of(s), &of(h));
                    let want = match (m, s, h) {
                        (1, _, _) => "map",
                        (2, _, _) => "map!",
                        (_, 1, _) => "search",
                        (_, _, 1) => "hint",
                        (_, 0, 0) => "missing",
                        _ => "mismatch",
                    };
                    let said = match got {
                        Resolved::Found {
                            via: Via::Map,
                            flagged,
                        } => {
                            if flagged {
                                "map!"
                            } else {
                                "map"
                            }
                        }
                        Resolved::Found {
                            via: Via::SearchRoot,
                            ..
                        } => "search",
                        Resolved::Found { via: Via::Hint, .. } => "hint",
                        Resolved::Mismatch => "mismatch",
                        Resolved::Missing => "missing",
                    };
                    assert_eq!(said, want, "{m} {s} {h}");
                }
            }
        }
    }
}
