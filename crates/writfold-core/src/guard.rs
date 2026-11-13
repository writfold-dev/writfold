//! Guard rules behind `writfold kb hook pre-tool-use` and `stop`: what one
//! rule decides once its requirements have been looked up. Nothing here
//! reads a rule file, a run or a gate; the caller matches the call and
//! gathers one [`Fact`] per requirement.
//!
//! A rule is a guard, not enforcement. A call it does not match passes. A
//! matched call passes only when every requirement is [`Fact::Met`]; an
//! [`Fact::Unknown`] requirement (the KB could not be read after the rule
//! matched) never passes, so a rule cannot be bypassed by breaking the
//! index. The Stop nudge fires at most once per session and never while
//! Claude Code is already continuing because of a stop hook.
//!
//! Each function is a few lines over a tiny domain; the tests below
//! enumerate it, and that enumeration is the proof (no Lean translation).

/// One requirement, as it stands now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fact {
    Met,
    Unmet,
    /// It could not be evaluated (the KB, a run or a gate was unreadable).
    Unknown,
}

/// What a matched rule does when a requirement is not met.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Deny,
    Ask,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::Deny => "deny",
            Action::Ask => "ask",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "deny" => Some(Action::Deny),
            "ask" => Some(Action::Ask),
            _ => None,
        }
    }
}

/// The outcome for one call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Pass,
    Deny,
    Ask,
}

/// Whether every fact is met (true for none).
pub fn all_met(facts: &[Fact]) -> bool {
    let mut i = 0;
    while i < facts.len() {
        if facts[i] != Fact::Met {
            return false;
        }
        i += 1;
    }
    true
}

/// `Pass` iff the rule did not match or every requirement is met;
/// otherwise the rule's action.
pub fn decide(matched: bool, action: Action, facts: &[Fact]) -> Decision {
    if !matched || all_met(facts) {
        return Decision::Pass;
    }
    match action {
        Action::Deny => Decision::Deny,
        Action::Ask => Decision::Ask,
    }
}

/// Several rules' decisions together: a denial wins over a question,
/// which wins over a pass.
pub fn combine(a: Decision, b: Decision) -> Decision {
    match (a, b) {
        (Decision::Deny, _) | (_, Decision::Deny) => Decision::Deny,
        (Decision::Ask, _) | (_, Decision::Ask) => Decision::Ask,
        _ => Decision::Pass,
    }
}

/// Whether a stop rule nudges now: not while Claude Code is already
/// continuing because of a stop hook, at most once per session, and only
/// when some requirement is not met.
pub fn nudge(stop_hook_active: bool, already_nudged: bool, facts: &[Fact]) -> bool {
    !stop_hook_active && !already_nudged && !all_met(facts)
}

#[cfg(test)]
mod tests {
    use super::{Action, Decision, Fact, all_met, combine, decide, nudge};
    use alloc::vec::Vec;

    const FACTS: [Fact; 3] = [Fact::Met, Fact::Unmet, Fact::Unknown];
    const DECISIONS: [Decision; 3] = [Decision::Pass, Decision::Deny, Decision::Ask];

    /// Every list of facts up to length three.
    fn lists() -> Vec<Vec<Fact>> {
        let mut out: Vec<Vec<Fact>> = alloc::vec![Vec::new()];
        let mut frontier = out.clone();
        for _ in 0..3 {
            let mut next = Vec::new();
            for list in &frontier {
                for fact in FACTS {
                    let mut longer = list.clone();
                    longer.push(fact);
                    next.push(longer);
                }
            }
            out.extend(next.iter().cloned());
            frontier = next;
        }
        out
    }

    #[test]
    fn decide_passes_iff_unmatched_or_every_fact_is_met() {
        for facts in lists() {
            let met = facts.iter().all(|f| *f == Fact::Met);
            for action in [Action::Deny, Action::Ask] {
                assert_eq!(decide(false, action, &facts), Decision::Pass);
                let want = match (met, action) {
                    (true, _) => Decision::Pass,
                    (false, Action::Deny) => Decision::Deny,
                    (false, Action::Ask) => Decision::Ask,
                };
                assert_eq!(decide(true, action, &facts), want, "{facts:?}");
                if facts.contains(&Fact::Unknown) {
                    assert_ne!(decide(true, action, &facts), Decision::Pass);
                }
            }
            assert_eq!(all_met(&facts), met);
        }
    }

    #[test]
    fn nudge_only_once_never_while_active_and_only_when_owed() {
        for facts in lists() {
            for active in [false, true] {
                for nudged in [false, true] {
                    let want = !active && !nudged && !facts.iter().all(|f| *f == Fact::Met);
                    assert_eq!(nudge(active, nudged, &facts), want, "{facts:?}");
                }
            }
        }
    }

    #[test]
    fn combine_is_the_strictest_and_order_free() {
        let rank = |d: Decision| match d {
            Decision::Pass => 0,
            Decision::Ask => 1,
            Decision::Deny => 2,
        };
        for a in DECISIONS {
            for b in DECISIONS {
                let c = combine(a, b);
                assert_eq!(c, combine(b, a));
                assert_eq!(rank(c), rank(a).max(rank(b)));
            }
        }
    }

    #[test]
    fn actions_round_trip() {
        for action in [Action::Deny, Action::Ask] {
            assert_eq!(Action::parse(action.name()), Some(action));
        }
        assert_eq!(Action::parse("warn"), None);
    }
}
