//! Gate obligations behind `writfold kb gate`: what one evaluation's ending
//! means, and the obligation state derived from a gate's receipts and
//! statements. Nothing here reads a repository or a receipt; the caller
//! gathers the facts. State is never stored: it is this function of the
//! definition and the receipts, recomputed on every read. Which paths a
//! scope covers is [`crate::scope`].
//!
//! The decisions `outcome`, `state`, `standing` and `Standing::met` are in
//! the Charon `start_from` list (named one by one, so the derived trait
//! impls and the `name`/`parse` text conveniences are never translated);
//! `proofs/Writfold/Gate.lean` proves them for every input
//! (docs/writfold/aeneas.md). The property tests below state the same
//! properties for small inputs.

/// How one evaluation ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ending {
    /// The command exited with this status.
    Exited(i32),
    /// It was killed by a signal it did not ask for (a crash).
    Signaled,
    /// It ran past its timeout and was stopped.
    TimedOut,
    /// It could not be started or supervised.
    Error,
}

/// The outcome a receipt records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pass,
    Fail,
    /// A timeout, crash or error: never evidence of success.
    Unknown,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Unknown => "unknown",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pass" => Some(Outcome::Pass),
            "fail" => Some(Outcome::Fail),
            "unknown" => Some(Outcome::Unknown),
            _ => None,
        }
    }
}

/// Only exit status 0 is a pass; any other exit status is a failure; every
/// other ending is unknown. `subject_moved` (an in-scope path changed while
/// the command ran) makes any ending unknown: the result is about no single
/// subject.
pub fn outcome(ending: Ending, subject_moved: bool) -> Outcome {
    if subject_moved {
        return Outcome::Unknown;
    }
    match ending {
        Ending::Exited(0) => Outcome::Pass,
        Ending::Exited(_) => Outcome::Fail,
        Ending::Signaled | Ending::TimedOut | Ending::Error => Outcome::Unknown,
    }
}

/// A gate's obligation on one subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// No receipt.
    Open,
    /// The newest receipt about the current subject passed.
    Satisfied,
    /// Receipts exist, but none is about the current definition and the
    /// current content of the paths in scope.
    Stale,
    /// The newest receipt about the current subject failed.
    Failed,
    /// The newest receipt about the current subject is unknown.
    Unknown,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Open => "open",
            State::Satisfied => "satisfied",
            State::Stale => "stale",
            State::Failed => "failed",
            State::Unknown => "unknown",
        }
    }
}

/// What one valid receipt says, compared with the present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub outcome: Outcome,
    /// The receipt's definition digest equals the current definition's.
    pub same_definition: bool,
    /// The receipt's scope digest equals the present content in scope.
    pub same_scope: bool,
}

/// The state from a gate's valid receipts, oldest first, and the index of
/// the receipt that decides it: the newest receipt about the current
/// subject, else the newest receipt (which is then stale).
pub fn state(receipts: &[Evidence]) -> (State, Option<usize>) {
    let mut index = receipts.len();
    while index > 0 {
        index -= 1;
        let evidence = receipts[index];
        if evidence.same_definition && evidence.same_scope {
            let state = match evidence.outcome {
                Outcome::Pass => State::Satisfied,
                Outcome::Fail => State::Failed,
                Outcome::Unknown => State::Unknown,
            };
            return (state, Some(index));
        }
    }
    match receipts.len() {
        0 => (State::Open, None),
        n => (State::Stale, Some(n - 1)),
    }
}

/// What stands for an obligation once statements are counted: a checked
/// state, or one of the two statement classes. A statement is never a
/// passing check: `Attested` and `Waived` are their own standings, and no
/// statement ever yields `Checked(Satisfied)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Standing {
    Checked(State),
    /// An attested obligation with a statement about the current subject.
    Attested,
    /// A waiver, with a reason, about the current subject.
    Waived,
}

impl Standing {
    pub fn name(self) -> &'static str {
        match self {
            Standing::Checked(state) => state.name(),
            Standing::Attested => "attested",
            Standing::Waived => "waived",
        }
    }

    /// Whether nothing more is owed: satisfied by a check, attested where
    /// only an attestation is asked for, or waived with a reason.
    pub fn met(self) -> bool {
        !matches!(
            self,
            Standing::Checked(State::Open | State::Stale | State::Failed | State::Unknown)
        )
    }
}

