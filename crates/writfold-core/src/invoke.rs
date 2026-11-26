//! Agent invocations (#147, #148, RFC 2026-11-25-agent-vocabulary,
//! calculus §3.8 and §6): whether `agent prepare` may record one.
//!
//! [`prepare_check`] judges six facts the caller classified, in a fixed
//! order, and names the first that does not hold:
//!
//! 1. the run is open;
//! 2. the step named is the run's current step;
//! 3. the profile's closure is admitted ([`crate::compose::closure`]);
//! 4. every plan template a listed skill names is in the run, at the
//!    current step or an ancestor ([`crate::compose::covers`] over the
//!    run's splice keys);
//! 5. the input file was read;
//! 6. when the profile says `packet = "required"`, the packet is complete
//!    ([`crate::packet::complete`]).
//!
//! [`bound`] (#149) is the built-in check `invocation.bound`: a delivery
//! candidate passes only while it names an invocation of this run and step,
//! prepared for the mode's agent under the closure as it is now, with no
//! second report for it. An unreadable record is `Unknown`.
//!
//! Gathering the facts, the record's codec, the package bytes and the
//! write plan are the caller's (tested). In the Charon `start_from` list;
//! `proofs/Writfold/Invoke.lean` proves it for every input
//! (docs/writfold/aeneas.md).
use crate::gate::Outcome;

/// What `agent prepare` found, each fact classified by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Prepare {
    /// The run exists and is not closed.
    pub run_open: bool,
    /// The step named is the run's current step (re-checked under the lock).
    pub current_step: bool,
    /// The profile's closure is admitted.
    pub closure_admitted: bool,
    /// Every plan template a listed skill names is in the run.
    pub plans_in_run: bool,
    /// The input file was read in full.
    pub input_read: bool,
    /// The profile requires a context packet.
    pub packet_required: bool,
    /// The context packet is complete.
    pub packet_complete: bool,
}

/// Why `agent prepare` is refused: the first fact, in the fixed order,
/// that does not hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrepareRefusal {
    RunClosed,
    NotCurrentStep,
    Closure,
    PlanNotInRun,
    InputUnread,
    PacketIncomplete,
}

/// The first of the six facts that does not hold, or admitted.
pub fn prepare_check(facts: &Prepare) -> Result<(), PrepareRefusal> {
    if !facts.run_open {
        return Err(PrepareRefusal::RunClosed);
    }
    if !facts.current_step {
        return Err(PrepareRefusal::NotCurrentStep);
    }
    if !facts.closure_admitted {
        return Err(PrepareRefusal::Closure);
    }
    if !facts.plans_in_run {
        return Err(PrepareRefusal::PlanNotInRun);
    }
    if !facts.input_read {
        return Err(PrepareRefusal::InputUnread);
    }
    if facts.packet_required && !facts.packet_complete {
        return Err(PrepareRefusal::PacketIncomplete);
    }
    Ok(())
}

/// What `invocation.bound` found for one delivery candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Named {
    /// The candidate names an invocation.
    pub names: bool,
    /// The named invocation record reads and is intact.
    pub readable: bool,
    /// The record is of this run.
    pub this_run: bool,
    /// Its step is the step the candidate is judged for.
    pub this_step: bool,
    /// Its agent is the mode's `agent`.
    pub this_agent: bool,
    /// Its closure digest is the profile's present one.
    pub closure_current: bool,
    /// Every live candidate naming it holds the same bytes.
    pub single: bool,
}

/// `invocation.bound`: `Fail` when no invocation is named, `Unknown` when
/// the named record does not read, else `Pass` exactly when every fact
/// holds.
pub fn bound(named: &Named) -> Outcome {
    if !named.names {
        return Outcome::Fail;
    }
    if !named.readable {
        return Outcome::Unknown;
    }
    if named.this_run
        && named.this_step
        && named.this_agent
        && named.closure_current
        && named.single
    {
        Outcome::Pass
    } else {
        Outcome::Fail
    }
}

#[cfg(test)]
mod tests {
    use super::{Named, Prepare, PrepareRefusal, bound, prepare_check};
    use crate::gate::Outcome;

    const BOUND: Named = Named {
        names: true,
        readable: true,
        this_run: true,
        this_step: true,
        this_agent: true,
        closure_current: true,
        single: true,
    };

    #[test]
    fn bound_passes_only_with_every_fact() {
        assert_eq!(bound(&BOUND), Outcome::Pass);
        for facts in [
            Named {
                this_run: false,
                ..BOUND
            },
            Named {
                this_step: false,
                ..BOUND
            },
            Named {
                this_agent: false,
                ..BOUND
            },
            Named {
                closure_current: false,
                ..BOUND
            },
            Named {
                single: false,
                ..BOUND
            },
            Named {
                names: false,
                ..BOUND
            },
        ] {
            assert_eq!(bound(&facts), Outcome::Fail, "{facts:?}");
        }
        let unread = Named {
            readable: false,
            single: false,
            ..BOUND
        };
        assert_eq!(bound(&unread), Outcome::Unknown);
    }

    const ALL: Prepare = Prepare {
        run_open: true,
        current_step: true,
        closure_admitted: true,
        plans_in_run: true,
        input_read: true,
        packet_required: true,
        packet_complete: true,
    };

    #[test]
    fn all_six_admit() {
        assert_eq!(prepare_check(&ALL), Ok(()));
    }

    #[test]
    fn each_fact_refuses_in_order() {
        let cases = [
            (
                Prepare {
                    run_open: false,
                    ..ALL
                },
                PrepareRefusal::RunClosed,
            ),
            (
                Prepare {
                    current_step: false,
                    ..ALL
                },
                PrepareRefusal::NotCurrentStep,
            ),
            (
                Prepare {
                    closure_admitted: false,
                    ..ALL
                },
                PrepareRefusal::Closure,
            ),
            (
                Prepare {
                    plans_in_run: false,
                    ..ALL
                },
                PrepareRefusal::PlanNotInRun,
            ),
            (
                Prepare {
                    input_read: false,
                    ..ALL
                },
                PrepareRefusal::InputUnread,
            ),
            (
                Prepare {
                    packet_complete: false,
                    ..ALL
                },
                PrepareRefusal::PacketIncomplete,
            ),
        ];
        for (facts, want) in cases {
            assert_eq!(prepare_check(&facts), Err(want));
        }
        // The earliest failing fact is named.
        let both = Prepare {
            current_step: false,
            input_read: false,
            ..ALL
        };
        assert_eq!(prepare_check(&both), Err(PrepareRefusal::NotCurrentStep));
    }

    #[test]
    fn a_packet_not_required_is_not_judged() {
        let facts = Prepare {
            packet_required: false,
            packet_complete: false,
            ..ALL
        };
        assert_eq!(prepare_check(&facts), Ok(()));
    }
}
