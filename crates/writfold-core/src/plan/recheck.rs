//! The plan writes' last decisions before a byte is written: whether the
//! run takes plan writes at all, and, under the KB write lock, whether
//! what a write decided from the fold it read still stands on the fold
//! read again (writfold-kb `plan::recheck`). The caller compares step
//! paths and reads the run's state; nothing here does IO.
//!
//! In the Charon `start_from` list; `proofs/Writfold/Recheck.lean` proves
//! `still_stands` case by case and `writable` for every input.

/// What a plan write decided from the fold it read, reduced to what the
/// re-check compares.
pub enum Decided {
    /// The run had no plan (`plan set`, `plan use` on an unplanned run).
    Unplanned,
    /// The step it names was the current one (`plan advance`, `plan skip`).
    Current,
    /// A splice under a parent at `ordinal`; `parent_current`: the parent
    /// was the current step (false for a required end template at the
    /// root, which is not under a current step).
    Splice { parent_current: bool, ordinal: u64 },
}

/// Why a decision no longer stands.
pub enum Moved {
    /// A plan was set meanwhile.
    Planned,
    /// The step the decision names is no longer the current one.
    NotCurrent,
    /// Another splice under the same parent took the ordinal.
    Taken,
}

/// Whether `decided` still stands on the fold read again: `planned`, the
/// run has a plan now; `is_current`, the step the decision names (the
/// step itself, or the splice's parent) is the current step now; `next`,
/// the ordinal the next splice under that parent takes now. `None` when it
/// stands.
pub fn still_stands(
    decided: &Decided,
    planned: bool,
    is_current: bool,
    next: u64,
) -> Option<Moved> {
    match decided {
        Decided::Unplanned => {
            if planned {
                Some(Moved::Planned)
            } else {
                None
            }
        }
        Decided::Current => {
            if is_current {
                None
            } else {
                Some(Moved::NotCurrent)
            }
        }
        Decided::Splice {
            parent_current,
            ordinal,
        } => {
            if *parent_current && !is_current {
                return Some(Moved::NotCurrent);
            }
            if next != *ordinal {
                Some(Moved::Taken)
            } else {
                None
            }
        }
    }
}

/// Whether a run takes plan writes. Each argument is whether a record of
/// the run's state says `closed`, `None` where it says nothing: the run's
/// sidecar wins over its manifest, and a run is open unless the state in
/// force is `closed`.
pub fn writable(sidecar_closed: Option<bool>, manifest_closed: Option<bool>) -> bool {
    match sidecar_closed {
        Some(closed) => !closed,
        None => match manifest_closed {
            Some(closed) => !closed,
            None => true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decision_stands_while_what_it_read_holds() {
        let stands = |d: &Decided, p, c, n| still_stands(d, p, c, n).is_none();
        assert!(stands(&Decided::Unplanned, false, false, 0));
        assert!(matches!(
            still_stands(&Decided::Unplanned, true, true, 0),
            Some(Moved::Planned)
        ));
        assert!(stands(&Decided::Current, true, true, 7));
        assert!(matches!(
            still_stands(&Decided::Current, true, false, 0),
            Some(Moved::NotCurrent)
        ));
        let under = |pc| Decided::Splice {
            parent_current: pc,
            ordinal: 3,
        };
        assert!(stands(&under(true), true, true, 3));
        assert!(matches!(
            still_stands(&under(true), true, false, 3),
            Some(Moved::NotCurrent)
        ));
        assert!(matches!(
            still_stands(&under(true), true, true, 4),
            Some(Moved::Taken)
        ));
        assert!(stands(&under(false), true, false, 3));
        assert!(matches!(
            still_stands(&under(false), true, false, 4),
            Some(Moved::Taken)
        ));
    }

    #[test]
    fn the_sidecar_wins_and_only_closed_refuses() {
        for sidecar in [None, Some(false), Some(true)] {
            for manifest in [None, Some(false), Some(true)] {
                let closed = sidecar.or(manifest).unwrap_or(false);
                assert_eq!(
                    writable(sidecar, manifest),
                    !closed,
                    "{sidecar:?} {manifest:?}"
                );
            }
        }
    }
}
