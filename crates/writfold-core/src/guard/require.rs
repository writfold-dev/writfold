//! Each requirement's fact, from what the caller found when it looked it
//! up: whether there was anything to look at, and whether it could be
//! read. A requirement that could not be judged is [`Fact::Unknown`],
//! which never passes ([`super::decide`]).
use super::Fact;

/// Whether the thing a requirement judges was looked at, and could be read.
pub enum Looked {
    /// There was nothing to look at (no current run).
    No,
    /// It could not be read or judged.
    Failed,
    Read,
}

/// `run.current`: met only for a current run that is open.
pub fn run_current(found: bool, open: bool) -> Fact {
    if found && open {
        Fact::Met
    } else {
        Fact::Unmet
    }
}

/// `step.met`: with no current run, unmet; when the plan or the step's
/// gates could not be read, unknown; otherwise met when the run has no
/// plan, the plan has no current step, or the current step owes nothing.
pub fn step_fact(read: Looked, planned: bool, has_current: bool, owed: usize) -> Fact {
    match read {
        Looked::No => Fact::Unmet,
        Looked::Failed => Fact::Unknown,
        Looked::Read => {
            if !planned || !has_current || owed == 0 {
                Fact::Met
            } else {
                Fact::Unmet
            }
        }
    }
}

/// `step.settled` (#79): [`step_fact`], except that a current loop step
/// whose rounds are all used without meeting its gates (spent) is also
/// settled: nothing is left for the session to do there but report. An
/// unread plan is never settled.
pub fn settled_fact(
    read: Looked,
    planned: bool,
    has_current: bool,
    owed: usize,
    spent: bool,
) -> Fact {
    match read {
        Looked::No => Fact::Unmet,
        Looked::Failed => Fact::Unknown,
        Looked::Read => {
            if spent {
                return Fact::Met;
            }
            step_fact(Looked::Read, planned, has_current, owed)
        }
    }
}

/// `gate:<id>`: unknown unless the gates were judged; then met only when
/// the gate is defined and met.
pub fn gate_fact(read: Looked, defined: bool, met: bool) -> Fact {
    match read {
        Looked::No | Looked::Failed => Fact::Unknown,
        Looked::Read => {
            if defined && met {
                Fact::Met
            } else {
                Fact::Unmet
            }
        }
    }
}

/// `gates.met`: unknown unless the gates were judged; then
/// [`super::gates_fact`] of the counts.
pub fn gates_read(read: Looked, owed: usize, unmet: usize, invalid: usize) -> Fact {
    match read {
        Looked::No | Looked::Failed => Fact::Unknown,
        Looked::Read => super::gates_fact(owed, unmet, invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::{Looked, gate_fact, gates_read, run_current, step_fact};
    use crate::guard::Fact;

    fn looks() -> [Looked; 3] {
        [Looked::No, Looked::Failed, Looked::Read]
    }

    #[test]
    fn facts_are_met_only_when_read_and_holding() {
        for found in [false, true] {
            for open in [false, true] {
                assert_eq!(run_current(found, open) == Fact::Met, found && open);
            }
        }
        for read in looks() {
            let was_read = matches!(read, Looked::Read);
            for planned in [false, true] {
                for current in [false, true] {
                    for owed in 0..3 {
                        let got = step_fact(Looked::Read, planned, current, owed);
                        let want = !planned || !current || owed == 0;
                        assert_eq!(got == Fact::Met, want);
                    }
                }
            }
            for defined in [false, true] {
                for met in [false, true] {
                    let got = gate_fact(clone(&read), defined, met);
                    assert_eq!(got == Fact::Met, was_read && defined && met);
                    assert_eq!(got == Fact::Unknown, !was_read);
                }
            }
            let got = gates_read(clone(&read), 1, 0, 0);
            assert_eq!(got == Fact::Met, was_read);
        }
        assert_eq!(step_fact(Looked::No, true, true, 0), Fact::Unmet);
        assert_eq!(step_fact(Looked::Failed, false, false, 0), Fact::Unknown);
    }

    fn clone(read: &Looked) -> Looked {
        match read {
            Looked::No => Looked::No,
            Looked::Failed => Looked::Failed,
            Looked::Read => Looked::Read,
        }
    }
}
