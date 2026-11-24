//! What grounds a cited piece of evidence, for a checklist gate's cover
//! rows. The caller resolves the reference (a gate, a gate receipt, an
//! attestation, an observation, a citation token) and gathers the facts; the decision is
//! here. The standing words a reference records (`satisfied`,
//! `not-fresh:3`) are the caller's rendering.
//!
//! A waiver is never evidence: a gate that stands only on a waiver grounds
//! nothing. Only a passing check grounds; an attestation is named as such.
//!
//! In the Charon `start_from` list; `proofs/Writfold/Ground.lean` proves
//! every function for every input (docs/writfold/aeneas.md).
use crate::gate::{Standing, State};
use crate::grounding::Grounding;
use crate::observe::{Status, grounds};

/// `gate:<id>`: grounded while the gate is satisfied by a check; attested
/// while it stands on a current attestation; anything else (open, stale,
/// failed, unknown, waived, or no such gate) grounds nothing.
pub fn of_gate(found: bool, standing: Standing) -> Grounding {
    if !found {
        return Grounding::Ungrounded;
    }
    match standing {
        Standing::Checked(State::Satisfied) => Grounding::Grounded,
        Standing::Attested => Grounding::Attested,
        Standing::Checked(_) | Standing::Waived => Grounding::Ungrounded,
    }
}

/// What a cited gate receipt of this run is, compared with the present.
pub struct ReceiptFacts {
    /// It is about this subject repository.
    pub same_subject: bool,
    /// It was made under the gate's current definition.
    pub same_definition: bool,
    /// It is about the present content in the gate's scope.
    pub same_content: bool,
    /// It passed.
    pub passed: bool,
    /// The gate stands met now.
    pub gate_met: bool,
}

/// A gate receipt grounds only while all five hold.
pub fn of_receipt(facts: &ReceiptFacts) -> Grounding {
    if facts.same_subject
        && facts.same_definition
        && facts.same_content
        && facts.passed
        && facts.gate_met
    {
        Grounding::Grounded
    } else {
        Grounding::Ungrounded
    }
}

/// A citation token or locator: grounded only when it reads as exactly one
/// citation and that citation is fresh.
pub fn of_token(count: usize, not_fresh: usize) -> Grounding {
    if count == 1 && not_fresh == 0 {
        Grounding::Grounded
    } else {
        Grounding::Ungrounded
    }
}

/// An observation: grounded only when the look answered in full (`ok`) and
/// its embedded citations, at least one, are all fresh.
pub fn of_observation(status: Status, count: usize, not_fresh: usize) -> Grounding {
    if grounds(status, count > 0 && not_fresh == 0) {
        Grounding::Grounded
    } else {
        Grounding::Ungrounded
    }
}

/// An attestation: attested while it is the gate's current attestation.
pub fn of_statement(current_attestation: bool) -> Grounding {
    if current_attestation {
        Grounding::Attested
    } else {
        Grounding::Ungrounded
    }
}

/// Whether a recorded reference moved: it no longer grounds, or its
/// standing word differs from the one recorded.
pub fn moved(now: Grounding, same_word: bool) -> bool {
    matches!(now, Grounding::Ungrounded) || !same_word
}

#[cfg(test)]
mod tests {
    use super::{ReceiptFacts, moved, of_gate, of_observation, of_receipt, of_statement, of_token};
    use crate::gate::{Standing, State};
    use crate::grounding::Grounding;
    use crate::observe::{ALL, Status};

    #[test]
    fn a_waiver_never_grounds_and_only_a_pass_does() {
        let states = [
            State::Open,
            State::Satisfied,
            State::Stale,
            State::Failed,
            State::Unknown,
        ];
        let mut all: alloc::vec::Vec<Standing> =
            states.iter().map(|s| Standing::Checked(*s)).collect();
        all.push(Standing::Attested);
        all.push(Standing::Waived);
        for s in all {
            assert_eq!(of_gate(false, s), Grounding::Ungrounded);
            let want = match s {
                Standing::Checked(State::Satisfied) => Grounding::Grounded,
                Standing::Attested => Grounding::Attested,
                _ => Grounding::Ungrounded,
            };
            assert_eq!(of_gate(true, s), want);
        }
        for bits in 0..32u8 {
            let f = ReceiptFacts {
                same_subject: bits & 1 != 0,
                same_definition: bits & 2 != 0,
                same_content: bits & 4 != 0,
                passed: bits & 8 != 0,
                gate_met: bits & 16 != 0,
            };
            assert_eq!(of_receipt(&f) == Grounding::Grounded, bits == 31);
        }
        for count in 0..3 {
            for not_fresh in 0..3 {
                let any = count > 0 && not_fresh == 0;
                let one = count == 1 && not_fresh == 0;
                assert_eq!(of_token(count, not_fresh) == Grounding::Grounded, one);
                for status in ALL {
                    let got = of_observation(status, count, not_fresh);
                    assert_eq!(got == Grounding::Grounded, status == Status::Ok && any);
                }
            }
        }
        assert_eq!(of_statement(true), Grounding::Attested);
        assert_eq!(of_statement(false), Grounding::Ungrounded);
        assert!(moved(Grounding::Ungrounded, true));
        assert!(moved(Grounding::Grounded, false));
        assert!(!moved(Grounding::Attested, true));
    }
}
