//! The run graph's decisions behind `writfold kb plan`: which step is
//! current, and whether the current step may be recorded as done. Nothing
//! here reads a file or a clock; the caller folds the run's plan events
//! into one ordered list of steps (post-order, so a step's spliced children
//! come before it) and supplies, for each gate of a step, whether it is met
//! (satisfied, attested or waived).
//!
//! In the Charon `start_from` list: the proofs gate translates this module
//! to Lean and `proofs/Writfold/Plan.lean` proves its theorems for every
//! input (docs/writfold/aeneas.md): `current` is the least open step,
//! `advance_check` admits exactly the current step with every gate met in
//! the fixed refusal order, `stale` never moves the current step, `splice`
//! keeps every existing step in order with the done prefix, and
//! `splice_check` is idempotent on its key and bounded in depth. The
//! exhaustive tests below state the same properties for small inputs.

use crate::gate::Standing;
use crate::policy::Accept;
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

/// One gate of a step as the caller found it: no usable definition, or
/// the standing the gate kernel gives it with the standings its policy
/// accepts.
pub enum GateFact {
    /// The gate has no usable definition: never met.
    Undefined,
    Stands(Standing, Accept),
}

/// Whether one gate is met: it stands, and its standing is met under its
/// policy ([`crate::policy::met`]).
pub fn fact_met(fact: &GateFact) -> bool {
    match fact {
        GateFact::Undefined => false,
        GateFact::Stands(standing, accept) => crate::policy::met(*standing, accept),
    }
}

/// Each gate's [`fact_met`], in order.
pub fn met_all(gates: &[GateFact]) -> Vec<bool> {
    let mut met: Vec<bool> = Vec::new();
    let mut index = 0;
    while index < gates.len() {
        met.push(fact_met(&gates[index]));
        index += 1;
    }
    met
}

/// [`advance_check`] on the step's gates as found: an undefined gate is
/// owed, and a step with no gates owes nothing.
pub fn advance(nodes: &[Node], at: usize, gates: &[GateFact]) -> Result<(), Refusal> {
    advance_check(nodes, at, &met_all(gates))
}

/// A done step whose gates no longer all hold: shown `!stale`. It stays
/// done, and the current step does not move back.
pub fn stale(node: &Node, met: &[bool]) -> bool {
    node.done && first_owed(met).is_some()
}

/// Why `plan skip` is refused, in the fixed order it is checked: the plan
/// is complete; the step named is not the current one; the step is locked
/// by the run's pinned policy (`unskippable`).
pub enum SkipRefusal {
    Complete,
    NotCurrent(usize),
    Locked,
}

/// Whether step `at` may be skipped: it is the current step, and the
/// run's pin does not lock it.
pub fn skip_check(nodes: &[Node], at: usize, locked: bool) -> Result<(), SkipRefusal> {
    let now = match current(nodes) {
        None => return Err(SkipRefusal::Complete),
        Some(now) => now,
    };
    if now != at {
        return Err(SkipRefusal::NotCurrent(now));
    }
    if locked {
        return Err(SkipRefusal::Locked);
    }
    Ok(())
}

/// Whether a step counts as skipped: a skip record exists and the run's
/// pin does not lock the step. A skip of a locked step counts for nothing.
pub fn skipped(recorded: bool, locked: bool) -> bool {
    recorded && !locked
}

/// Whether closing the plan's last open step is refused: when `last_open`
/// (the step being closed is the last open one), the first required
/// template that is not `present` in the plan; otherwise nothing.
pub fn close_check(last_open: bool, present: &[bool]) -> Option<usize> {
    if !last_open {
        return None;
    }
    first_owed(present)
}

/// Whether `key` is one of `keys`.
pub fn contains(keys: &[Vec<u8>], key: &[u8]) -> bool {
    let mut index = 0;
    while index < keys.len() {
        if crate::bytes::same(&keys[index], key) {
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
mod tests;
