//! The guard site's decision for one event (#184, law S in
//! `proofs/Calculus/Site.lean`, RFC 2026-11-27-guard-check-and-posture).
//!
//! A site either judged the event (no rules, no match, or a decision), or
//! could not (`Undecided`: no KB, input that is not an event, a rule file
//! that does not parse, a required rule missing, the deadline). Only an
//! undecided event reads the posture; `Open` is today's behaviour, a
//! declared exemption from "unknown never admits".

use crate::guard::Decision;

/// How far the site got with one event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Basis {
    NoRules,
    NoMatch,
    Judged(Decision),
    Undecided,
}

/// What an undecided event gets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Posture {
    Open,
    Ask,
    Deny,
}

impl Posture {
    pub fn name(self) -> &'static str {
        match self {
            Posture::Open => "open",
            Posture::Ask => "ask",
            Posture::Deny => "deny",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "open" => Some(Posture::Open),
            "ask" => Some(Posture::Ask),
            "deny" => Some(Posture::Deny),
            _ => None,
        }
    }
}

/// The site's decision: a judged event keeps its decision under every
/// posture; an undecided one gets the posture's.
pub fn decide(basis: Basis, posture: Posture) -> Decision {
    match basis {
        Basis::NoRules => Decision::Pass,
        Basis::NoMatch => Decision::Pass,
        Basis::Judged(decided) => decided,
        Basis::Undecided => match posture {
            Posture::Open => Decision::Pass,
            Posture::Ask => Decision::Ask,
            Posture::Deny => Decision::Deny,
        },
    }
}

/// The stricter of two postures (the registration's and the KB's).
pub fn stricter(a: Posture, b: Posture) -> Posture {
    if matches!(a, Posture::Deny) || matches!(b, Posture::Deny) {
        return Posture::Deny;
    }
    if matches!(a, Posture::Ask) || matches!(b, Posture::Ask) {
        return Posture::Ask;
    }
    Posture::Open
}

/// Whether a blocking stop rule blocks again: only while its requirement
/// is unmet and it has blocked fewer than `limit` times this turn.
pub fn block_again(unmet: bool, blocked: usize, limit: usize) -> bool {
    unmet && blocked < limit
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASES: [Basis; 6] = [
        Basis::NoRules,
        Basis::NoMatch,
        Basis::Judged(Decision::Pass),
        Basis::Judged(Decision::Ask),
        Basis::Judged(Decision::Deny),
        Basis::Undecided,
    ];
    const POSTURES: [Posture; 3] = [Posture::Open, Posture::Ask, Posture::Deny];

    #[test]
    fn open_is_today_and_judged_is_unchanged() {
        for basis in BASES {
            let open = decide(basis, Posture::Open);
            for posture in POSTURES {
                if let Basis::Judged(d) = basis {
                    assert_eq!(decide(basis, posture), d);
                } else if basis == Basis::Undecided {
                    assert_eq!(decide(basis, Posture::Open), Decision::Pass);
                } else {
                    assert_eq!(decide(basis, posture), open);
                }
            }
        }
        assert_eq!(decide(Basis::Undecided, Posture::Ask), Decision::Ask);
        assert_eq!(decide(Basis::Undecided, Posture::Deny), Decision::Deny);
    }

    fn rank(p: Posture) -> u8 {
        match p {
            Posture::Open => 0,
            Posture::Ask => 1,
            Posture::Deny => 2,
        }
    }

    #[test]
    fn stricter_is_the_maximum() {
        for a in POSTURES {
            assert_eq!(stricter(Posture::Open, a), a);
            for b in POSTURES {
                assert_eq!(stricter(a, b), stricter(b, a));
                assert_eq!(stricter(a, b), if rank(a) >= rank(b) { a } else { b });
            }
        }
        for p in POSTURES {
            assert_eq!(Posture::parse(p.name()), Some(p));
        }
    }

    #[test]
    fn block_again_is_bounded() {
        assert!(block_again(true, 0, 1));
        assert!(!block_again(true, 1, 1));
        assert!(!block_again(false, 0, 3));
        assert!(!block_again(true, 0, 0));
    }
}
