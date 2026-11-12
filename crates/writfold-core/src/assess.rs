//! The Weft assessed gate's decision behind `writfold kb gate run` on a
//! gate of class `assessed`. Weft decides each check's result from the
//! assumptions it was given; Writfold decides whether those assumptions
//! are grounded. This kernel joins the two: a check is met only when Weft
//! entailed it and every assumption Weft used for it (its premises) is
//! grounded. A check resting on an ungrounded assumption keeps Weft's
//! result and is `hypothetical`; it never meets the gate.
//!
//! The words are Weft's: results `entailed`, `refuted`, `undetermined`,
//! `inconsistent-premises` (and `unknown`, `not-evaluated` for work Weft
//! could not finish); bases `grounded` and `hypothetical`. Nothing here
//! reads a report or a citation; the caller gathers the facts.
//!
//! Proof candidate, not in the Charon `start_from` list: no theorem is
//! claimed. The property tests below check, over every combination, that
//! a gate passes exactly when every required check is entailed with every
//! premise grounded.
use crate::gate::Outcome;

/// What Writfold found for one cited piece of evidence, or for one
/// assumption once all its citations are read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Grounding {
    /// Fresh (a KB citation) or satisfied (a receipt).
    Grounded,
    /// Met by an attestation: grounded, and named as such.
    Attested,
    /// Stale, failed, missing, unreadable, or cited nothing.
    Ungrounded,
}

impl Grounding {
    pub fn name(self) -> &'static str {
        match self {
            Grounding::Grounded => "grounded",
            Grounding::Attested => "attested",
            Grounding::Ungrounded => "ungrounded",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "grounded" => Some(Grounding::Grounded),
            "attested" => Some(Grounding::Attested),
            "ungrounded" => Some(Grounding::Ungrounded),
            _ => None,
        }
    }
}

/// Weft's result for one check, as its report names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    Entailed,
    Refuted,
    Undetermined,
    Inconsistent,
    /// Weft could not complete the check (a budget, an invalid input).
    Unknown,
    NotEvaluated,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Entailed => "entailed",
            Verdict::Refuted => "refuted",
            Verdict::Undetermined => "undetermined",
            Verdict::Inconsistent => "inconsistent-premises",
            Verdict::Unknown => "unknown",
            Verdict::NotEvaluated => "not-evaluated",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "entailed" => Some(Verdict::Entailed),
            "refuted" => Some(Verdict::Refuted),
            "undetermined" => Some(Verdict::Undetermined),
            "inconsistent-premises" => Some(Verdict::Inconsistent),
            "unknown" => Some(Verdict::Unknown),
            "not-evaluated" => Some(Verdict::NotEvaluated),
            _ => None,
        }
    }
}

/// On what a check's result rests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Basis {
    /// Every premise is grounded (or attested).
    Grounded,
    /// Some premise is not.
    Hypothetical,
}

impl Basis {
    pub fn name(self) -> &'static str {
        match self {
            Basis::Grounded => "grounded",
            Basis::Hypothetical => "hypothetical",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "grounded" => Some(Basis::Grounded),
            "hypothetical" => Some(Basis::Hypothetical),
            _ => None,
        }
    }
}

/// An assumption's grounding from its citations' groundings: ungrounded
/// when it cites nothing or any citation is ungrounded; attested when any
/// citation is only attested; grounded otherwise.
pub fn assumption(citations: &[Grounding]) -> Grounding {
    if citations.is_empty() {
        return Grounding::Ungrounded;
    }
    let mut attested = false;
    let mut index = 0;
    while index < citations.len() {
        match citations[index] {
            Grounding::Ungrounded => return Grounding::Ungrounded,
            Grounding::Attested => attested = true,
            Grounding::Grounded => {}
        }
        index += 1;
    }
    match attested {
        true => Grounding::Attested,
        false => Grounding::Grounded,
    }
}

/// A check's basis from the groundings of the assumptions Weft used for
/// it: grounded only when none is ungrounded. A check with no premises
/// (a tautology) rests on nothing and is grounded.
pub fn basis(premises: &[Grounding]) -> Basis {
    let mut index = 0;
    while index < premises.len() {
        if premises[index] == Grounding::Ungrounded {
            return Basis::Hypothetical;
        }
        index += 1;
    }
    Basis::Grounded
}

/// Whether one check meets the gate: entailed, on a grounded basis.
pub fn met(verdict: Verdict, basis: Basis) -> bool {
    verdict == Verdict::Entailed && basis == Basis::Grounded
}

/// What the assessment found for one required check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Required {
    /// The report has no check of that name.
    Missing,
    Found(Verdict, Basis),
}

