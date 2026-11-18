//! Evidence binding for checklist items: whether the references a cover
//! row cites answer the item they cover, not only whether they are
//! grounded. A checklist item may declare what answers it (`evidence` in
//! the gate definition); the default, [`Need::Any`], is the behaviour
//! before binding existed: any grounded reference answers any item.
//!
//! The caller classifies each cited reference against the item
//! ([`Bound`]) and says whether it is grounded; nothing here reads a row,
//! an observation or a definition. Binding narrows [`crate::checklist::item`]
//! and never widens it: a row that does not stand without binding never
//! stands with it.
//!
//! What binding does not decide: whether evidence is semantically about the
//! item, or whether the conclusion follows. `for` is the recorder's own word;
//! only [`Bound::Named`] (the item names the evidence) is checked from both
//! sides.
//!
//! `answers`, `answered`, `answered_from` and `item_under` are in the Charon
//! `start_from` list; `proofs/Writfold/Bind.lean` proves them for every input
//! (docs/writfold/aeneas.md).

use crate::checklist::{Item, Row, item};

/// How one cited reference relates to the item it is cited for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bound {
    /// The item names it: a receipt or standing of a gate the item's
    /// `evidence` list names.
    Named,
    /// It names this item: an observation recorded `--for` this item.
    This,
    /// It names another item: an observation recorded `--for` another item.
    Other,
    /// Neither names the other: a citation token, a retrieval, an
    /// observation recorded without `--for`, a gate the item does not name.
    Unbound,
}

/// What an item declares answers it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Need {
    /// Any grounded reference (no `evidence` key).
    Any,
    /// Evidence of its own: a look recorded for this item, or a gate the
    /// item names (`evidence = "own"`).
    Own,
    /// Only the gates the item names (`evidence = ["gate:<id>", …]`).
    Named,
}

/// One cited reference, as the caller found it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ref {
    /// It grounds now (see [`crate::ground`]).
    pub grounded: bool,
    pub bound: Bound,
}

/// Whether a reference bound this way answers an item with this need.
pub fn answers(need: Need, bound: Bound) -> bool {
    match need {
        Need::Any => true,
        Need::Own => match bound {
            Bound::Named | Bound::This => true,
            Bound::Other | Bound::Unbound => false,
        },
        Need::Named => match bound {
            Bound::Named => true,
            Bound::This | Bound::Other | Bound::Unbound => false,
        },
    }
}

/// Whether some reference from `index` on is grounded and answers the item.
pub fn answered_from(need: Need, refs: &[Ref], index: usize) -> bool {
    let mut at = index;
    while at < refs.len() {
        if refs[at].grounded && answers(need, refs[at].bound) {
            return true;
        }
        at += 1;
    }
    false
}

/// Whether some cited reference is grounded and answers the item.
pub fn answered(need: Need, refs: &[Ref]) -> bool {
    answered_from(need, refs, 0)
}

/// The standing of one item under its need: [`item`] of the newest row,
/// except that a `supported` or `none-found` standing whose references do
/// not answer the item is `stale`, as a row whose evidence moved is.
pub fn item_under(newest: Option<Row>, answered: bool) -> Item {
    let base = item(newest);
    match base {
        Item::Supported | Item::NoneFound => {
            if answered {
                base
            } else {
                Item::Stale
            }
        }
        Item::Open | Item::NotApplicable | Item::Unsupported | Item::Stale => base,
    }
}

#[cfg(test)]
mod tests {
    use super::{Bound, Need, Ref, answered, answers, item_under};
    use crate::checklist::{Item, Outcome, Row, item};
    use alloc::vec::Vec;

    const BOUNDS: [Bound; 4] = [Bound::Named, Bound::This, Bound::Other, Bound::Unbound];
    const NEEDS: [Need; 3] = [Need::Any, Need::Own, Need::Named];

    #[test]
    fn answers_by_need() {
        for b in BOUNDS {
            assert!(answers(Need::Any, b));
            assert_eq!(
                answers(Need::Own, b),
                matches!(b, Bound::Named | Bound::This)
            );
            assert_eq!(answers(Need::Named, b), b == Bound::Named);
        }
    }

    #[test]
    fn answered_is_some_grounded_answering_ref() {
        let mut all: Vec<Ref> = Vec::new();
        for b in BOUNDS {
            for g in [false, true] {
                all.push(Ref {
                    grounded: g,
                    bound: b,
                });
            }
        }
        for need in NEEDS {
            for mask in 0..(1u32 << all.len()) {
                let refs: Vec<Ref> = (0..all.len())
                    .filter(|i| mask & (1 << i) != 0)
                    .map(|i| all[i])
                    .collect();
                let want = refs.iter().any(|r| r.grounded && answers(need, r.bound));
                assert_eq!(answered(need, &refs), want);
            }
        }
    }

    #[test]
    fn item_under_narrows_item() {
        let outcomes = [
            Outcome::Supported,
            Outcome::Unsupported,
            Outcome::NotApplicable,
            Outcome::NoneFound,
        ];
        let mut rows: Vec<Option<Row>> = Vec::new();
        rows.push(None);
        for outcome in outcomes {
            for current in [false, true] {
                for evidence in 0..2 {
                    for grounded in [false, true] {
                        rows.push(Some(Row {
                            outcome,
                            current,
                            evidence,
                            grounded,
                        }));
                    }
                }
            }
        }
        for row in rows {
            assert_eq!(item_under(row, true), item(row));
            let got = item_under(row, false);
            match item(row) {
                Item::Supported | Item::NoneFound => assert_eq!(got, Item::Stale),
                other => assert_eq!(got, other),
            }
        }
    }
}
