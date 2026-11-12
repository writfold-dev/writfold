//! The run graph's decisions behind `writfold kb plan`: which step is
//! current, and whether the current step may be recorded as done. Nothing
//! here reads a file or a clock; the caller folds the run's plan events
//! into one ordered list of steps (post-order, so a step's spliced children
//! come before it) and supplies, for each gate of a step, whether it is met
//! (satisfied, attested or waived).
//!
//! Proof candidate, not yet in the Charon `start_from` list: no theorem is
//! claimed. It is written to the translation rules (owned data, no derives,
//! explicit `while` loops, one early return per loop) so the theorems of
//! the plan kernel can be stated over this code. The exhaustive tests
//! below state them for every list of up to four steps, and for `splice`
//! (insert a template's steps before the current step, as its children)
//! over every insertion of up to three steps.

use alloc::vec::Vec;

/// One step of the folded list: whether a `done-` or a `skip-` record
/// closes it. A step may carry both after a merge; either closes it.
pub struct Node {
    pub done: bool,
    pub skipped: bool,
}

/// Why `advance` is refused, in the fixed order it is checked: the plan
/// is complete; the step named is not the current one; a gate of the
/// current step is owed (the first one, in the order the step lists them).
pub enum Refusal {
    /// Every step is done or skipped.
    Complete,
    /// The step named is not the current step, which is this one.
    NotCurrent(usize),
    /// The current step owes this gate (its index in the step's list).
    Owed(usize),
}

/// A splice nested deeper than this (counting `use=` includes) is refused.
pub const MAX_DEPTH: usize = 4;

/// Why a splice is refused.
pub enum SpliceRefusal {
    /// Every step is done or skipped: there is no current step to splice under.
    Complete,
    /// The new steps would sit deeper than [`MAX_DEPTH`].
    TooDeep,
}

/// What an admitted splice does.
pub enum Spliced {
    /// The same template at the same digest is already spliced at the
    /// current step or one of its ancestors: nothing changes.
    Already,
    /// Insert the new steps at this index: before the current step (the
    /// start of an empty list for a root splice).
    Insert(usize),
}

/// Whether a step is closed: done or skipped.
pub fn closed(node: &Node) -> bool {
    node.done || node.skipped
}

/// The current step: the first that is neither done nor skipped. `None`
/// exactly when every step is closed (or there are none).
pub fn current(nodes: &[Node]) -> Option<usize> {
    let mut index = 0;
    while index < nodes.len() {
        if !closed(&nodes[index]) {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// The first gate that is not met, in list order.
pub fn first_owed(met: &[bool]) -> Option<usize> {
    let mut index = 0;
    while index < met.len() {
        if !met[index] {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Whether step `at` may be recorded as done: it is the current step and
/// every one of its gates is met. A step with no gates is always met.
pub fn advance_check(nodes: &[Node], at: usize, met: &[bool]) -> Result<(), Refusal> {
    let now = match current(nodes) {
        None => return Err(Refusal::Complete),
        Some(now) => now,
    };
    if now != at {
        return Err(Refusal::NotCurrent(now));
    }
    match first_owed(met) {
        Some(gate) => Err(Refusal::Owed(gate)),
        None => Ok(()),
    }
}

/// A done step whose gates no longer all hold: shown `!stale`. It stays
/// done, and the current step does not move back.
pub fn stale(node: &Node, met: &[bool]) -> bool {
    node.done && first_owed(met).is_some()
}

/// Whether two keys are the same bytes.
pub fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Whether `key` is one of `keys`.
pub fn contains(keys: &[Vec<u8>], key: &[u8]) -> bool {
    let mut index = 0;
    while index < keys.len() {
        if same(&keys[index], key) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether a splice of the template `key` (its name and digest) may be
/// written, in the fixed order it is checked: `spliced` holds the keys
/// already spliced at the current step or its ancestors (empty for
/// `--again`), and a match is [`Spliced::Already`]; then `depth`, the
/// depth of the deepest new step, must be at most [`MAX_DEPTH`]; then an
/// empty list takes a root splice at 0, and otherwise the steps go before
/// the current step, which must exist.
pub fn splice_check(
    nodes: &[Node],
    spliced: &[Vec<u8>],
    key: &[u8],
    depth: usize,
) -> Result<Spliced, SpliceRefusal> {
    if contains(spliced, key) {
        return Ok(Spliced::Already);
    }
    if depth > MAX_DEPTH {
        return Err(SpliceRefusal::TooDeep);
    }
    if nodes.is_empty() {
        return Ok(Spliced::Insert(0));
    }
    match current(nodes) {
        None => Err(SpliceRefusal::Complete),
        Some(at) => Ok(Spliced::Insert(at)),
    }
}

/// The list after inserting `count` open steps at `at`: every existing
/// step keeps its state and relative order. `at` past the end appends.
pub fn splice(nodes: &[Node], at: usize, count: usize) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::new();
    let mut index = 0;
    while index < nodes.len() && index < at {
        out.push(Node {
            done: nodes[index].done,
            skipped: nodes[index].skipped,
        });
        index += 1;
    }
    let mut added = 0;
    while added < count {
        out.push(Node {
            done: false,
            skipped: false,
        });
        added += 1;
    }
    while index < nodes.len() {
        out.push(Node {
            done: nodes[index].done,
            skipped: nodes[index].skipped,
        });
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_DEPTH, Node, Refusal, SpliceRefusal, Spliced, advance_check, closed, current,
        first_owed, splice, splice_check, stale,
    };
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
}
