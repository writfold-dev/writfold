//! Exhaustive small-input checks of the round kernel beside its proofs.
use super::{
    AgainRefusal, Bounds, Exit, Next, Place, Refusal, advance_check, again_check, bounds_ok, exit,
    has_exit, next, over, stale, superseded, superseded_any,
};
use crate::plan::Node;
use alloc::vec;
use alloc::vec::Vec;

fn open() -> Node {
    Node {
        done: false,
        skipped: false,
    }
}

fn done() -> Node {
    Node {
        done: true,
        skipped: false,
    }
}

#[test]
fn bounds_and_exits() {
    assert!(bounds_ok(&Bounds { min: 1, max: 32 }));
    assert!(!bounds_ok(&Bounds { min: 0, max: 3 }));
    assert!(!bounds_ok(&Bounds { min: 4, max: 3 }));
    assert!(!bounds_ok(&Bounds { min: 1, max: 33 }));
    assert!(!has_exit(&Bounds { min: 1, max: 3 }, 0));
    assert!(has_exit(&Bounds { min: 3, max: 3 }, 0));
    let b = Bounds { min: 2, max: 3 };
    assert!(matches!(exit(&b, 1, &[true]), Exit::Again));
    assert!(matches!(exit(&b, 2, &[true]), Exit::Met));
    assert!(matches!(exit(&b, 2, &[false]), Exit::Again));
    assert!(matches!(exit(&b, 3, &[false]), Exit::Spent));
}

#[test]
fn again_and_advance_never_both_and_spent_is_neither() {
    let nodes = vec![done(), open()];
    for min in 1..=3usize {
        for max in min..=3usize {
            let b = Bounds { min, max };
            for rounds in 0..=4usize {
                for met in [vec![], vec![true], vec![false], vec![true, false]] {
                    let again = again_check(&nodes, 1, true, &b, rounds, &met);
                    let advance = advance_check(&nodes, 1, &b, rounds, &met);
                    assert!(!(again.is_ok() && advance.is_ok()));
                    match exit(&b, rounds, &met) {
                        Exit::Spent => {
                            assert!(matches!(again, Err(AgainRefusal::Spent)));
                            assert!(advance.is_err());
                        }
                        Exit::Met => assert!(advance.is_ok()),
                        Exit::Again => {
                            assert!(matches!(again, Ok(n) if n == rounds + 1 && n <= max));
                        }
                    }
                }
            }
        }
    }
    let b = Bounds { min: 1, max: 3 };
    assert!(matches!(
        again_check(&nodes, 0, true, &b, 1, &[false]),
        Err(AgainRefusal::NotCurrent(1))
    ));
    assert!(matches!(
        again_check(&nodes, 1, false, &b, 1, &[false]),
        Err(AgainRefusal::NotLoop)
    ));
    assert!(matches!(
        again_check(&[done()], 0, true, &b, 1, &[false]),
        Err(AgainRefusal::Complete)
    ));
    let more = Bounds { min: 3, max: 3 };
    assert!(matches!(
        advance_check(&nodes, 1, &more, 1, &[true]),
        Err(Refusal::Rounds(2))
    ));
}

#[test]
fn history_next_and_over() {
    assert!(superseded(&Place {
        round: 1,
        rounds: 2
    }));
    assert!(!superseded(&Place {
        round: 2,
        rounds: 2
    }));
    let path: Vec<Place> = vec![
        Place {
            round: 2,
            rounds: 2,
        },
        Place {
            round: 1,
            rounds: 3,
        },
    ];
    assert!(superseded_any(&path));
    assert!(!stale(&done(), &[false], true));
    assert!(stale(&done(), &[false], false));
    assert!(over(&Bounds { min: 1, max: 2 }, 3));
    let b = Bounds { min: 1, max: 2 };
    assert!(matches!(next(&b, 1, &[true], &[true]), Next::Advance));
    assert!(matches!(next(&b, 1, &[false], &[false]), Next::Judge(0)));
    assert!(matches!(next(&b, 1, &[false], &[true]), Next::Again));
    assert!(matches!(next(&b, 2, &[false], &[true]), Next::Report));
}
