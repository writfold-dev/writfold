use super::{BlockFrom, Level, Ran, Tool, rules, settle, usable, weigh};
use crate::gate::{Ending, Outcome};
use crate::prose::{Weight, agrees, blocking};
use alloc::vec::Vec;

const LEVELS: [Level; 3] = [Level::Suggestion, Level::Warning, Level::Error];
const FROMS: [BlockFrom; 4] = [
    BlockFrom::Never,
    BlockFrom::Error,
    BlockFrom::Warning,
    BlockFrom::Suggestion,
];

fn rank(level: Level) -> u8 {
    match level {
        Level::Suggestion => 1,
        Level::Warning => 2,
        Level::Error => 3,
    }
}

fn floor(from: BlockFrom) -> u8 {
    match from {
        BlockFrom::Never => 4,
        BlockFrom::Error => 3,
        BlockFrom::Warning => 2,
        BlockFrom::Suggestion => 1,
    }
}

#[test]
fn a_level_blocks_from_its_threshold_up() {
    for from in FROMS {
        for level in LEVELS {
            let blocks = weigh(level, from) == Weight::Blocking;
            assert_eq!(blocks, rank(level) >= floor(from), "{level:?} {from:?}");
        }
    }
}

fn run(tool: Tool, ending: Ending, whole: bool, errors: usize) -> Ran {
    Ran {
        tool,
        ending,
        whole,
        errors,
        enabled: 3,
        canary: 1,
    }
}

#[test]
fn only_a_whole_pinned_run_whose_exit_agrees_is_usable() {
    let endings = [
        Ending::Exited(0),
        Ending::Exited(1),
        Ending::Exited(2),
        Ending::Exited(-1),
        Ending::Signaled,
        Ending::TimedOut,
        Ending::Error,
    ];
    for tool in [Tool::Absent, Tool::Other, Tool::Pinned] {
        for ending in endings {
            for whole in [false, true] {
                for errors in 0..3 {
                    let want = tool == Tool::Pinned
                        && whole
                        && match ending {
                            Ending::Exited(0) => errors == 0,
                            Ending::Exited(1) => errors > 0,
                            _ => false,
                        };
                    assert_eq!(usable(run(tool, ending, whole, errors)), want);
                }
            }
        }
    }
}

#[test]
fn settling_never_passes_without_a_usable_covered_run() {
    assert_eq!(rules(5, 0), 0);
    assert_eq!(rules(5, 2), 5);
    for len in 0..4u32 {
        for bits in 0..(1u32 << len) {
            let found: Vec<Weight> = (0..len)
                .map(|i| match bits >> i & 1 {
                    0 => Weight::Advice,
                    _ => Weight::Blocking,
                })
                .collect();
            let n = blocking(&found);
            let bad = run(Tool::Other, Ending::Exited(0), true, 0);
            assert_eq!(settle(bad, &found), Outcome::Unknown);
            for (enabled, canary) in [(0, 1), (3, 0), (3, 2)] {
                let mut ran = run(Tool::Pinned, Ending::Exited(0), true, 0);
                ran.enabled = enabled;
                ran.canary = canary;
                let o = settle(ran, &found);
                let covered = enabled > 0 && canary > 0;
                assert_eq!(o == Outcome::Pass, covered && n == 0);
                assert_eq!(o == Outcome::Fail, covered && n > 0);
                assert!(agrees(o, n));
            }
        }
    }
}

#[test]
fn a_pass_under_a_stricter_threshold_passes_under_a_laxer_one() {
    let ran = run(Tool::Pinned, Ending::Exited(1), true, 1);
    let docs: [&[Level]; 4] = [
        &[],
        &[Level::Suggestion],
        &[Level::Warning, Level::Suggestion],
        &[Level::Error],
    ];
    for doc in docs {
        for strict in FROMS {
            for lax in FROMS {
                if floor(lax) < floor(strict) {
                    continue;
                }
                let weighed = |from| doc.iter().map(|l| weigh(*l, from)).collect::<Vec<_>>();
                if settle(ran, &weighed(strict)) == Outcome::Pass {
                    assert_eq!(settle(ran, &weighed(lax)), Outcome::Pass);
                }
            }
        }
    }
}
