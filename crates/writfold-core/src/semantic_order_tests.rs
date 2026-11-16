use super::{Lexical, before, blocks, lexical_key, semantic_key, semantic_slot, semantic_wins};

#[test]
fn real_rrf_insertion_and_lexical_first_differ() {
    let lexical = [
        Lexical {
            exact: true,
            rank: 1,
            semantic_rank: 0,
        },
        Lexical {
            exact: false,
            rank: 2,
            semantic_rank: 0,
        },
        Lexical {
            exact: false,
            rank: 3,
            semantic_rank: 0,
        },
    ];
    assert_eq!(semantic_slot(&lexical, 1, false), 3);
    assert_eq!(semantic_slot(&lexical, 1, true), 1);
    assert!(before(lexical_key(0), semantic_key(1, 1)));
    assert!(before(semantic_key(1, 1), lexical_key(1)));
    assert!(before(lexical_key(1), lexical_key(2)));
}

#[test]
fn later_exact_and_overlap_are_protected() {
    let lexical = [
        Lexical {
            exact: false,
            rank: 40,
            semantic_rank: 0,
        },
        Lexical {
            exact: true,
            rank: 41,
            semantic_rank: 0,
        },
        Lexical {
            exact: false,
            rank: 42,
            semantic_rank: 50,
        },
        Lexical {
            exact: false,
            rank: 43,
            semantic_rank: 0,
        },
    ];
    assert_eq!(semantic_slot(&lexical, 1, true), 3);
    assert!(blocks(lexical[1], 1));
    assert!(blocks(lexical[2], 1));
    assert_eq!(semantic_slot(&[], 1, true), 0);
}

#[test]
fn bounded_rrf_matches_rational_cross_products() {
    for lexical in 1..=50 {
        for overlap in 0..=50 {
            for semantic in 1..=50 {
                let numerator = if overlap == 0 {
                    1
                } else {
                    120 + lexical + overlap
                };
                let denominator = if overlap == 0 {
                    60 + lexical
                } else {
                    (60 + lexical) * (60 + overlap)
                };
                assert_eq!(
                    semantic_wins(lexical, overlap, semantic),
                    denominator > numerator * (60 + semantic)
                );
            }
        }
    }
}

#[test]
fn ties_are_lexical_first_and_semantic_rank_stable() {
    assert!(!semantic_wins(7, 0, 7));
    assert!(before(semantic_key(4, 2), semantic_key(4, 3)));
    assert!(!before(semantic_key(4, 3), semantic_key(4, 2)));
    assert!(!before(semantic_key(4, 2), semantic_key(4, 2)));
}

#[test]
fn key_order_is_transitive() {
    let keys = [
        lexical_key(0),
        lexical_key(1),
        lexical_key(2),
        semantic_key(0, 1),
        semantic_key(1, 1),
        semantic_key(1, 2),
        semantic_key(3, 1),
    ];
    for a in keys {
        for b in keys {
            for c in keys {
                if before(a, b) && before(b, c) {
                    assert!(before(a, c));
                }
            }
        }
    }
}

#[test]
fn invalid_large_ranks_refuse_without_overflow() {
    assert!(!semantic_wins(u32::MAX, 0, 1));
    assert!(!semantic_wins(1, u32::MAX, 1));
    assert!(!semantic_wins(1, 1, u32::MAX));
    let lexical = [Lexical {
        exact: false,
        rank: u32::MAX,
        semantic_rank: 0,
    }];
    assert_eq!(semantic_slot(&lexical, 1, true), 1);
}
