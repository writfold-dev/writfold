use super::{
    GateFact, MAX_DEPTH, Node, Refusal, SkipRefusal, SpliceRefusal, Spliced, advance,
    advance_check, close_check, closed, current, first_owed, skip_check, skipped, splice,
    splice_check, stale,
};
use crate::gate::{Standing, State};
use crate::policy::Accept;
use alloc::vec::Vec;

/// Every list of up to `max` steps, each open, done, skipped or both.
fn lists(max: usize) -> Vec<Vec<(bool, bool)>> {
    let mut out: Vec<Vec<(bool, bool)>> = Vec::new();
    out.push(Vec::new());
    let mut from = 0;
    for _ in 0..max {
        let to = out.len();
        for i in from..to {
            for state in [(false, false), (true, false), (false, true), (true, true)] {
                let mut next = out[i].clone();
                next.push(state);
                out.push(next);
            }
        }
        from = to;
    }
    out
}

fn nodes(states: &[(bool, bool)]) -> Vec<Node> {
    states
        .iter()
        .map(|&(done, skipped)| Node { done, skipped })
        .collect()
}

/// Every met vector of up to three gates.
fn mets() -> Vec<Vec<bool>> {
    let mut out = Vec::new();
    for n in 0..=3usize {
        for bits in 0..(1usize << n) {
            out.push((0..n).map(|i| bits & (1 << i) != 0).collect());
        }
    }
    out
}

#[test]
fn current_is_the_least_open_step() {
    for states in lists(4) {
        let list = nodes(&states);
        let expected = list.iter().position(|n| !closed(n));
        assert_eq!(current(&list), expected, "{states:?}");
        if let Some(i) = current(&list) {
            assert!(list[..i].iter().all(closed));
        } else {
            assert!(list.iter().all(closed));
        }
    }
}

#[test]
fn advance_is_admitted_exactly_for_the_current_step_with_every_gate_met() {
    for states in lists(4) {
        let list = nodes(&states);
        for at in 0..=list.len() {
            for met in mets() {
                let admitted = advance_check(&list, at, &met).is_ok();
                let expected = current(&list) == Some(at) && met.iter().all(|m| *m);
                assert_eq!(admitted, expected, "{states:?} at={at} met={met:?}");
                // The fixed refusal order.
                match advance_check(&list, at, &met) {
                    Ok(()) => {}
                    Err(Refusal::Complete) => assert!(current(&list).is_none()),
                    Err(Refusal::NotCurrent(now)) => {
                        assert_eq!(current(&list), Some(now));
                        assert_ne!(now, at);
                    }
                    Err(Refusal::Owed(gate)) => {
                        assert_eq!(current(&list), Some(at));
                        assert_eq!(first_owed(&met), Some(gate));
                        assert!(!met[gate] && met[..gate].iter().all(|m| *m));
                    }
                }
            }
        }
    }
}

#[test]
fn an_admitted_advance_moves_current_strictly_later() {
    for states in lists(4) {
        let mut list = nodes(&states);
        let Some(at) = current(&list) else {
            continue;
        };
        assert!(advance_check(&list, at, &[]).is_ok());
        list[at].done = true;
        assert!(current(&list).is_none_or(|next| next > at));
    }
}

#[test]
fn stale_is_a_done_step_with_an_owed_gate() {
    for states in lists(3) {
        for met in mets() {
            for node in nodes(&states) {
                let expected = node.done && met.iter().any(|m| !*m);
                assert_eq!(stale(&node, &met), expected);
                // A stale step is still closed: it never becomes current again.
                assert!(!stale(&node, &met) || closed(&node));
            }
        }
    }
}

fn states(list: &[Node]) -> Vec<(bool, bool)> {
    list.iter().map(|n| (n.done, n.skipped)).collect()
}

#[test]
fn splice_keeps_every_existing_step_and_the_done_prefix() {
    for before in lists(4) {
        let list = nodes(&before);
        for count in 0..=3usize {
            let admitted = splice_check(&list, &[], b"t", 1);
            // No keys: never already; depth 1: never too deep.
            assert!(!matches!(
                admitted,
                Ok(Spliced::Already) | Err(SpliceRefusal::TooDeep)
            ));
            let Ok(Spliced::Insert(at)) = admitted else {
                assert!(!list.is_empty() && current(&list).is_none());
                continue;
            };
            assert_eq!(Some(at), current(&list).or(Some(0)));
            let after = splice(&list, at, count);
            let after_states = states(&after);
            assert_eq!(after.len(), list.len() + count);
            // The prefix (all closed) and the suffix are unchanged.
            assert_eq!(after_states[..at], before[..at]);
            assert_eq!(after_states[at + count..], before[at..]);
            assert!(after[at..at + count].iter().all(|n| !closed(n)));
            // The current step is the first new one, or unchanged.
            let expected = if count > 0 { Some(at) } else { current(&list) };
            assert_eq!(current(&after), expected, "{before:?} +{count}");
        }
    }
}

