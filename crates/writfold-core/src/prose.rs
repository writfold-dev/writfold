//! Prose rules for delivery drafts (`prose.rules`, #127, RFC
//! 2026-11-25-prose-rules-and-advice): what a list of rule matches means.
//! Each match is advice or blocking. Advice is recorded and shown and never
//! decides; any blocking match fails; a rule set with no rules is never a
//! pass. [`agrees`] is the reader's check that a delivery receipt's outcome
//! fits its blocking findings, for every delivery check.
//!
//! Matching text against rules is the caller's (tested); the decision is
//! here. In the Charon `start_from` list; `proofs/Writfold/Prose.lean`
//! proves every function for every input (docs/writfold/aeneas.md).
use crate::gate::Outcome;

/// How much one rule match weighs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Weight {
    /// Shown and recorded; never changes the outcome.
    Advice,
    /// Fails the check.
    Blocking,
}

/// How many matches block.
pub fn blocking(found: &[Weight]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < found.len() {
        match found[index] {
            Weight::Blocking => count += 1,
            Weight::Advice => {}
        }
        index += 1;
    }
    count
}

/// The outcome of a prose check with `rules` rules that found `found`:
/// `unknown` with no rules, `fail` when anything blocks, else `pass`.
pub fn verdict(rules: usize, found: &[Weight]) -> Outcome {
    if rules == 0 {
        return Outcome::Unknown;
    }
    if blocking(found) > 0 {
        Outcome::Fail
    } else {
        Outcome::Pass
    }
}

/// Whether a delivery receipt's outcome fits its count of blocking
/// findings: a pass has none, a failure has some, unknown says nothing.
pub fn agrees(outcome: Outcome, blocking: usize) -> bool {
    match outcome {
        Outcome::Pass => blocking == 0,
        Outcome::Fail => blocking > 0,
        Outcome::Unknown => true,
    }
}

#[cfg(test)]
mod tests;
