//! Attached stores: companion Git repositories of disposable evidence
//! (probe scripts, data pulls, review notes) that a KB cites and never
//! curates. A store is declared in a committed `.writfold/stores/<name>.toml`.
//! The decisions: which rule judges a citation (the head of the store, as
//! for any repository, or the pinned revision the citation names); the
//! verdict under the pin; and where a declared store's clone is found on
//! this machine; and whether a run may record a pointer into a store
//! (`checkpoint --artifact`); and what a store directory's `subject:`
//! claim amounts to (backlinks, [`claim`]). Translated and proved (`proofs/Writfold/Store.lean`).
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

/// Whether a recorded verdict (a check receipt written by `kb stale
/// --record` on a machine that had the store) may stand in for a citation:
/// only under the pin, only for a store that declares `recorded_max_age`,
/// and only while the receipt is within it (U3, #4). A head-judged
/// citation never uses one.
pub fn recorded(basis: &Basis, declared_max_age: bool, within_age: bool) -> bool {
    match basis {
        Basis::Pin => declared_max_age && within_age,
        Basis::Head => false,
    }
}

/// Whether a reader uses a recorded verdict instead of its own: only when
/// its own is `unverifiable` (no clone here, or the commit not in it) and
/// [`recorded`] allows. A verdict this machine could reach always wins.
pub fn use_recorded(local: &Verdict, recorded: bool) -> bool {
    match local {
        Verdict::Unverifiable => recorded,
        _ => false,
    }
}

