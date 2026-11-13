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

const NOTHING: Needs = Needs {
    ready: false,
    actor: false,
    commit: false,
    evidence: false,
    successor: false,
};

/// What going from `from` to `to` needs, or `None` when Writfold does not
/// write that change.
pub fn step(from: State, to: State) -> Option<Needs> {
    match (from, to) {
        (State::Superseded, State::Superseded) => None,
        (_, State::Superseded) => Some(Needs {
            successor: true,
            ..NOTHING
        }),
        (State::Draft, State::Proposed) => Some(Needs {
            ready: true,
            ..NOTHING
        }),
        (State::Proposed, State::Draft) => Some(NOTHING),
        (State::Proposed, State::Accepted) => Some(Needs {
            ready: true,
            actor: true,
            ..NOTHING
        }),
        (State::Proposed, State::Rejected) => Some(Needs {
            actor: true,
            ..NOTHING
        }),
        (State::Accepted, State::Deferred) => Some(NOTHING),
        (State::Accepted, State::Applied) => Some(Needs {
            commit: true,
            ..NOTHING
        }),
        (State::Applied, State::Verified) => Some(Needs {
            evidence: true,
            ..NOTHING
        }),
        _ => None,
    }
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
