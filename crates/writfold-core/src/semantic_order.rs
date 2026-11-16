//! Stable lexical/semantic fusion. Inputs are already deduplicated record ranks.
//! Ranks are one-based and at most 50; overlap rank zero means absent.
//! The conservative RRF merge inserts after the last lexical score blocker or
//! exact match, so it never changes the supplied lexical order.

#[derive(Clone, Copy, Debug)]
pub struct Lexical {
    pub exact: bool,
    pub rank: u32,
    pub semantic_rank: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Key {
    pub slot: usize,
    pub semantic: bool,
    pub rank: u32,
}

/// Strict comparison of 1/(60+s) with 1/(60+l) + overlap/(60+t).
/// Ranks above 50 refuse semantic insertion; zero remains arithmetically safe
/// (ordinary input ranks are one-based). Products are at most 24,200.
pub fn semantic_wins(lexical_rank: u32, overlap_rank: u32, semantic_rank: u32) -> bool {
    if lexical_rank > 50 {
        return false;
    }
    if overlap_rank > 50 {
        return false;
    }
    if semantic_rank > 50 {
        return false;
    }
    if overlap_rank == 0 {
        semantic_rank < lexical_rank
    } else {
        let lexical_denominator = 60 + lexical_rank;
        let overlap_denominator = 60 + overlap_rank;
        let semantic_denominator = 60 + semantic_rank;
        lexical_denominator * overlap_denominator
            > (lexical_denominator + overlap_denominator) * semantic_denominator
    }
}

/// Exact matches and equal scores block semantic insertion.
pub fn blocks(lexical: Lexical, semantic_rank: u32) -> bool {
    lexical.exact || !semantic_wins(lexical.rank, lexical.semantic_rank, semantic_rank)
}

/// Number of lexical hits preceding this semantic-only hit.
/// For RRF this is the position after the last exact/score blocker; for
/// lexical-first it is the lexical length. No lexical candidate is reordered.
pub fn semantic_slot(lexical: &[Lexical], semantic_rank: u32, rrf: bool) -> usize {
    if !rrf {
        return lexical.len();
    }
    let mut slot = 0;
    let mut index = 0;
    while index < lexical.len() {
        let next = index + 1;
        if blocks(lexical[index], semantic_rank) {
            slot = next;
        }
        index = next;
    }
    slot
}

pub fn lexical_key(index: usize) -> Key {
    Key {
        slot: index,
        semantic: false,
        rank: 0,
    }
}

pub fn semantic_key(slot: usize, rank: u32) -> Key {
    Key {
        slot,
        semantic: true,
        rank,
    }
}

/// Strict lexicographic key order, not a mixed score/ordinal comparator.
/// A semantic candidate at slot i precedes lexical candidate i. Semantic
/// ranks come from the input's descending-score, ascending-record-ID order.
pub fn before(left: Key, right: Key) -> bool {
    if left.slot != right.slot {
        return left.slot < right.slot;
    }
    if left.semantic != right.semantic {
        return left.semantic;
    }
    left.rank < right.rank
}

#[cfg(test)]
#[path = "semantic_order_tests.rs"]
mod tests;
