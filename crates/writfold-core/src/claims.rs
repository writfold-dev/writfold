//! Claims inside text (#189, RFC 2026-11-27-claims-in-text): whether every
//! prose block of a candidate is accounted for, and a floor on how many
//! citation tokens `claims.cited-fresh` needs. The caller splits the
//! candidate into blocks and classes each one; this module only decides.

use crate::gate::Outcome;

/// How a block's citation tokens stand together.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Standing {
    Fresh,
    Stale,
    Unverifiable,
}

/// One prose block of a candidate, as the caller classed it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Block {
    /// Holds at least one citation token.
    Cited(Standing),
    /// Carries exactly one status tag the mode takes.
    Tagged,
    /// A reviewer's row says it makes no claim (about these bytes).
    Excused,
    /// None of the above.
    Bare,
}

/// Whether a block fails the check by itself: bare, or a stale citation.
pub fn fails(block: Block) -> bool {
    match block {
        Block::Bare => true,
        Block::Cited(standing) => matches!(standing, Standing::Stale),
        Block::Tagged => false,
        Block::Excused => false,
    }
}

/// Whether a block cannot be told: an unverifiable citation.
pub fn unsure(block: Block) -> bool {
    match block {
        Block::Cited(standing) => matches!(standing, Standing::Unverifiable),
        Block::Bare => false,
        Block::Tagged => false,
        Block::Excused => false,
    }
}

/// `claims.accounted`: every block is cited and fresh, tagged, or excused.
/// No block is `Unknown` (nothing to account for is never a pass); a bare
/// block or a stale citation is `Fail`; otherwise an unverifiable citation
/// is `Unknown`; otherwise `Pass`.
pub fn accounted(blocks: &[Block]) -> Outcome {
    if blocks.is_empty() {
        return Outcome::Unknown;
    }
    let mut unknown = false;
    let mut index = 0;
    while index < blocks.len() {
        if fails(blocks[index]) {
            return Outcome::Fail;
        }
        unknown = unknown || unsure(blocks[index]);
        index += 1;
    }
    if unknown {
        Outcome::Unknown
    } else {
        Outcome::Pass
    }
}

/// `claims.cited-fresh` with a minimum: the shipped outcome, never a pass
/// below `min` tokens (`min = 0` changes nothing).
pub fn cited(tokens: usize, min: usize, shipped: Outcome) -> Outcome {
    if tokens < min {
        return Outcome::Fail;
    }
    shipped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_block_must_be_accounted_for() {
        use Block::*;
        assert_eq!(accounted(&[]), Outcome::Unknown);
        assert_eq!(
            accounted(&[Tagged, Excused, Cited(Standing::Fresh)]),
            Outcome::Pass
        );
        assert_eq!(accounted(&[Tagged, Bare]), Outcome::Fail);
        assert_eq!(
            accounted(&[Cited(Standing::Unverifiable), Cited(Standing::Stale)]),
            Outcome::Fail
        );
        assert_eq!(
            accounted(&[Cited(Standing::Unverifiable), Tagged]),
            Outcome::Unknown
        );
    }

    #[test]
    fn a_minimum_only_withholds() {
        assert_eq!(cited(0, 1, Outcome::Pass), Outcome::Fail);
        assert_eq!(cited(0, 0, Outcome::Pass), Outcome::Pass);
        assert_eq!(cited(3, 1, Outcome::Unknown), Outcome::Unknown);
    }
}
