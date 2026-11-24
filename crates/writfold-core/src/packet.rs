//! The handover packet behind `writfold kb resume --packet`: which of a
//! run's rows fit a budget, and whether the packet may say it is complete.
//!
//! The caller lists the rows required items first, then optional ones in
//! the order they should survive, and gives each a cost (its estimated
//! tokens). The packet is the longest prefix of that list within the
//! budget ([`cut`]), so the selection keeps the input order and never
//! holds an optional row while a required one was dropped. A packet is
//! complete only when every required row is in it and nothing it was
//! built from was unreadable ([`complete`]).
//!
//! In the Charon `start_from` list; `proofs/Writfold/Packet.lean` proves
//! every function for every input (docs/writfold/aeneas.md).

/// How many leading rows fit in `budget`: the largest `k` such that the
/// costs of the first `k` rows add up to at most `budget`.
pub fn cut(costs: &[usize], budget: usize) -> usize {
    let mut used = 0;
    let mut index = 0;
    while index < costs.len() {
        let cost = costs[index];
        if cost > budget - used {
            return index;
        }
        used += cost;
        index += 1;
    }
    costs.len()
}

/// How many of the `required` leading rows a packet of `kept` rows left
/// out.
#[expect(
    clippy::implicit_saturating_sub,
    reason = "plain comparison and subtraction translate to Lean as written; the proof covers the bound"
)]
pub fn required_omitted(kept: usize, required: usize) -> usize {
    if kept >= required {
        0
    } else {
        required - kept
    }
}

/// A packet is complete exactly when it holds every required row and no
/// input it was built from was unreadable.
pub fn complete(kept: usize, required: usize, unreadable: usize) -> bool {
    kept >= required && unreadable == 0
}

#[cfg(test)]
mod tests {
    use super::{complete, cut, required_omitted};
    use alloc::vec::Vec;

    #[test]
    fn the_cut_is_the_longest_prefix_within_budget() {
        let lists: [&[usize]; 6] = [&[], &[0], &[3], &[1, 2, 3], &[5, 0, 1], &[2, 2, 2, 2]];
        for costs in lists {
            for budget in 0..10 {
                let k = cut(costs, budget);
                assert!(k <= costs.len());
                let sum = |n: usize| costs.iter().take(n).sum::<usize>();
                assert!(sum(k) <= budget);
                if k < costs.len() {
                    assert!(sum(k + 1) > budget);
                }
                if sum(costs.len()) <= budget {
                    assert_eq!(k, costs.len());
                }
            }
        }
        let big: Vec<usize> = Vec::from([usize::MAX, 1]);
        assert_eq!(cut(&big, usize::MAX), 1);
    }

    #[test]
    fn complete_only_with_every_required_row_and_nothing_unreadable() {
        for kept in 0..5 {
            for required in 0..5 {
                assert_eq!(
                    required_omitted(kept, required),
                    required - kept.min(required)
                );
                for unreadable in 0..2 {
                    assert_eq!(
                        complete(kept, required, unreadable),
                        required_omitted(kept, required) == 0 && unreadable == 0
                    );
                }
            }
        }
    }
}