/// The statements recorded for one obligation, compared with the present
/// (the same definition and the same subject content).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Statements {
    /// An attestation about the current subject exists.
    pub attested_current: bool,
    /// Some attestation exists (current or not).
    pub attested_any: bool,
    /// A waiver about the current subject exists.
    pub waived_current: bool,
}

/// The standing of one obligation. An `attested_class` obligation has no
/// check and is met only by a current attestation; every other obligation
/// stands on its checked `state`. A current waiver stands for any
/// obligation that is not already satisfied or attested.
pub fn standing(attested_class: bool, state: State, statements: Statements) -> Standing {
    if attested_class {
        if statements.attested_current {
            return Standing::Attested;
        }
    } else if matches!(state, State::Satisfied) {
        return Standing::Checked(State::Satisfied);
    }
    if statements.waived_current {
        return Standing::Waived;
    }
    match (attested_class, statements.attested_any) {
        (true, true) => Standing::Checked(State::Stale),
        (true, false) => Standing::Checked(State::Open),
        (false, _) => Standing::Checked(state),
    }
}

#[cfg(test)]
mod tests {
    use super::{Ending, Evidence, Outcome, Standing, State, Statements, outcome, standing, state};

    /// Over every input: an attested-class obligation is never satisfied,
    /// a statement never makes anything `Satisfied`, a waiver stands only
    /// when current, and a satisfied check is never shown as waived.
    #[test]
    fn statements_are_never_passing_checks() {
        let states = [
            State::Open,
            State::Satisfied,
            State::Stale,
            State::Failed,
            State::Unknown,
        ];
        for attested_class in [false, true] {
            for state in states {
                for bits in 0..8u8 {
                    let s = Statements {
                        attested_current: bits & 1 != 0,
                        attested_any: bits & 3 != 0,
                        waived_current: bits & 4 != 0,
                    };
                    let got = standing(attested_class, state, s);
                    let satisfied = got == Standing::Checked(State::Satisfied);
                    assert_eq!(satisfied, !attested_class && state == State::Satisfied);
                    assert!(got != Standing::Waived || s.waived_current);
                    assert!(got != Standing::Attested || (attested_class && s.attested_current));
                    let expected = if attested_class {
                        s.attested_current || s.waived_current
                    } else {
                        state == State::Satisfied || s.waived_current
                    };
                    assert_eq!(got.met(), expected, "{attested_class} {state:?} {s:?}");
                }
            }
        }
    }

    #[test]
    fn only_a_clean_zero_exit_passes() {
        for moved in [false, true] {
            for ending in [
                Ending::Exited(0),
                Ending::Exited(1),
                Ending::Exited(-1),
                Ending::Exited(255),
                Ending::Signaled,
                Ending::TimedOut,
                Ending::Error,
            ] {
                let got = outcome(ending, moved);
                let pass = ending == Ending::Exited(0) && !moved;
                assert_eq!(got == Outcome::Pass, pass, "{ending:?} moved={moved}");
                let fail = matches!(ending, Ending::Exited(n) if n != 0) && !moved;
                assert_eq!(got == Outcome::Fail, fail, "{ending:?} moved={moved}");
            }
        }
    }

    /// Every sequence of up to three receipts over every combination: the
    /// state is satisfied only when the newest receipt about the current
    /// subject passed, and stale exactly when receipts exist but none is
    /// about the current subject.
    #[test]
    fn a_receipt_about_another_subject_never_satisfies() {
        let outcomes = [Outcome::Pass, Outcome::Fail, Outcome::Unknown];
        let mut all = alloc::vec::Vec::new();
        for outcome in outcomes {
            for same_definition in [false, true] {
                for same_scope in [false, true] {
                    all.push(Evidence {
                        outcome,
                        same_definition,
                        same_scope,
                    });
                }
            }
        }
        assert_eq!(state(&[]), (State::Open, None));
        let n = all.len();
        for a in 0..n {
            for b in 0..=n {
                for c in 0..=n {
                    let mut seq = alloc::vec![all[a]];
                    seq.extend(all.get(b).copied());
                    seq.extend(all.get(c).copied());
                    let (got, at) = state(&seq);
                    let current = seq.iter().rposition(|e| e.same_definition && e.same_scope);
                    match current {
                        None => assert_eq!((got, at), (State::Stale, Some(seq.len() - 1))),
                        Some(i) => {
                            assert_eq!(at, Some(i));
                            let pass = seq[i].outcome == Outcome::Pass;
                            assert_eq!(got == State::Satisfied, pass);
                        }
                    }
                }
            }
        }
    }
}