/// The receipt's outcome. `complete` is false when Weft did not run to a
/// complete report (no binary, a timeout, a crash, an incomplete or
/// unreadable report, citations it could not place): unknown, never a
/// pass. Otherwise a check Weft could not evaluate is unknown; every
/// required check met is a pass; anything else (missing, refuted,
/// undetermined, inconsistent, or hypothetical) is a failure.
pub fn outcome(complete: bool, required: &[Required]) -> Outcome {
    if !complete || required.is_empty() {
        return Outcome::Unknown;
    }
    let mut all = true;
    let mut index = 0;
    while index < required.len() {
        match required[index] {
            Required::Missing => all = false,
            Required::Found(Verdict::Unknown | Verdict::NotEvaluated, _) => {
                return Outcome::Unknown;
            }
            Required::Found(verdict, basis) => {
                if !met(verdict, basis) {
                    all = false;
                }
            }
        }
        index += 1;
    }
    match all {
        true => Outcome::Pass,
        false => Outcome::Fail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    const GROUNDINGS: [Grounding; 3] = [
        Grounding::Grounded,
        Grounding::Attested,
        Grounding::Ungrounded,
    ];
    const VERDICTS: [Verdict; 6] = [
        Verdict::Entailed,
        Verdict::Refuted,
        Verdict::Undetermined,
        Verdict::Inconsistent,
        Verdict::Unknown,
        Verdict::NotEvaluated,
    ];

    /// Every list of up to three groundings.
    fn lists() -> Vec<Vec<Grounding>> {
        let mut out = alloc::vec![Vec::new()];
        for _ in 0..3 {
            let mut next = Vec::new();
            for list in &out {
                for g in GROUNDINGS {
                    let mut longer = list.clone();
                    longer.push(g);
                    next.push(longer);
                }
            }
            out.extend(next);
        }
        out
    }

    #[test]
    fn names_round_trip() {
        for g in GROUNDINGS {
            assert_eq!(Grounding::parse(g.name()), Some(g));
        }
        for v in VERDICTS {
            assert_eq!(Verdict::parse(v.name()), Some(v));
        }
        for b in [Basis::Grounded, Basis::Hypothetical] {
            assert_eq!(Basis::parse(b.name()), Some(b));
        }
        assert_eq!(Verdict::parse("inconsistent"), None);
    }

    #[test]
    fn an_assumption_is_as_weak_as_its_weakest_citation() {
        assert_eq!(assumption(&[]), Grounding::Ungrounded);
        for list in lists().into_iter().filter(|l| !l.is_empty()) {
            let expected = if list.contains(&Grounding::Ungrounded) {
                Grounding::Ungrounded
            } else if list.contains(&Grounding::Attested) {
                Grounding::Attested
            } else {
                Grounding::Grounded
            };
            assert_eq!(assumption(&list), expected, "{list:?}");
        }
    }

    /// The grounding check: a check is met exactly when Weft entailed it
    /// and no premise is ungrounded. Removing the premise test from
    /// `basis` fails this property.
    #[test]
    fn a_check_is_met_only_when_entailed_from_grounded_premises() {
        for premises in lists() {
            for verdict in VERDICTS {
                let grounded = !premises.contains(&Grounding::Ungrounded);
                let b = basis(&premises);
                assert_eq!(b == Basis::Grounded, grounded, "{premises:?}");
                assert_eq!(
                    met(verdict, b),
                    verdict == Verdict::Entailed && grounded,
                    "{verdict:?} {premises:?}"
                );
            }
        }
    }

    #[test]
    fn the_gate_passes_exactly_when_every_required_check_is_met() {
        let mut each = alloc::vec![Required::Missing];
        for v in VERDICTS {
            each.push(Required::Found(v, Basis::Grounded));
            each.push(Required::Found(v, Basis::Hypothetical));
        }
        for a in &each {
            for b in &each {
                let required = [*a, *b];
                let unfinished = required.iter().any(|r| {
                    matches!(
                        r,
                        Required::Found(Verdict::Unknown | Verdict::NotEvaluated, _)
                    )
                });
                let all = required
                    .iter()
                    .all(|r| matches!(r, Required::Found(v, b) if met(*v, *b)));
                let expected = if unfinished {
                    Outcome::Unknown
                } else if all {
                    Outcome::Pass
                } else {
                    Outcome::Fail
                };
                assert_eq!(outcome(true, &required), expected, "{required:?}");
                assert_eq!(outcome(false, &required), Outcome::Unknown);
            }
        }
        assert_eq!(outcome(true, &[]), Outcome::Unknown);
        // Hypothetical entailment is never a pass.
        let hypothetical = [Required::Found(Verdict::Entailed, Basis::Hypothetical)];
        assert_eq!(outcome(true, &hypothetical), Outcome::Fail);
    }
}
