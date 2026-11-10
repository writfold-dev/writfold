//! Dolt-backed publication decisions: the typed adapter over
//! `writfold_core::publication`. That kernel is a proof candidate; no theorem
//! is claimed about it yet. The application orchestrates the native steps
//! (`writfold_app::publication`) and calls exactly these decisions.
use writfold_core::publication as core;
pub use writfold_core::publication::{Before, Facts};

/// The decision before any write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    Proceed,
    Conflict,
    Misinterpreted,
    AlreadyPrepared,
}

pub fn admit(before: &Before) -> Admission {
    match core::admit(before) {
        core::Admission::Proceed => Admission::Proceed,
        core::Admission::Conflict => Admission::Conflict,
        core::Admission::Misinterpreted => Admission::Misinterpreted,
        core::Admission::AlreadyPrepared => Admission::AlreadyPrepared,
    }
}

/// How an interrupted publication resolves from observation alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Resolution {
    Published,
    NotPublished,
    /// Completable; `prepared` says whether the prepared commit exists.
    Interrupted {
        prepared: bool,
    },
    Diverged,
}

pub fn resolve(facts: &Facts) -> Resolution {
    match core::resolve(facts) {
        core::Resolution::Published => Resolution::Published,
        core::Resolution::NotPublished => Resolution::NotPublished,
        core::Resolution::Interrupted(prepared) => Resolution::Interrupted { prepared },
        core::Resolution::Diverged => Resolution::Diverged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(bits: u8) -> Facts {
        let bit = |n: u8| bits & (1 << n) != 0;
        Facts {
            published: bit(0),
            prepared: bit(1),
            prepared_names_applied: bit(2),
            applied: bit(3),
            head_is_applied: bit(4),
            completed_is_base: bit(5),
        }
    }

    /// Over every combination: `Published` needs the Dolt commit and the
    /// prepared commit naming it; `NotPublished` never leaves a Dolt commit or
    /// a prepared commit behind; completion is offered only when the Git ref is
    /// still the base and the Dolt head is exactly this request's commit.
    #[test]
    fn resolution_never_splits_git_and_dolt() {
        for bits in 0..64 {
            let f = facts(bits);
            match resolve(&f) {
                Resolution::Published => {
                    assert!(f.published && f.applied && f.prepared && f.prepared_names_applied);
                }
                Resolution::NotPublished => {
                    assert!(!f.published && !f.applied && !f.prepared);
                }
                Resolution::Interrupted { prepared } => {
                    assert!(!f.published && f.applied && f.head_is_applied);
                    assert!(f.completed_is_base && prepared == f.prepared);
                    assert!(!prepared || f.prepared_names_applied);
                }
                Resolution::Diverged => {}
            }
        }
    }

    #[test]
    fn admission_checks_conflict_first() {
        let before = |completed_is_base, base_pin_matches, prepared| Before {
            completed_is_base,
            base_pin_matches,
            prepared,
        };
        assert_eq!(admit(&before(false, false, true)), Admission::Conflict);
        assert_eq!(admit(&before(true, false, true)), Admission::Misinterpreted);
        assert_eq!(admit(&before(true, true, true)), Admission::AlreadyPrepared);
        assert_eq!(admit(&before(true, true, false)), Admission::Proceed);
    }
}
