//! Which decision state changes `status --set` writes, and what each one
//! must be given. The caller reads the record, checks readiness, resolves
//! the commit and grounds the evidence; nothing here does IO.
//!
//! | From | To | Needs |
//! |---|---|---|
//! | draft | proposed | readiness |
//! | proposed | draft | nothing |
//! | proposed | accepted | readiness and an actor (`--by`) |
//! | proposed | rejected | an actor |
//! | accepted | deferred | nothing |
//! | accepted | applied | a commit on the target ref (`--commit`) |
//! | applied | verified | grounded evidence (`--evidence`) |
//! | any but superseded | superseded | a successor |
//!
//! `landing` and `regressed` are never written.
//!
//! `step` is in the Charon `start_from` list (function by function, so
//! the derives and `name`/`parse` are never translated):
//! `proofs/Writfold/Decision.lean` proves it is exactly this table.

/// A decision's lifecycle state; `Other` is any status outside the
/// vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Draft,
    Proposed,
    Accepted,
    Deferred,
    Rejected,
    Landing,
    Applied,
    Verified,
    Regressed,
    Superseded,
    Other,
}

pub const STATES: [State; 11] = [
    State::Draft,
    State::Proposed,
    State::Accepted,
    State::Deferred,
    State::Rejected,
    State::Landing,
    State::Applied,
    State::Verified,
    State::Regressed,
    State::Superseded,
    State::Other,
];

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Draft => "draft",
            State::Proposed => "proposed",
            State::Accepted => "accepted",
            State::Deferred => "deferred",
            State::Rejected => "rejected",
            State::Landing => "landing",
            State::Applied => "applied",
            State::Verified => "verified",
            State::Regressed => "regressed",
            State::Superseded => "superseded",
            State::Other => "other",
        }
    }

    /// The state a status names; `Other` for anything else.
    pub fn parse(text: &str) -> State {
        let mut index = 0;
        while index < STATES.len() {
            let state = STATES[index];
            if !matches!(state, State::Other) && state.name() == text {
                return state;
            }
            index += 1;
        }
        State::Other
    }
}

/// What a written transition must be given.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Needs {
    /// The review readiness `proposed` requires.
    pub ready: bool,
    /// Who decided (`--by`).
    pub actor: bool,
    /// A commit that is on the target ref (`--commit`).
    pub commit: bool,
    /// Evidence grounded now (`--evidence`).
    pub evidence: bool,
    /// An existing successor decision (`--superseded-by`).
    pub successor: bool,
}

/// The requirements, field by field.
pub fn needs(ready: bool, actor: bool, commit: bool, evidence: bool, successor: bool) -> Needs {
    Needs {
        ready,
        actor,
        commit,
        evidence,
        successor,
    }
}

/// What going from `from` to `to` needs, or `None` when Writfold does not
/// write that change. Matched on the target first, then on where it may
/// come from (one flat match each, for the translation).
pub fn step(from: State, to: State) -> Option<Needs> {
    match to {
        State::Superseded => match from {
            State::Superseded => None,
            _ => Some(needs(false, false, false, false, true)),
        },
        State::Proposed => match from {
            State::Draft => Some(needs(true, false, false, false, false)),
            _ => None,
        },
        State::Draft => match from {
            State::Proposed => Some(needs(false, false, false, false, false)),
            _ => None,
        },
        State::Accepted => match from {
            State::Proposed => Some(needs(true, true, false, false, false)),
            _ => None,
        },
        State::Rejected => match from {
            State::Proposed => Some(needs(false, true, false, false, false)),
            _ => None,
        },
        State::Deferred => match from {
            State::Accepted => Some(needs(false, false, false, false, false)),
            _ => None,
        },
        State::Applied => match from {
            State::Accepted => Some(needs(false, false, true, false, false)),
            _ => None,
        },
        State::Verified => match from {
            State::Applied => Some(needs(false, false, false, true, false)),
            _ => None,
        },
        State::Landing | State::Regressed | State::Other => None,
    }
}

/// What `status --set` was given: `--by`, `--commit`, `--evidence`,
/// `--run` or `--repo`, `--superseded-by`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Given {
    pub actor: bool,
    pub commit: bool,
    pub evidence: bool,
    pub run_or_repo: bool,
    pub successor: bool,
}

/// The flags, field by field.
pub fn given_flags(
    actor: bool,
    commit: bool,
    evidence: bool,
    run_or_repo: bool,
    successor: bool,
) -> Given {
    Given {
        actor,
        commit,
        evidence,
        run_or_repo,
        successor,
    }
}

/// Why the flags given do not fit the change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flag {
    /// `accepted` and `rejected` need `--by`.
    NeedsActor,
    /// `applied` needs `--commit`.
    NeedsCommit,
    /// `--commit` goes only with `applied`.
    CommitNotTaken,
    /// `verified` needs `--evidence`.
    NeedsEvidence,
    /// `--evidence` goes only with `verified`.
    EvidenceNotTaken,
    /// `--run` and `--repo` go only with `verified`.
    RunOrRepoNotTaken,
    /// `--superseded-by` goes only with `superseded`.
    SuccessorNotTaken,
}

