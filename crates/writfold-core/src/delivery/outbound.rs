//! The outbound remainder (#187, RFC 2026-11-27-outbound-remainder): what
//! the delivery guard makes of every simple command and tool call of one
//! call, not only those its lists name. A call no list classifies is
//! unclassified, and the KB's `unclassified` policy says what it gets; a
//! program that runs its arguments (`bash -c`, `eval`, `ssh` with a
//! command) is opaque and gets the same. Without the policy keys every
//! member is routed, listed or local, and the decision is the shipped one.

use crate::guard::{Decision, combine};

/// What the delivery guard makes of one simple command or tool call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Class {
    /// Named by the KB's `local` list.
    Local,
    /// Under a route: the shipped guard decision.
    Routed(Decision),
    /// A listed command or tool: the lenient rule.
    Listed(Decision),
    /// Runs its arguments: `bash -c`, `eval`, `xargs`, `ssh` with a command.
    Opaque,
    /// None of the above.
    Unclassified,
}

/// What an unclassified call gets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unclassified {
    Pass,
    /// Passes, and the guard log keeps the command.
    Log,
    Ask,
    Deny,
}

impl Unclassified {
    pub fn name(self) -> &'static str {
        match self {
            Unclassified::Pass => "pass",
            Unclassified::Log => "log",
            Unclassified::Ask => "ask",
            Unclassified::Deny => "deny",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pass" => Some(Unclassified::Pass),
            "log" => Some(Unclassified::Log),
            "ask" => Some(Unclassified::Ask),
            "deny" => Some(Unclassified::Deny),
            _ => None,
        }
    }
}

/// What a call no list classifies gets under `policy`.
pub fn left_over(policy: Unclassified) -> Decision {
    match policy {
        Unclassified::Pass => Decision::Pass,
        Unclassified::Log => Decision::Pass,
        Unclassified::Ask => Decision::Ask,
        Unclassified::Deny => Decision::Deny,
    }
}

/// One member's contribution to the call's decision.
pub fn contribution(class: Class, policy: Unclassified) -> Decision {
    match class {
        Class::Local => Decision::Pass,
        Class::Routed(decided) => decided,
        Class::Listed(decided) => decided,
        Class::Opaque => left_over(policy),
        Class::Unclassified => left_over(policy),
    }
}

/// One call's decision: every member's contribution, combined so that a
/// denial wins over a question, which wins over a pass.
pub fn outbound(classes: &[Class], policy: Unclassified) -> Decision {
    let mut decided = Decision::Pass;
    let mut index = 0;
    while index < classes.len() {
        decided = combine(decided, contribution(classes[index], policy));
        index += 1;
    }
    decided
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICIES: [Unclassified; 4] = [
        Unclassified::Pass,
        Unclassified::Log,
        Unclassified::Ask,
        Unclassified::Deny,
    ];

    #[test]
    fn the_strictest_member_decides() {
        let mixed = [
            Class::Local,
            Class::Routed(Decision::Pass),
            Class::Unclassified,
        ];
        assert_eq!(outbound(&mixed, Unclassified::Pass), Decision::Pass);
        assert_eq!(outbound(&mixed, Unclassified::Ask), Decision::Ask);
        assert_eq!(outbound(&mixed, Unclassified::Deny), Decision::Deny);
        let refused = [Class::Routed(Decision::Deny), Class::Unclassified];
        assert_eq!(outbound(&refused, Unclassified::Ask), Decision::Deny);
    }

    #[test]
    fn local_never_refuses_and_opaque_is_unclassified() {
        for policy in POLICIES {
            assert_eq!(
                outbound(&[Class::Local, Class::Local], policy),
                Decision::Pass
            );
            assert_eq!(
                outbound(&[Class::Opaque], policy),
                outbound(&[Class::Unclassified], policy)
            );
            assert_eq!(Unclassified::parse(policy.name()), Some(policy));
        }
        assert_eq!(outbound(&[], Unclassified::Deny), Decision::Pass);
    }
}
