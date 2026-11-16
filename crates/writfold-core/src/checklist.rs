//! Checklist gates behind `writfold kb gate cover`: a gate of class
//! `checklist` declares items, and each item is covered by a row someone
//! records: `supported` (with references to evidence), `unsupported` or
//! `not-applicable` (each with a reason). This kernel decides what one
//! item stands on, from its newest row compared with the present, and
//! what the items add up to. Nothing here reads a row or a reference; the
//! caller gathers the facts and picks the newest row (by `seq`).
//!
//! A covered checklist is reported through the gate kernel as an attested
//! obligation ([`crate::gate::standing`]): a cover row is someone's
//! judgement with checked evidence, never a passing check.
//!
//! `item`, `covered`, `owes`, `first_owed`, `stale`, `any_stale`, `rollup`, `state`
//! and `standing` are in the Charon `start_from` list (named one by one, so
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
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Supported => "supported",
            Outcome::Unsupported => "unsupported",
            Outcome::NotApplicable => "not-applicable",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "supported" => Some(Outcome::Supported),
            "unsupported" => Some(Outcome::Unsupported),
            "not-applicable" => Some(Outcome::NotApplicable),
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
            Item::Unsupported => "unsupported",
            Item::Stale => "stale",
        }
    }
}

/// The standing of one item from its newest row: none is `Open`; a row
/// that is not current is `Stale`; a `Supported` row that cites nothing,
/// or whose evidence is not all grounded as recorded, is `Stale`;
/// otherwise the row's outcome.
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
    }
}

/// Whether an item counts as covered: supported, or not applicable.
pub fn covered(item: Item) -> bool {
    match item {
        Item::Supported | Item::NotApplicable => true,
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

/// One declared item as it stands, with whether the checklist requires it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    pub item: Item,
    pub required: bool,
}

/// Whether an entry is owed: required and not covered.
pub fn owes(entry: &Entry) -> bool {
    entry.required && !covered(entry.item)
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

/// Add up the entries. Optional items never change whether the checklist
/// is covered or stale.
pub fn rollup(entries: &[Entry]) -> Rollup {
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
mod tests {
    use super::{Entry, Item, Outcome, Row, covered, item, rollup};
    use alloc::vec::Vec;

    const OUTCOMES: [Outcome; 3] = [
        Outcome::Supported,
        Outcome::Unsupported,
        Outcome::NotApplicable,
    ];
    const ITEMS: [Item; 5] = [
        Item::Open,
        Item::Supported,
        Item::NotApplicable,
        Item::Unsupported,
        Item::Stale,
    ];

    fn rows() -> Vec<Row> {
        let mut all = Vec::new();
        for outcome in OUTCOMES {
            for current in [false, true] {
                for evidence in 0..3 {
                    for grounded in [false, true] {
                        all.push(Row {
                            outcome,
                            current,
                            evidence,
                            grounded,
                        });
                    }
                }
            }
        }
        all
    }

    /// Over every row: `Supported` only when current, grounded and citing
    /// something; a row that is not current is always `Stale`; the other
    /// outcomes need no evidence.
    #[test]
    fn supported_needs_current_grounded_evidence() {
        assert_eq!(item(None), Item::Open);
        for row in rows() {
            let got = item(Some(row));
            assert_ne!(got, Item::Open);
            assert_eq!(
                got == Item::Supported,
                row.current
                    && row.grounded
                    && row.evidence > 0
                    && row.outcome == Outcome::Supported,
                "{row:?}"
            );
            assert_eq!(
                got == Item::NotApplicable,
                row.current && row.outcome == Outcome::NotApplicable,
                "{row:?}"
            );
            assert_eq!(
                got == Item::Unsupported,
                row.current && row.outcome == Outcome::Unsupported,
                "{row:?}"
            );
            if !row.current {
                assert_eq!(got, Item::Stale, "{row:?}");
            }
        }
    }

    /// Every list of up to four items, each required or not: covered
    /// exactly when every required item is supported or not applicable;
    /// stale exactly when a required item is stale; the first owed item
    /// is the first required item not covered; and removing the optional
    /// items never changes the result.
    #[test]
    fn rollup_counts_required_items_only() {
        let mut cells = Vec::new();
        for item in ITEMS {
            for required in [false, true] {
                cells.push((item, required));
            }
        }
        let mut lists: Vec<Vec<(Item, bool)>> = alloc::vec![Vec::new()];
        let mut last: Vec<Vec<(Item, bool)>> = alloc::vec![Vec::new()];
        for _ in 0..4 {
            let mut longer = Vec::new();
            for list in &last {
                for cell in &cells {
                    let mut next = list.clone();
                    next.push(*cell);
                    longer.push(next);
                }
            }
            lists.extend(longer.iter().cloned());
            last = longer;
        }
        assert_eq!(lists.len(), 1 + 10 + 100 + 1000 + 10_000);
        let entries = |list: &[(Item, bool)]| -> Vec<Entry> {
            list.iter()
                .map(|&(item, required)| Entry { item, required })
                .collect()
        };
        for list in &lists {
            let got = rollup(&entries(list));
            let required: Vec<(usize, Item)> = list
                .iter()
                .enumerate()
                .filter(|(_, (_, r))| *r)
                .map(|(i, (item, _))| (i, *item))
                .collect();
            assert_eq!(
                got.covered,
                required.iter().all(|(_, i)| covered(*i)),
                "{list:?}"
            );
            assert_eq!(
                got.stale,
                required.iter().any(|(_, i)| *i == Item::Stale),
                "{list:?}"
            );
            assert_eq!(
                got.first_owed,
                required
                    .iter()
                    .find(|(_, i)| !covered(*i))
                    .map(|(at, _)| *at),
                "{list:?}"
            );
            let only: Vec<(Item, bool)> = list.iter().copied().filter(|(_, r)| *r).collect();
            let without = rollup(&entries(&only));
            assert_eq!(
                (without.covered, without.stale),
                (got.covered, got.stale),
                "{list:?}"
            );
        }
    }

    #[test]
    fn names_round_trip() {
        for outcome in OUTCOMES {
            assert_eq!(Outcome::parse(outcome.name()), Some(outcome));
        }
        assert_eq!(Outcome::parse("met"), None);
        for item in ITEMS {
            assert!(!item.name().is_empty());
        }
    }
}
