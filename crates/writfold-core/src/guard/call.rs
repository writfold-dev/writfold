//! One call's decision from every matched pre-tool-use rule, and which
//! judged rules the guard log keeps.
use super::{Action, Decision, Mode, combine, effect, verdict};

/// One matched pre-tool-use rule, judged: its mode, its action, and
/// whether every requirement is met ([`all_met`] of its facts).
pub struct Ruled {
    pub mode: Mode,
    pub action: Action,
    pub met: bool,
}

/// What one judged rule does to the call: [`effect`] of its [`verdict`].
pub fn ruled(rule: &Ruled) -> Decision {
    effect(rule.mode, verdict(rule.met, rule.action))
}

/// The call's decision from every matched rule: [`combine`] of each
/// rule's effect, `Pass` when no rule matched.
pub fn decide_all(rules: &[Ruled]) -> Decision {
    let mut decided = Decision::Pass;
    let mut index = 0;
    while index < rules.len() {
        decided = combine(decided, ruled(&rules[index]));
        index += 1;
    }
    decided
}

/// Whether a judged rule goes to the guard log: every audit evaluation,
/// every decision that is not a pass, every pass that rested on a waiver.
pub fn logged(mode: Mode, decided: Decision, waived: bool) -> bool {
    matches!(mode, Mode::Audit) || !matches!(decided, Decision::Pass) || waived
}

#[cfg(test)]
mod tests {
    use super::{Ruled, decide_all, logged};
    use crate::guard::{Action, Decision, Mode};
    use alloc::vec::Vec;

    const DECISIONS: [Decision; 3] = [Decision::Pass, Decision::Deny, Decision::Ask];

    /// Every pair of rules: the call passes only when every enforcing
    /// rule's requirements are met, and is denied when one enforcing deny
    /// rule's are not; logging keeps audits, refusals and waivers.
    #[test]
    fn decide_all_combines_each_rules_effect() {
        let mut one = Vec::new();
        for mode in [Mode::Enforce, Mode::Audit] {
            for action in [Action::Deny, Action::Ask] {
                for met in [false, true] {
                    one.push((mode, action, met));
                }
            }
        }
        let rule = |(mode, action, met): (Mode, Action, bool)| Ruled { mode, action, met };
        assert_eq!(decide_all(&[]), Decision::Pass);
        for a in &one {
            for b in &one {
                let got = decide_all(&[rule(*a), rule(*b)]);
                let owes = |r: &(Mode, Action, bool)| r.0 == Mode::Enforce && !r.2;
                assert_eq!(got == Decision::Pass, !owes(a) && !owes(b));
                let denies = |r: &(Mode, Action, bool)| owes(r) && r.1 == Action::Deny;
                assert_eq!(got == Decision::Deny, denies(a) || denies(b));
            }
        }
        for mode in [Mode::Enforce, Mode::Audit] {
            for decided in DECISIONS {
                for waived in [false, true] {
                    let want = mode == Mode::Audit || decided != Decision::Pass || waived;
                    assert_eq!(logged(mode, decided, waived), want);
                }
            }
        }
    }
}