/// The first flag that does not fit the change, in this order: a missing
/// actor, commit or evidence, then a flag the change does not take.
/// `None` when each flag it needs is given and none it does not take.
pub fn flags(needs: Needs, got: Given) -> Option<Flag> {
    if needs.actor && !got.actor {
        return Some(Flag::NeedsActor);
    }
    if needs.commit && !got.commit {
        return Some(Flag::NeedsCommit);
    }
    if !needs.commit && got.commit {
        return Some(Flag::CommitNotTaken);
    }
    if needs.evidence && !got.evidence {
        return Some(Flag::NeedsEvidence);
    }
    if !needs.evidence && got.evidence {
        return Some(Flag::EvidenceNotTaken);
    }
    if !needs.evidence && got.run_or_repo {
        return Some(Flag::RunOrRepoNotTaken);
    }
    if !needs.successor && got.successor {
        return Some(Flag::SuccessorNotTaken);
    }
    None
}

/// What the reads established: the record is ready for review, the commit
/// is on its target ref, the evidence grounds now, the successor exists.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Facts {
    pub ready: bool,
    pub on_ref: bool,
    pub grounded: bool,
    pub successor: bool,
}

/// The facts, field by field.
pub fn facts(ready: bool, on_ref: bool, grounded: bool, successor: bool) -> Facts {
    Facts {
        ready,
        on_ref,
        grounded,
        successor,
    }
}

/// Whether the change is written: every fact it needs holds.
pub fn writes(needs: Needs, facts: Facts) -> bool {
    (!needs.ready || facts.ready)
        && (!needs.commit || facts.on_ref)
        && (!needs.evidence || facts.grounded)
        && (!needs.successor || facts.successor)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every pair of states: exactly the table's changes are written, each
    /// with exactly its requirements.
    #[test]
    fn exhaustive() {
        for from in STATES {
            for to in STATES {
                let got = step(from, to);
                let want = match (from.name(), to.name()) {
                    ("superseded", "superseded") => None,
                    (_, "superseded") => Some((false, false, false, false, true)),
                    ("draft", "proposed") => Some((true, false, false, false, false)),
                    ("proposed", "draft") => Some((false, false, false, false, false)),
                    ("proposed", "accepted") => Some((true, true, false, false, false)),
                    ("proposed", "rejected") => Some((false, true, false, false, false)),
                    ("accepted", "deferred") => Some((false, false, false, false, false)),
                    ("accepted", "applied") => Some((false, false, true, false, false)),
                    ("applied", "verified") => Some((false, false, false, true, false)),
                    _ => None,
                };
                let got = got.map(|n| (n.ready, n.actor, n.commit, n.evidence, n.successor));
                assert_eq!(got, want, "{} -> {}", from.name(), to.name());
            }
        }
    }

    #[test]
    fn never_written() {
        for from in STATES {
            for to in [State::Landing, State::Regressed, State::Other] {
                assert_eq!(step(from, to), None);
            }
            // Nothing skips a stage: verified only from applied, applied
            // only from accepted, accepted only from proposed.
            for (to, only) in [
                (State::Verified, State::Applied),
                (State::Applied, State::Accepted),
                (State::Accepted, State::Proposed),
            ] {
                assert_eq!(step(from, to).is_some(), from == only);
            }
        }
    }

    #[test]
    fn flags_and_facts() {
        let accept = needs(true, true, false, false, false);
        let none = given_flags(false, false, false, false, false);
        assert_eq!(flags(accept, none), Some(Flag::NeedsActor));
        let by = given_flags(true, false, false, false, false);
        assert_eq!(flags(accept, by), None);
        let apply = needs(false, false, true, false, false);
        assert_eq!(flags(apply, none), Some(Flag::NeedsCommit));
        assert_eq!(
            flags(accept, given_flags(true, true, false, false, false)),
            Some(Flag::CommitNotTaken)
        );
        let verify = needs(false, false, false, true, false);
        assert_eq!(flags(verify, none), Some(Flag::NeedsEvidence));
        assert_eq!(
            flags(verify, given_flags(false, false, true, true, false)),
            None
        );
        assert_eq!(
            flags(accept, given_flags(true, false, true, false, false)),
            Some(Flag::EvidenceNotTaken)
        );
        assert_eq!(
            flags(accept, given_flags(true, false, false, true, false)),
            Some(Flag::RunOrRepoNotTaken)
        );
        assert_eq!(
            flags(accept, given_flags(true, false, false, false, true)),
            Some(Flag::SuccessorNotTaken)
        );
        assert!(writes(accept, facts(true, false, false, false)));
        assert!(!writes(accept, facts(false, true, true, true)));
        assert!(!writes(verify, facts(true, true, false, true)));
        assert!(writes(
            needs(false, false, false, false, false),
            Facts::default()
        ));
    }

    #[test]
    fn names_round_trip() {
        for state in STATES {
            if state != State::Other {
                assert_eq!(State::parse(state.name()), state);
            }
        }
        assert_eq!(State::parse("other"), State::Other);
        assert_eq!(State::parse("Accepted"), State::Other);
    }
}
