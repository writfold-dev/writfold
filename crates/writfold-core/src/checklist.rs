//! Checklist gates behind `writfold kb gate cover`: a gate of class
//! `checklist` declares items, and each item is covered by a row someone
//! records: `supported` (with references to evidence), `unsupported` or
//! `not-applicable` (each with a reason), or `none-found` (citing recorded
//! looks that found nothing, with a reason). This kernel decides what one
//! item stands on, from its newest row compared with the present, and
//! what the items add up to. Nothing here reads a row or a reference; the
//! caller gathers the facts and picks the newest row (by `seq`).
//!
//! A covered checklist is reported through the gate kernel as an attested
//! obligation ([`crate::gate::standing`]): a cover row is someone's
//! judgement with checked evidence, never a passing check.
//!
//! Which outcomes cover an item is the team's policy: `not-applicable`
//! only for the items the checklist lists in `not_applicable` (every item
//! when the key is absent), `none-found` only for those in `none_found`
//! ([`covered_under`]).
//!
//! `item`, `covered_under`, `owes`, `first_owed`, `stale`, `any_stale`,
//! `rollup_under`, `state` and `standing` are in the Charon `start_from` list (named one by one, so
//! the derives and the text conveniences are never translated);
//! `proofs/Writfold/Checklist.lean` proves them for every input
//! (docs/writfold/aeneas.md). The exhaustive tests below state the same
//! properties for small inputs.

use crate::gate::{Standing, State, Statements};

/// What a cover row says about its item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// The item holds, and the row cites evidence for it.
    Supported,
    /// The item does not hold (with a reason).
    Unsupported,
    /// The item does not apply here (with a reason).
    NotApplicable,
    /// Recorded looks found nothing (citing them, with a reason): never a
    /// claim that nothing exists.
    NoneFound,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Supported => "supported",
            Outcome::Unsupported => "unsupported",
            Outcome::NotApplicable => "not-applicable",
            Outcome::NoneFound => "none-found",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "supported" => Some(Outcome::Supported),
            "unsupported" => Some(Outcome::Unsupported),
            "not-applicable" => Some(Outcome::NotApplicable),
            "none-found" => Some(Outcome::NoneFound),
            _ => None,
        }
    }
}

/// The newest cover row of one item, compared with the present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Row {
    pub outcome: Outcome,
    /// The row is about the gate's current definition and the present
    /// content in its scope.
    pub current: bool,
    /// How many references the row cites.
    pub evidence: usize,
    /// Every reference still stands as recorded and is grounded.
    pub grounded: bool,
}

/// What one item stands on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Item {
    /// No row.
    Open,
    /// The newest row is current and supported by grounded evidence.
    Supported,
    /// The newest row is current and says the item does not apply.
    NotApplicable,
    /// The newest row is current and cites at least one look that still
    /// reads as having found nothing.
    NoneFound,
    /// The newest row is current and says the item does not hold.
    Unsupported,
    /// The newest row is about other content or another definition, or
    /// its evidence moved.
    Stale,
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Open => "open",
            Item::Supported => "supported",
            Item::NotApplicable => "not-applicable",
            Item::NoneFound => "none-found",
            Item::Unsupported => "unsupported",
            Item::Stale => "stale",
        }
    }
}

/// The standing of one item from its newest row: none is `Open`; a row
/// that is not current is `Stale`; a `Supported` or `NoneFound` row that
/// cites nothing, or whose evidence is not all grounded as recorded, is
/// `Stale`; otherwise the row's outcome.
pub fn item(newest: Option<Row>) -> Item {
    let row = match newest {
        None => return Item::Open,
        Some(row) => row,
    };
    if !row.current {
        return Item::Stale;
    }
    match row.outcome {
        Outcome::Supported => {
            if row.evidence > 0 && row.grounded {
                Item::Supported
            } else {
                Item::Stale
            }
        }
        Outcome::Unsupported => Item::Unsupported,
        Outcome::NotApplicable => Item::NotApplicable,
        Outcome::NoneFound => {
            if row.evidence > 0 && row.grounded {
                Item::NoneFound
            } else {
                Item::Stale
            }
        }
    }
}

/// Whether an item counts as covered under its policy: supported; not
/// applicable where the policy accepts that (`na`); none found where it
/// accepts that (`none`).
pub fn covered_under(item: Item, na: bool, none: bool) -> bool {
    match item {
        Item::Supported => true,
        Item::NotApplicable => na,
        Item::NoneFound => none,
        Item::Open | Item::Unsupported | Item::Stale => false,
    }
}

/// What the items of one checklist add up to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rollup {
    /// Every required item is covered.
    pub covered: bool,
    /// Some required item is stale: it was covered for other content, or
    /// its evidence moved.
    pub stale: bool,
    /// The first required item that is not covered.
    pub first_owed: Option<usize>,
}

/// One declared item as it stands, with whether the checklist requires it
/// and which of the reasoned outcomes its policy accepts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    pub item: Item,
    pub required: bool,
    /// `not-applicable` covers this item.
    pub na: bool,
    /// `none-found` covers this item.
    pub none: bool,
}

/// Whether an entry is owed: required and not [`covered_under`] its policy.
pub fn owes(entry: &Entry) -> bool {
    entry.required && !covered_under(entry.item, entry.na, entry.none)
}

/// The first owed entry, in the checklist's order.
pub fn first_owed(entries: &[Entry]) -> Option<usize> {
    let mut index = 0;
    while index < entries.len() {
        if owes(&entries[index]) {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Whether an entry is required and stale.
pub fn stale(entry: &Entry) -> bool {
    entry.required && matches!(entry.item, Item::Stale)
}

/// Whether some required entry is stale.
pub fn any_stale(entries: &[Entry]) -> bool {
    let mut index = 0;
    while index < entries.len() {
        if stale(&entries[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Add up the entries under their policy. Optional items never change
/// whether the checklist is covered or stale.
pub fn rollup_under(entries: &[Entry]) -> Rollup {
    let first = first_owed(entries);
    Rollup {
        covered: first.is_none(),
        stale: any_stale(entries),
        first_owed: first,
    }
}

/// The checked state a checklist reports: `stale` when a required item
/// is stale, else `open` (a checklist is never `satisfied` by a check).
pub fn state(rollup: Rollup) -> State {
    if rollup.stale {
        State::Stale
    } else {
        State::Open
    }
}

/// A checklist's standing through the gate kernel, as an attested
/// obligation: attested while every required item is covered, stale when
/// a required item is stale, waived by a current waiver, else open.
pub fn standing(rollup: Rollup, waived_current: bool) -> Standing {
    let said = Statements {
        attested_current: rollup.covered,
        attested_any: rollup.stale,
        waived_current,
    };
    crate::gate::standing(true, state(rollup), said)
}

#[cfg(test)]
mod tests;
