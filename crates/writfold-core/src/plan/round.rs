//! Loops (#75, `docs/rfcs/2026-11-21-loops.md`): a plan step that repeats
//! a template until its own gates are met, at most `max` rounds and at
//! least `min`. Writfold never runs a round: these decide whether the agent
//! may record another round (`plan again`), whether the loop step may be
//! advanced, what its `next:` line names, and which earlier rounds are
//! history. Translated and proved (`proofs/Writfold/Round.lean`); reading
//! the step words, counting rounds and the write are tested IO in
//! `writfold-kb`.

use super::{Node, Refusal as PlanRefusal, advance_check as plan_advance, current, first_owed};

/// The most rounds a loop may declare.
pub const MAX_ROUNDS: usize = 32;

/// A loop's bounds: at least `min` rounds, at most `max`.
pub struct Bounds {
    pub min: usize,
    pub max: usize,
}

/// `1 <= min <= max <= MAX_ROUNDS`.
pub fn bounds_ok(b: &Bounds) -> bool {
    1 <= b.min && b.min <= b.max && b.max <= MAX_ROUNDS
}

/// A loop can end on something of its own: a gate on the step, or a
/// `min` above one (a count).
pub fn has_exit(b: &Bounds, gates: usize) -> bool {
    gates > 0 || b.min > 1
}

/// Where a loop stands.
pub enum Exit {
    /// Enough rounds, and every gate of the step is met: advance.
    Met,
    /// Another round may be recorded.
    Again,
    /// Every round was used and a gate is still not met.
    Spent,
}

/// Every gate is met.
pub fn all_met(met: &[bool]) -> bool {
    first_owed(met).is_none()
}

/// `Met` when `rounds >= min` and every gate is met; else `Again` while
/// `rounds < max`; else `Spent`.
pub fn exit(b: &Bounds, rounds: usize, met: &[bool]) -> Exit {
    if rounds >= b.min && all_met(met) {
        return Exit::Met;
    }
    if rounds < b.max {
        return Exit::Again;
    }
    Exit::Spent
}

/// Why `plan again` is refused, in the order it is checked.
pub enum AgainRefusal {
    /// Every step is closed.
    Complete,
    /// The current step is this one, not the loop.
    NotCurrent(usize),
    /// The current step is not a loop.
    NotLoop,
    /// The loop's exit holds: advance instead.
    Met,
    /// Every round was used.
    Spent,
}

/// Whether the next round may be recorded under step `at` (`looped`: the
/// step declares a loop); `Ok` carries its number, `rounds + 1`.
pub fn again_check(
    nodes: &[Node],
    at: usize,
    looped: bool,
    b: &Bounds,
    rounds: usize,
    met: &[bool],
) -> Result<usize, AgainRefusal> {
    let now = match current(nodes) {
        None => return Err(AgainRefusal::Complete),
        Some(now) => now,
    };
    if now != at {
        return Err(AgainRefusal::NotCurrent(now));
    }
    if !looped {
        return Err(AgainRefusal::NotLoop);
    }
    match exit(b, rounds, met) {
        Exit::Met => Err(AgainRefusal::Met),
        Exit::Spent => Err(AgainRefusal::Spent),
        Exit::Again => Ok(rounds + 1),
    }
}

/// Why `plan advance` on a loop step is refused.
pub enum Refusal {
    /// The plan kernel's refusal (complete, not current, a gate owed).
    Plan(PlanRefusal),
    /// Every gate is met, but this many more rounds are owed.
    Rounds(usize),
}

/// `plan advance` on a loop step: the plan kernel's refusals first, then
/// the rounds still owed (`min - rounds`).
pub fn advance_check(
    nodes: &[Node],
    at: usize,
    b: &Bounds,
    rounds: usize,
    met: &[bool],
) -> Result<(), Refusal> {
    match plan_advance(nodes, at, met) {
        Err(refusal) => Err(Refusal::Plan(refusal)),
        Ok(()) => {
            if rounds < b.min {
                return Err(Refusal::Rounds(b.min - rounds));
            }
            Ok(())
        }
    }
}

/// Where a step sits in one loop on its path: in round `round` of
/// `rounds`.
pub struct Place {
    pub round: usize,
    pub rounds: usize,
}

/// A step of a round that is not the newest round of its loop.
pub fn superseded(place: &Place) -> bool {
    place.round < place.rounds
}

/// Some loop on the step's path has a newer round than the step's.
pub fn superseded_any(path: &[Place]) -> bool {
    let mut index = 0;
    while index < path.len() {
        if superseded(&path[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// [`super::stale`], except that a superseded step is history: never
/// stale.
pub fn stale(node: &Node, met: &[bool], superseded: bool) -> bool {
    if superseded {
        return false;
    }
    super::stale(node, met)
}

/// More rounds exist than `max` allows (two clones each added one).
pub fn over(b: &Bounds, rounds: usize) -> bool {
    rounds > b.max
}

/// Whether gate `i` has a verdict about the present (a missing entry has
/// none).
pub fn judged_at(judged: &[bool], i: usize) -> bool {
    i < judged.len() && judged[i]
}

/// What a loop step's `next:` line names.
pub enum Next {
    /// `plan advance`.
    Advance,
    /// Run or judge this gate first: it has no verdict about the present.
    Judge(usize),
    /// `plan again`.
    Again,
    /// Stop and report: the loop is spent.
    Report,
}

/// The `next:` of a loop step. `judged[i]`: gate `i` has a verdict about
/// the present bytes (failed, unknown, an unsupported item), as opposed to
/// open or stale; a missing entry counts as not judged.
pub fn next(b: &Bounds, rounds: usize, met: &[bool], judged: &[bool]) -> Next {
    match exit(b, rounds, met) {
        Exit::Met => Next::Advance,
        Exit::Again => owed_or(met, judged, Next::Again),
        Exit::Spent => owed_or(met, judged, Next::Report),
    }
}

/// The first owed gate when it has no verdict yet, else `then`.
pub fn owed_or(met: &[bool], judged: &[bool], then: Next) -> Next {
    match first_owed(met) {
        Some(i) => {
            if judged_at(judged, i) {
                then
            } else {
                Next::Judge(i)
            }
        }
        None => then,
    }
}

#[cfg(test)]
mod tests;
