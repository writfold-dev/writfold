//! Agent vocabulary (#144, RFC 2026-11-25-agent-vocabulary, calculus
//! §3.6): whether an agent profile's closure holds together.
//!
//! - [`covers`] is the first tool a listed skill needs that the profile
//!   does not grant, or `None` when every one is granted.
//! - [`closure`] is the first reference that does not resolve to exactly
//!   one definition of the expected kind, else the first uncovered tool.
//!
//! Reading the TOML files, resolving references into [`RefFact`]s, the
//! closure digest and rendering are the caller's (tested). In the Charon
//! `start_from` list; `proofs/Writfold/Compose.lean` proves every function
//! for every input (docs/writfold/aeneas.md).
use alloc::vec::Vec;

/// What resolving one reference (a skill, a tool, a plan) found, in the
/// order the profile lists them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefFact {
    /// Exactly one definition of the expected kind.
    Found,
    /// No definition with that id.
    Missing,
    /// A definition with that id, of another kind.
    WrongKind,
    /// More than one definition with that id (KB root and repository).
    Twice,
}

/// Why a closure does not hold together: the index of the first fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The reference at this index did not resolve.
    Reference(usize),
    /// The required tool at this index is not granted.
    Tool(usize),
}

/// The least index of a required tool that is not granted.
pub fn covers(required: &[Vec<u8>], granted: &[Vec<u8>]) -> Option<usize> {
    let mut index = 0;
    while index < required.len() {
        if !crate::plan::contains(granted, &required[index]) {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// The first reference that is not [`RefFact::Found`]; else the first
/// required tool that is not granted; else admitted.
pub fn closure(refs: &[RefFact], required: &[Vec<u8>], granted: &[Vec<u8>]) -> Result<(), Refusal> {
    let mut index = 0;
    while index < refs.len() {
        match refs[index] {
            RefFact::Found => {}
            RefFact::Missing | RefFact::WrongKind | RefFact::Twice => {
                return Err(Refusal::Reference(index));
            }
        }
        index += 1;
    }
    match covers(required, granted) {
        Some(tool) => Err(Refusal::Tool(tool)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
