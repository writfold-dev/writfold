//! When an investigation may be marked `concluded`: it says what it was
//! checked against and where its limits are, and no hypothesis is still
//! open. This is completion of the write-up, not a claim that its findings
//! are true. The caller reads the record; nothing here does IO.

/// A hypothesis's recorded outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hypothesis {
    Open,
    Supported,
    Refuted,
    /// Looked into, and left undecided on purpose.
    Unknown,
}

/// What a conclusion still lacks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Missing {
    pub checked_against: bool,
    pub scope_and_limits: bool,
    /// How many hypotheses are still open.
    pub open: usize,
}

impl Missing {
    /// Nothing is missing.
    pub fn none(&self) -> bool {
        !self.checked_against && !self.scope_and_limits && self.open == 0
    }
}

/// What stops the conclusion, given whether the two sections have text and
/// the hypotheses' outcomes.
pub fn conclude(
    checked_against: bool,
    scope_and_limits: bool,
    hypotheses: &[Hypothesis],
) -> Missing {
    let mut open = 0;
    let mut index = 0;
    while index < hypotheses.len() {
        if let Hypothesis::Open = hypotheses[index] {
            open += 1;
        }
        index += 1;
    }
    Missing {
        checked_against: !checked_against,
        scope_and_limits: !scope_and_limits,
        open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    const ALL: [Hypothesis; 4] = [
        Hypothesis::Open,
        Hypothesis::Supported,
        Hypothesis::Refuted,
        Hypothesis::Unknown,
    ];

    /// Every combination of the two sections and up to three hypotheses:
    /// concluding is allowed exactly when both sections have text and no
    /// hypothesis is open.
    #[test]
    fn exhaustive() {
        for checked in [false, true] {
            for scope in [false, true] {
                for n in 0..=3usize {
                    for code in 0..4usize.pow(n as u32) {
                        let mut hs = Vec::new();
                        let mut c = code;
                        for _ in 0..n {
                            hs.push(ALL[c % 4]);
                            c /= 4;
                        }
                        let open = hs.iter().filter(|h| **h == Hypothesis::Open).count();
                        let missing = conclude(checked, scope, &hs);
                        assert_eq!(missing.open, open);
                        assert_eq!(missing.none(), checked && scope && open == 0);
                    }
                }
            }
        }
    }
}
