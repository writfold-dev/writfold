//! Project asset catalogs (#84, RFC
//! `docs/rfcs/2026-11-21-project-asset-catalogs.md`): what a catalog may
//! write to one asset slot, and whether a catalog's whole plan is clean,
//! drift, or invalid. A slot is one target path/ID: whether the catalog's
//! input wants a record there, who owns what is there now (nothing, this
//! catalog, or anything else), and whether the wanted bytes equal the
//! present ones. Collisions between catalogs and with hand-owned records
//! reach the kernel as foreign-owned slots; unreadable or partly listed
//! input as `complete = false`. Translated and proved
//! (`proofs/Writfold/Catalog.lean`); parsing, enumeration, path checks,
//! comparison and patch encoding are tested IO in `writfold-kb`.

/// Who owns what is at a slot now.
pub enum Owner {
    /// Nothing is there.
    Absent,
    /// A record this catalog generated (its ownership marker names it).
    Mine,
    /// A hand-owned record, another catalog's, or any other claim.
    Foreign,
}

/// What a catalog does to one slot.
pub enum Action {
    Keep,
    Create,
    Replace,
    Remove,
    /// The catalog wants a slot something else owns: never written.
    Conflict,
}

/// One target slot as the catalog found it.
pub struct Slot {
    /// The input has a row for it.
    pub wanted: bool,
    pub owner: Owner,
    /// The wanted bytes equal the present ones (meaningful when owned).
    pub equal: bool,
}

/// The whole plan's verdict.
pub enum Verdict {
    /// Input unreadable, partly listed, malformed or conflicting: no patch.
    Invalid,
    /// Valid, with changes to make.
    Drift,
    /// Valid, complete, conflict-free and nothing to change.
    Clean,
}

/// What the catalog does to `slot`: create only where nothing is, replace
/// or remove only its own records, leave everything foreign untouched.
pub fn action(slot: &Slot) -> Action {
    match slot.owner {
        Owner::Absent => {
            if slot.wanted {
                Action::Create
            } else {
                Action::Keep
            }
        }
        Owner::Mine => {
            if !slot.wanted {
                Action::Remove
            } else if slot.equal {
                Action::Keep
            } else {
                Action::Replace
            }
        }
        Owner::Foreign => {
            if slot.wanted {
                Action::Conflict
            } else {
                Action::Keep
            }
        }
    }
}

/// Whether `slot` is a conflict.
pub fn conflicts(slot: &Slot) -> bool {
    match action(slot) {
        Action::Conflict => true,
        Action::Keep => false,
        Action::Create => false,
        Action::Replace => false,
        Action::Remove => false,
    }
}

/// Whether `slot` would be written.
pub fn changes(slot: &Slot) -> bool {
    match action(slot) {
        Action::Create => true,
        Action::Replace => true,
        Action::Remove => true,
        Action::Keep => false,
        Action::Conflict => false,
    }
}

/// Whether any slot is a conflict.
pub fn any_conflict(slots: &[Slot]) -> bool {
    let mut index = 0;
    while index < slots.len() {
        if conflicts(&slots[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether any slot would be written.
pub fn any_change(slots: &[Slot]) -> bool {
    let mut index = 0;
    while index < slots.len() {
        if changes(&slots[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Invalid when the input was not read in full or anything conflicts;
/// otherwise drift exactly when something would change.
pub fn verdict(complete: bool, conflict: bool, changed: bool) -> Verdict {
    if !complete || conflict {
        return Verdict::Invalid;
    }
    if changed {
        return Verdict::Drift;
    }
    Verdict::Clean
}

/// The verdict over every slot of the plan.
pub fn plan(complete: bool, slots: &[Slot]) -> Verdict {
    let conflict = any_conflict(slots);
    let changed = any_change(slots);
    verdict(complete, conflict, changed)
}

/// Whether a patch may be emitted (empty when clean): never for an
/// invalid plan, not even the part that was valid.
pub fn patches(verdict: &Verdict) -> bool {
    match verdict {
        Verdict::Invalid => false,
        Verdict::Drift => true,
        Verdict::Clean => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Owner, Slot, Verdict, action, patches, plan};

    fn slot(wanted: bool, owner: Owner, equal: bool) -> Slot {
        Slot {
            wanted,
            owner,
            equal,
        }
    }

    #[test]
    fn the_slot_table() {
        assert!(matches!(
            action(&slot(true, Owner::Absent, false)),
            Action::Create
        ));
        assert!(matches!(
            action(&slot(false, Owner::Absent, false)),
            Action::Keep
        ));
        assert!(matches!(
            action(&slot(true, Owner::Mine, true)),
            Action::Keep
        ));
        assert!(matches!(
            action(&slot(true, Owner::Mine, false)),
            Action::Replace
        ));
        assert!(matches!(
            action(&slot(false, Owner::Mine, true)),
            Action::Remove
        ));
        assert!(matches!(
            action(&slot(true, Owner::Foreign, true)),
            Action::Conflict
        ));
        assert!(matches!(
            action(&slot(false, Owner::Foreign, false)),
            Action::Keep
        ));
    }

    #[test]
    fn partial_input_or_a_conflict_never_patches() {
        let good = [slot(true, Owner::Absent, false)];
        assert!(matches!(plan(true, &good), Verdict::Drift));
        assert!(!patches(&plan(false, &good)));
        let collided = [
            slot(true, Owner::Absent, false),
            slot(true, Owner::Foreign, false),
        ];
        assert!(!patches(&plan(true, &collided)));
        assert!(matches!(
            plan(true, &[slot(true, Owner::Mine, true)]),
            Verdict::Clean
        ));
        assert!(matches!(plan(true, &[]), Verdict::Clean));
    }
}
