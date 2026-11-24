use super::{Weight, agrees, blocking, verdict};
use crate::gate::Outcome;
use alloc::vec::Vec;

/// Every list of up to four matches: advice never decides, order does not
/// matter, no rules is never a pass, and the verdict always agrees with
/// its own blocking count.
#[test]
fn advice_never_decides_and_no_rules_is_unknown() {
    for len in 0..5u32 {
        for bits in 0..(1u32 << len) {
            let found: Vec<Weight> = (0..len)
                .map(|i| match bits >> i & 1 {
                    0 => Weight::Advice,
                    _ => Weight::Blocking,
                })
                .collect();
            let n = blocking(&found);
            assert_eq!(n, bits.count_ones() as usize);
            assert_eq!(verdict(0, &found), Outcome::Unknown);
            for rules in 1..3 {
                let v = verdict(rules, &found);
                assert_eq!(v == Outcome::Pass, n == 0);
                let only: Vec<Weight> = found
                    .iter()
                    .copied()
                    .filter(|w| *w == Weight::Blocking)
                    .collect();
                assert_eq!(verdict(rules, &only), v);
                let mut back = found.clone();
                back.reverse();
                assert_eq!(verdict(rules, &back), v);
                assert!(agrees(v, n));
            }
        }
    }
    assert!(agrees(Outcome::Pass, 0) && !agrees(Outcome::Pass, 1));
    assert!(agrees(Outcome::Fail, 1) && !agrees(Outcome::Fail, 0));
    assert!(agrees(Outcome::Unknown, 0) && agrees(Outcome::Unknown, 3));
}
