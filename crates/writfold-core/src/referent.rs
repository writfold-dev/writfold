//! The asset boundary (#196, RFC 2026-11-27-asset-boundary): the one path
//! from what a provider observed to a citation's verdict.
//!
//! A provider resolves the snapshot a citation is judged
//! against, says how the cited snapshot relates to it, and observes the
//! bytes there. It never supplies a verdict: [`judge`] computes it as
//! [`verdict::observed`](crate::verdict::observed) of the fact, then
//! [`settle`], which only withholds. An unresolved referent supports no
//! verdict, and a referent behind the cited snapshot cannot show that
//! anything changed, moved or went.
//!
//! Nothing here names a provider. In the Charon `start_from` list;
//! `proofs/Writfold/Referent.lean` proves every function for every input
//! (docs/writfold/aeneas.md).
use crate::verdict::{Observed, Verdict, observed};

/// How the cited snapshot relates to the one judged against.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Relation {
    /// The referent contains the cited snapshot.
    Contains,
    /// The referent is an ancestor of the cited snapshot.
    Behind,
    /// Neither contains the other.
    Diverged,
    /// Not checked, or the cited snapshot is not here.
    Unknown,
}

/// Whether a verdict says the cited content changed, moved or went.
pub fn claims_change(v: Verdict) -> bool {
    matches!(
        v,
        Verdict::Changed | Verdict::Gone | Verdict::Moved | Verdict::Renamed
    )
}

/// The verdict a referent can support: none from an unresolved referent,
/// and no change from one behind the cited snapshot.
pub fn settle(resolved: bool, relation: Relation, said: Verdict) -> Verdict {
    if !resolved {
        return Verdict::Unverifiable;
    }
    match relation {
        Relation::Behind if claims_change(said) => Verdict::Unverifiable,
        Relation::Behind | Relation::Contains | Relation::Diverged | Relation::Unknown => said,
    }
}

/// A provider's facts to a verdict.
pub fn judge(resolved: bool, relation: Relation, fact: Observed) -> Verdict {
    settle(resolved, relation, observed(fact))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Verdict; 9] = [
        Verdict::Fresh,
        Verdict::Moved,
        Verdict::Renamed,
        Verdict::Changed,
        Verdict::Gone,
        Verdict::Expired,
        Verdict::Unverifiable,
        Verdict::Marked,
        Verdict::Due,
    ];
    const RELATIONS: [Relation; 4] = [
        Relation::Contains,
        Relation::Behind,
        Relation::Diverged,
        Relation::Unknown,
    ];

    #[test]
    fn settle_says_no_more_than_the_referent_can() {
        for v in ALL {
            for rel in RELATIONS {
                assert_eq!(settle(false, rel, v), Verdict::Unverifiable);
                let want = match (rel, claims_change(v)) {
                    (Relation::Behind, true) => Verdict::Unverifiable,
                    _ => v,
                };
                assert_eq!(settle(true, rel, v), want, "{rel:?} {v:?}");
            }
        }
    }

    /// #196: the boundary names no Git concept, so the next provider
    /// inherits it unchanged.
    #[test]
    fn the_boundary_names_no_git_concept() {
        let source = include_str!("referent.rs");
        let body = source.split("#[cfg(test)]").next().unwrap_or(source);
        let words: alloc::vec::Vec<&str> = body
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        for git in [
            "git", "Git", "remote", "ref", "clone", "origin", "branch", "commit",
        ] {
            assert!(!words.contains(&git), "referent.rs names {git}");
        }
    }

    #[test]
    fn judge_is_observed_then_settled() {
        let gone = Observed::State(None);
        assert_eq!(observed(gone), Verdict::Gone);
        assert_eq!(judge(true, Relation::Contains, gone), Verdict::Gone);
        assert_eq!(judge(true, Relation::Behind, gone), Verdict::Unverifiable);
        assert_eq!(
            judge(false, Relation::Contains, gone),
            Verdict::Unverifiable
        );
    }
}