/// Whether a verdict under the pin may be written to a receipt: every one
/// but `unverifiable`, which describes this machine, not the citation.
pub fn pin_recordable(judged: &Verdict) -> bool {
    !matches!(judged, Verdict::Unverifiable)
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

/// Why a run artifact is not recorded, in the order they are checked.
pub enum Refusal {
    /// No `.writfold/stores/<name>.toml` declares the store.
    Undeclared,
    /// The store has no usable clone on this machine (missing, or a clone
    /// of another repository).
    NotLocal,
    /// The revision is not a full commit ID.
    ShortRev,
    /// The clone does not have the commit (fetch it).
    NoCommit,
    /// No branch or tag reaches the commit (push or tag it first).
    Unanchored,
    /// The path holds no file at that commit.
    NoBlob,
}

/// Whether `checkpoint --artifact` records a pointer.
pub enum Artifact {
    Admit,
    Refuse(Refusal),
}

/// A run artifact is recorded exactly when its store is declared and
/// found here, the revision is a full commit ID that some ref reaches, and
/// the path holds a file there: exactly the facts under which its token
/// reads fresh under the pin ([`pinned`]). The pointer is an ordinary
/// citation, judged by the same rule on every later read.
pub fn artifact(
    declared: bool,
    found: bool,
    full_rev: bool,
    commit: &Commit,
    blob: bool,
) -> Artifact {
    if !declared {
        return Artifact::Refuse(Refusal::Undeclared);
    }
    if !found {
        return Artifact::Refuse(Refusal::NotLocal);
    }
    if !full_rev {
        return Artifact::Refuse(Refusal::ShortRev);
    }
    match commit {
        Commit::Absent => Artifact::Refuse(Refusal::NoCommit),
        Commit::Unanchored => Artifact::Refuse(Refusal::Unanchored),
        Commit::Anchored => {
            if blob {
                Artifact::Admit
            } else {
                Artifact::Refuse(Refusal::NoBlob)
            }
        }
    }
}

/// What one `subject:` entry of a store directory's `README.md` amounts to.
pub enum Claim {
    /// It names exactly one record: listed under that record.
    Listed,
    /// It is an alias of two or more records: listed under none.
    Ambiguous,
    /// It names no record.
    Unknown,
    /// The directory was not read from a declared, readable attached
    /// store: nothing is listed from it.
    NotAStore,
}

/// `attached`: the directory was read from the clone [`resolve`] found for
/// a declared store. `exact`: a record has the subject as its ID.
/// `aliased`: how many distinct records have it as an alias. A record's
/// own ID wins over an alias other records share, the rule authored edges
/// follow.
pub fn claim(attached: bool, exact: bool, aliased: usize) -> Claim {
    if !attached {
        return Claim::NotAStore;
    }
    if exact {
        return Claim::Listed;
    }
    if aliased == 1 {
        Claim::Listed
    } else if aliased == 0 {
        Claim::Unknown
    } else {
        Claim::Ambiguous
    }
}

/// The facts about a snapshot that are not Git commits or bytes at a path.
pub struct SnapshotFacts {
    /// The look's status embeds a citation (`observe::embeds`).
    pub embeds: bool,
    /// The store is declared.
    pub declared: bool,
    /// Its freshness is `pinned`.
    pub pinned: bool,
    /// [`resolve`] found a clone with a work tree.
    pub local: bool,
    /// The bytes in the work tree are the ones its provenance file names.
    pub intact: bool,
}

/// Why `observe --snapshot` records nothing, in the order checked.
pub enum Withheld {
    /// The status embeds no citation (`no-data`, `not-found`, `error`, …).
    NoCitation,
    /// No declared store to keep it in.
    NotAStore,
    /// The store is judged at head, where a reorganisation would lose it.
    NotPinned,
    /// The store has no clone with a work tree here.
    NotLocal,
    /// The bytes no longer match their provenance file.
    Damaged,
    /// The store has no commit, or its `HEAD` holds nothing at the path:
    /// the only answer under which files may be written into the store.
    NotCommitted,
    /// `HEAD` holds other bytes at the path (a filter, or an edit).
    OtherBytes,
    /// No ref reaches `HEAD`.
    Unanchored,
}

/// Whether `observe --snapshot` records the observation.
pub enum Snapshot {
    Record,
    Withhold(Withheld),
}

/// `commit`: the store's `HEAD` commit (`Absent` when it has none). `at`:
/// what `HEAD:<path>` holds against the snapshot's bytes, the worse of the
/// file and its provenance file. A snapshot is recorded exactly when its
/// citation would read fresh under [`pinned`]: there is no second rule.
pub fn snapshot(facts: &SnapshotFacts, commit: &Commit, at: &AtPin) -> Snapshot {
    if !facts.embeds {
        return Snapshot::Withhold(Withheld::NoCitation);
    }
    if !facts.declared {
        return Snapshot::Withhold(Withheld::NotAStore);
    }
    if !facts.pinned {
        return Snapshot::Withhold(Withheld::NotPinned);
    }
    if !facts.local {
        return Snapshot::Withhold(Withheld::NotLocal);
    }
    if !facts.intact {
        return Snapshot::Withhold(Withheld::Damaged);
    }
    match (commit, at) {
        (Commit::Absent, _) | (_, AtPin::Nothing) => Snapshot::Withhold(Withheld::NotCommitted),
        (_, AtPin::Other) => Snapshot::Withhold(Withheld::OtherBytes),
        (Commit::Unanchored, AtPin::Cited) => Snapshot::Withhold(Withheld::Unanchored),
        (Commit::Anchored, AtPin::Cited) => Snapshot::Record,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Artifact, AtPin, Basis, Candidate, Claim, Commit, Refusal, Resolved, Via, artifact, basis,
        claim, pinned, resolve,
    };
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

    #[test]
    fn an_artifact_is_admitted_only_when_its_pin_reads_fresh() {
        let commits = || [Commit::Absent, Commit::Unanchored, Commit::Anchored];
        for bits in 0..16u8 {
            let (d, l, f, b) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0);
            for commit in commits() {
                let at = if b { AtPin::Cited } else { AtPin::Nothing };
                let fresh = pinned(&commit, &at) == Verdict::Fresh;
                let got = artifact(d, l, f, &commit, b);
                assert_eq!(matches!(got, Artifact::Admit), d && l && f && fresh);
                if !d {
                    assert!(matches!(got, Artifact::Refuse(Refusal::Undeclared)));
                }
            }
        }
    }

    #[test]
    fn a_claim_is_listed_only_from_a_store_for_one_record() {
        for attached in [false, true] {
            for exact in [false, true] {
                for aliased in 0..4usize {
                    let got = claim(attached, exact, aliased);
                    let want = match (attached, exact, aliased) {
                        (false, _, _) => "not-a-store",
                        (true, true, _) | (true, false, 1) => "listed",
                        (true, false, 0) => "unknown",
                        _ => "ambiguous",
                    };
                    let said = match got {
                        Claim::Listed => "listed",
                        Claim::Ambiguous => "ambiguous",
                        Claim::Unknown => "unknown",
                        Claim::NotAStore => "not-a-store",
                    };
                    assert_eq!(said, want, "{attached} {exact} {aliased}");
                }
            }
        }
    }
}
