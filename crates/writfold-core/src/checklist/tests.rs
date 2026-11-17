use super::{Entry, Item, Outcome, Row, covered_under, item, rollup_under};
use alloc::vec::Vec;

const OUTCOMES: [Outcome; 4] = [
    Outcome::Supported,
    Outcome::Unsupported,
    Outcome::NotApplicable,
    Outcome::NoneFound,
];
const ITEMS: [Item; 6] = [
    Item::Open,
    Item::Supported,
    Item::NotApplicable,
    Item::NoneFound,
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
            row.current && row.grounded && row.evidence > 0 && row.outcome == Outcome::Supported,
            "{row:?}"
        );
        assert_eq!(
            got == Item::NoneFound,
            row.current && row.grounded && row.evidence > 0 && row.outcome == Outcome::NoneFound,
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

/// Over every item and policy: supported always covers; not-applicable
/// only where accepted; none-found only where accepted; nothing else.
#[test]
fn covered_under_follows_the_policy() {
    for i in ITEMS {
        for na in [false, true] {
            for none in [false, true] {
                let want = i == Item::Supported
                    || (i == Item::NotApplicable && na)
                    || (i == Item::NoneFound && none);
                assert_eq!(covered_under(i, na, none), want, "{i:?} {na} {none}");
            }
        }
    }
}

type Cell = (Item, bool, bool, bool);

/// Every list of up to three items, each required or not and each with
/// every policy: covered exactly when every required item is covered
/// under its policy; stale exactly when a required item is stale; the
/// first owed item is the first required item not covered; and
/// removing the optional items never changes the result.
#[test]
fn rollup_counts_required_items_only() {
    let mut cells: Vec<Cell> = Vec::new();
    for item in ITEMS {
        for required in [false, true] {
            for na in [false, true] {
                for none in [false, true] {
                    cells.push((item, required, na, none));
                }
            }
        }
    }
    let mut lists: Vec<Vec<Cell>> = alloc::vec![Vec::new()];
    let mut last: Vec<Vec<Cell>> = alloc::vec![Vec::new()];
    for _ in 0..3 {
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
    assert_eq!(lists.len(), 1 + 48 + 48 * 48 + 48 * 48 * 48);
    let entries = |list: &[Cell]| -> Vec<Entry> {
        list.iter()
            .map(|&(item, required, na, none)| Entry {
                item,
                required,
                na,
                none,
            })
            .collect()
    };
    let cov = |c: &Cell| covered_under(c.0, c.2, c.3);
    for list in &lists {
        let got = rollup_under(&entries(list));
        let required: Vec<(usize, Cell)> = list
            .iter()
            .enumerate()
            .filter(|(_, c)| c.1)
            .map(|(i, c)| (i, *c))
            .collect();
        assert_eq!(
            got.covered,
            required.iter().all(|(_, c)| cov(c)),
            "{list:?}"
        );
        assert_eq!(
            got.stale,
            required.iter().any(|(_, c)| c.0 == Item::Stale),
            "{list:?}"
        );
        assert_eq!(
            got.first_owed,
            required.iter().find(|(_, c)| !cov(c)).map(|(at, _)| *at),
            "{list:?}"
        );
        let only: Vec<Cell> = list.iter().copied().filter(|c| c.1).collect();
        let without = rollup_under(&entries(&only));
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
