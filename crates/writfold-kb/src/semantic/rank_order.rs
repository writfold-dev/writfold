//! Supply bounded rank facts to the proved ordering kernel; no float decisions.
use super::{Candidate, LIMIT, Ranking};
use std::cmp::Ordering;
use writfold_core::semantic_order::{self as kernel, Lexical};

pub(super) fn rank(candidates: &mut [Candidate], lexical_len: usize, ranking: Ranking) {
    // The adapter caps each input at LIMIT before constructing these facts.
    let mut lexical = [Lexical {
        exact: false,
        rank: 0,
        semantic_rank: 0,
    }; LIMIT];
    for (index, candidate) in candidates[..lexical_len].iter().enumerate() {
        lexical[index] = Lexical {
            exact: candidate.row.matched_on == Some("exact"),
            rank: u32::try_from(index + 1).unwrap_or(u32::MAX),
            semantic_rank: candidate.semantic_rank,
        };
    }
    for candidate in &mut candidates[lexical_len..] {
        let slot = kernel::semantic_slot(
            &lexical[..lexical_len],
            candidate.semantic_rank,
            ranking == Ranking::Rrf,
        );
        candidate.key = kernel::semantic_key(slot, candidate.semantic_rank);
    }
    // Keys are unique: lexical ordinal, or the deduplicated semantic rank.
    // Unstable sort needs no allocation; kernel transitivity is proved.
    candidates.sort_unstable_by(|a, b| {
        if kernel::before(a.key, b.key) {
            Ordering::Less
        } else if kernel::before(b.key, a.key) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
}
