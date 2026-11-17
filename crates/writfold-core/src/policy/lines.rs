//! What `gate lines` says about a planned run, so that it agrees with the
//! plan's word ([`super::summary`]): a gate that only skipped steps name is
//! listed `skipped` and never owed, and the lines exit 0 only when the
//! plan reads `complete` or `complete-with-exceptions`, every other listed
//! gate is met and nothing is invalid. Translated and proved with the rest
//! of the module (`proofs/Writfold/Policy.lean`).

use super::{Close, Summary};

/// Whether a step was skipped.
pub fn is_skipped(close: &Close) -> bool {
    matches!(close, Close::Skipped)
}

/// Whether every step in `owners` (the steps naming one gate) was skipped
/// and there is at least one: such a gate is listed `skipped`.
pub fn all_skipped(owners: &[Close]) -> bool {
    if owners.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < owners.len() {
        if !is_skipped(&owners[index]) {
            return false;
        }
        index += 1;
    }
    true
}

/// Whether the plan's word closes it: `complete` or
/// `complete-with-exceptions`.
pub fn closes(summary: &Summary) -> bool {
    matches!(summary, Summary::Complete) || matches!(summary, Summary::CompleteWithExceptions)
}

/// One listed gate: whether only skipped steps name it, and whether it is
/// met under its policy.
pub struct Listed {
    pub skipped: bool,
    pub met: bool,
}

/// Whether every listed gate that is not skipped is met.
pub fn owed_met(listed: &[Listed]) -> bool {
    let mut index = 0;
    while index < listed.len() {
        if !listed[index].skipped && !listed[index].met {
            return false;
        }
        index += 1;
    }
    true
}

/// Whether `gate lines` exits 0 for a planned run: the plan's word closes
/// it, every gate not skipped is met, and nothing is invalid (`valid`).
pub fn lines_clean(summary: &Summary, listed: &[Listed], valid: bool) -> bool {
    closes(summary) && owed_met(listed) && valid
}

#[cfg(test)]
mod tests {
    use super::{Listed, all_skipped, lines_clean};
    use crate::policy::{Close, Summary};

    #[test]
    fn a_gate_is_skipped_only_when_every_step_naming_it_was() {
        assert!(!all_skipped(&[]));
        assert!(all_skipped(&[Close::Skipped, Close::Skipped]));
        assert!(!all_skipped(&[Close::Skipped, Close::Passed]));
        assert!(!all_skipped(&[Close::Open]));
    }

    #[test]
    fn the_lines_are_clean_only_when_the_plan_closes() {
        let open = || Listed {
            skipped: true,
            met: false,
        };
        let words = [
            (Summary::NoPlan, false),
            (Summary::Incomplete, false),
            (Summary::ClosedIncomplete, false),
            (Summary::PolicyMoved, false),
            (Summary::CompleteStale, false),
            (Summary::CompleteWithExceptions, true),
            (Summary::Complete, true),
        ];
        for (word, clean) in words {
            assert_eq!(lines_clean(&word, &[open()], true), clean);
            assert!(!lines_clean(&word, &[open()], false));
            let owed = Listed {
                skipped: false,
                met: false,
            };
            assert!(!lines_clean(&word, &[open(), owed], true));
        }
    }
}
