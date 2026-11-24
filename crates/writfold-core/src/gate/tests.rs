use super::{
    Ending, Evidence, Kept, Outcome, Standing, State, Statements, bound, outcome, outcome_kept,
    standing, state,
};

/// Missing evidence only ever blocks a pass; declaring nothing or keeping
/// everything changes nothing, and a failure stays a failure.
#[test]
fn kept_evidence_only_blocks_a_pass() {
    let endings = [
        Ending::Exited(0),
        Ending::Exited(1),
        Ending::Exited(-3),
        Ending::Signaled,
        Ending::TimedOut,
        Ending::Error,
    ];
    for ending in endings {
        for moved in [false, true] {
            let plain = outcome(ending, moved);
            assert_eq!(outcome_kept(ending, moved, Kept::Nothing), plain);
            assert_eq!(outcome_kept(ending, moved, Kept::All), plain);
            let missing = outcome_kept(ending, moved, Kept::Missing);
            assert_ne!(missing, Outcome::Pass);
            let want = if plain == Outcome::Pass {
                Outcome::Unknown
            } else {
                plain
            };
            assert_eq!(missing, want);
        }
    }
}

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

#[test]
fn bound_needs_all_three() {
    for bits in 0..8u8 {
        let (r, s, i) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        assert_eq!(bound(r, s, i), bits == 7);
    }
}

/// Every sequence of up to three receipts over every combination: the
/// state is satisfied only when the newest bound receipt about the
/// current subject passed; stale exactly when bound receipts exist but
/// none is about the current subject; and an unbound receipt changes
/// nothing (the state equals the state of the bound receipts alone).
#[test]
fn a_receipt_about_another_subject_never_satisfies() {
    let outcomes = [Outcome::Pass, Outcome::Fail, Outcome::Unknown];
    let mut all = alloc::vec::Vec::new();
    for outcome in outcomes {
        for same_definition in [false, true] {
            for same_scope in [false, true] {
                for bound in [false, true] {
                    all.push(Evidence {
                        outcome,
                        same_definition,
                        same_scope,
                        bound,
                    });
                }
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
                let current =
                    (seq.iter()).rposition(|e| e.bound && e.same_definition && e.same_scope);
                let only: alloc::vec::Vec<Evidence> =
                    seq.iter().copied().filter(|e| e.bound).collect();
                assert_eq!(state(&only).0, got, "{seq:?}");
                match current {
                    None => match seq.iter().rposition(|e| e.bound) {
                        None => assert_eq!((got, at), (State::Open, None)),
                        Some(n) => assert_eq!((got, at), (State::Stale, Some(n))),
                    },
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