#[test]
fn splice_is_idempotent_on_its_key_and_bounded_in_depth() {
    let keys = [b"review@1".to_vec(), b"change@2".to_vec()];
    for before in lists(3) {
        let list = nodes(&before);
        for key in [&b"review@1"[..], b"change@2"] {
            for depth in 0..=MAX_DEPTH + 2 {
                // A key already spliced is a no-op, whatever else holds.
                assert!(matches!(
                    splice_check(&list, &keys, key, depth),
                    Ok(Spliced::Already)
                ));
            }
        }
        for key in [&b"review@2"[..], b"review", b"other@1", b""] {
            for depth in 0..=MAX_DEPTH + 2 {
                let admitted = splice_check(&list, &keys, key, depth);
                assert!(!matches!(admitted, Ok(Spliced::Already)), "{key:?}");
                match admitted {
                    Ok(Spliced::Already) => {}
                    Err(SpliceRefusal::TooDeep) => assert!(depth > MAX_DEPTH),
                    Err(SpliceRefusal::Complete) => assert!(current(&list).is_none()),
                    Ok(Spliced::Insert(at)) => {
                        assert!(depth <= MAX_DEPTH);
                        assert_eq!(Some(at), current(&list).or(Some(0)));
                    }
                }
            }
        }
    }
}

/// Every list of up to four steps, every position and both lock values:
/// a skip is admitted exactly for the current, unlocked step, and refused
/// in the fixed order complete, not current, locked.
#[test]
fn skip_is_admitted_exactly_for_the_current_unlocked_step() {
    for states in lists(4) {
        let list = nodes(&states);
        let now = current(&list);
        for at in 0..=states.len() {
            for locked in [false, true] {
                let got = skip_check(&list, at, locked);
                match (now, got) {
                    (None, Err(SkipRefusal::Complete)) => {}
                    (Some(n), Err(SkipRefusal::NotCurrent(m))) => {
                        assert!(n == m && n != at, "{states:?} {at}");
                    }
                    (Some(n), Err(SkipRefusal::Locked)) => assert!(n == at && locked),
                    (Some(n), Ok(())) => assert!(n == at && !locked),
                    _ => unreachable!("{states:?} at={at} locked={locked}"),
                }
            }
        }
    }
}

/// A skip record of a locked step never closes it, so the step stays the
/// current one when every step before it is closed.
#[test]
fn a_locked_skip_never_closes() {
    for recorded in [false, true] {
        assert_eq!(skipped(recorded, false), recorded);
        assert!(!skipped(recorded, true));
        let node = Node {
            done: false,
            skipped: skipped(recorded, true),
        };
        assert!(!closed(&node));
        let list = [
            Node {
                done: true,
                skipped: false,
            },
            node,
        ];
        assert_eq!(current(&list), Some(1));
    }
}

#[test]
fn close_check_names_the_first_missing_template_on_the_last_step() {
    for met in mets() {
        assert_eq!(close_check(false, &met), None);
        assert_eq!(close_check(true, &met), met.iter().position(|b| !*b));
    }
}

/// `advance` judges each gate under its own policy: a waiver that the
/// policy does not accept leaves the gate owed.
#[test]
fn advance_judges_gates_under_their_policy() {
    let list = [Node {
        done: false,
        skipped: false,
    }];
    let strict = || Accept {
        satisfied: true,
        attested: true,
        waived: false,
    };
    let all = || Accept {
        satisfied: true,
        attested: true,
        waived: true,
    };
    let waived = [GateFact::Stands(Standing::Waived, strict())];
    assert!(matches!(advance(&list, 0, &waived), Err(Refusal::Owed(0))));
    let waived = [GateFact::Stands(Standing::Waived, all())];
    assert!(matches!(advance(&list, 0, &waived), Ok(())));
    let passed = [GateFact::Stands(
        Standing::Checked(State::Satisfied),
        strict(),
    )];
    assert!(matches!(advance(&list, 0, &passed), Ok(())));
}
