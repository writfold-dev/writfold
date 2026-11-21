//! The strict handoff decision (#57): a candidate is accepted only when
//! the inventory of required gates is complete, at least one gate is
//! required, and every required gate has the approved definition and a
//! current, mechanically passing result. There is no input for a plan,
//! a skip, an exception or a gate's `accept`: a statement (attested,
//! waived) is never a pass here. Translated and proved
//! (`proofs/Writfold/Handoff.lean`); reading the policy revision, the
//! candidate tree and the receipts is tested IO in `writfold-kb`.

use crate::gate::{Standing, State};

/// Why one required gate, or the whole set, does not pass.
pub enum Reason {
    /// No gate is required.
    Empty,
    /// The required set or its definitions could not all be read.
    Incomplete,
    /// A required gate has no definition at the policy revision.
    Undefined,
    /// Its evidence was made under another definition than the approved one.
    DefinitionChanged,
    /// It has no receipt.
    NoEvidence,
    /// Its newest receipt is about other content than the candidate's.
    Stale,
    /// It ran and did not pass (failed, or unknown).
    NotPassed,
    /// Only a statement stands for it (attested or waived), not a check.
    Statement,
}

/// One required gate as the receiver found it.
pub struct Item {
    /// Its definition exists at the policy revision.
    pub defined: bool,
    /// Its evidence was made under the approved definition.
    pub approved: bool,
    /// Its raw standing against the approved definition and the
    /// candidate's content (without the gate's `accept`).
    pub standing: Standing,
}

/// The receiver's answer.
pub enum Verdict {
    Accepted,
    /// The index of the first failing item (0 for `Empty` and
    /// `Incomplete`), and why.
    Refused(usize, Reason),
}

/// Why `item` does not pass, or `None` when it does. A statement is
/// named before anything else about the evidence; a receipt made under
/// another definition is named before staleness, which it also causes.
pub fn item(item: &Item) -> Option<Reason> {
    if !item.defined {
        return Some(Reason::Undefined);
    }
    if matches!(item.standing, Standing::Attested) || matches!(item.standing, Standing::Waived) {
        return Some(Reason::Statement);
    }
    if matches!(item.standing, Standing::Checked(State::Open)) {
        return Some(Reason::NoEvidence);
    }
    if !item.approved {
        return Some(Reason::DefinitionChanged);
    }
    match item.standing {
        Standing::Checked(State::Satisfied) => None,
        Standing::Checked(State::Stale) => Some(Reason::Stale),
        Standing::Checked(State::Failed) => Some(Reason::NotPassed),
        Standing::Checked(State::Unknown) => Some(Reason::NotPassed),
        Standing::Checked(State::Open) => Some(Reason::NoEvidence),
        Standing::Attested => Some(Reason::Statement),
        Standing::Waived => Some(Reason::Statement),
    }
}

/// The first failing item, with why.
pub fn first_failing(items: &[Item]) -> Option<(usize, Reason)> {
    let mut at = 0;
    while at < items.len() {
        if let Some(reason) = item(&items[at]) {
            return Some((at, reason));
        }
        at += 1;
    }
    None
}

/// Accept only a complete, non-empty set whose every item passes.
pub fn strict(complete: bool, items: &[Item]) -> Verdict {
    if !complete {
        return Verdict::Refused(0, Reason::Incomplete);
    }
    if items.is_empty() {
        return Verdict::Refused(0, Reason::Empty);
    }
    match first_failing(items) {
        Some((at, reason)) => Verdict::Refused(at, reason),
        None => Verdict::Accepted,
    }
}

/// [`strict`] over evidence read in full (#67): when an evidence directory
/// of the run could not be listed in full, or a receipt, statement or cover
/// in it does not read, the newest evidence is not known, so the candidate
/// is refused as incomplete before any gate is judged.
pub fn checked(evidence_complete: bool, complete: bool, items: &[Item]) -> Verdict {
    if !evidence_complete {
        return Verdict::Refused(0, Reason::Incomplete);
    }
    strict(complete, items)
}

#[cfg(test)]
mod tests {
    use super::{Item, Reason, Verdict, checked, strict};
    use crate::gate::{Standing, State};

    fn passing() -> Item {
        Item {
            defined: true,
            approved: true,
            standing: Standing::Checked(State::Satisfied),
        }
    }

    fn refused(v: &Verdict) -> Option<(usize, &Reason)> {
        match v {
            Verdict::Accepted => None,
            Verdict::Refused(at, why) => Some((*at, why)),
        }
    }

    #[test]
    fn incomplete_evidence_refuses_a_passing_set() {
        assert!(matches!(
            refused(&checked(false, true, &[passing()])),
            Some((0, Reason::Incomplete))
        ));
        assert!(matches!(checked(true, true, &[passing()]), Verdict::Accepted));
    }

    #[test]
    fn only_a_complete_nonempty_set_of_current_passes_is_accepted() {
        assert!(matches!(
            strict(true, &[passing(), passing()]),
            Verdict::Accepted
        ));
        assert!(matches!(
            refused(&strict(true, &[])),
            Some((0, Reason::Empty))
        ));
        assert!(matches!(
            refused(&strict(false, &[passing()])),
            Some((0, Reason::Incomplete))
        ));
        let statement = |standing| Item {
            standing,
            ..passing()
        };
        for (i, why) in [
            (statement(Standing::Attested), "statement"),
            (statement(Standing::Waived), "statement"),
            (statement(Standing::Checked(State::Open)), "no-evidence"),
            (statement(Standing::Checked(State::Stale)), "stale"),
            (statement(Standing::Checked(State::Failed)), "not-passed"),
            (
                Item {
                    approved: false,
                    standing: Standing::Checked(State::Stale),
                    ..passing()
                },
                "changed",
            ),
            (
                Item {
                    defined: false,
                    ..passing()
                },
                "undefined",
            ),
        ] {
            let verdict = strict(true, &[passing(), i]);
            assert!(matches!(refused(&verdict), Some((1, _))), "{why}");
        }
    }
}
