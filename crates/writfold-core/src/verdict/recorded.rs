//! Check receipts: which checker answers may be shared, how old a recorded
//! one is, and which of a token's statements (local or recorded) a reader
//! uses. Pure; the caller reads the files and the cache.
use super::{Checker, Said};

/// Whether a checker's answer may be written to a check receipt: only what
/// a checker said about the token itself. `Absent`, `Failed`, `Version`,
/// `Expired` and a checker's own `unverifiable` describe this machine or
/// this moment, and shared they would overwrite another machine's answer.
pub fn recordable(checker: Checker) -> bool {
    matches!(
        checker,
        Checker::Said(Said::Fresh | Said::Moved | Said::Changed | Said::Gone)
    )
}

/// How far a recorded clock may run ahead of the reader's, in seconds.
pub const CLOCK_SKEW: i64 = 3600;

/// The age in seconds of a recorded verdict checked at `checked_at`, read
/// at `now`: `None` (not usable) when it is dated more than [`CLOCK_SKEW`]
/// after `now`, else never below zero.
pub fn recorded_age(now: i64, checked_at: i64) -> Option<i64> {
    if checked_at > now.saturating_add(CLOCK_SKEW) {
        None
    } else {
        Some(now.saturating_sub(checked_at).max(0))
    }
}

/// One statement a checker made about a token: what it said, how many
/// seconds ago, and whether it was read from a check receipt (`recorded`)
/// rather than this machine's cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Statement {
    pub said: Said,
    pub age: i64,
    pub recorded: bool,
}

/// Which statement [`newest`] picked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pick {
    Local,
    /// The index into the recorded statements.
    Recorded(usize),
}

/// Worst first: `gone`, `changed`, `moved`, `fresh` (then `unverifiable`,
/// which no usable statement carries).
fn worst(said: Said) -> u8 {
    match said {
        Said::Gone => 0,
        Said::Changed => 1,
        Said::Moved => 2,
        Said::Fresh => 3,
        Said::Unverifiable => 4,
    }
}

/// The statement a reader uses: the newest (smallest age) of the local one
/// and the recorded ones. A local `unverifiable` is dropped (it never masks
/// a usable recorded verdict), as is a recorded `unverifiable`; every
/// recorded statement is dropped when the kind declares no `max_age`. A tie
/// goes to a receipt, between receipts to the worst verdict, then to the
/// first in `recorded` (the caller orders them by file name). The pick is
/// then aged with [`super::aged`].
pub fn newest(
    local: Option<Statement>,
    recorded: &[Statement],
    max_age: Option<i64>,
) -> Option<Pick> {
    let local = local
        .filter(|s| s.said != Said::Unverifiable)
        .map(|s| (Pick::Local, s));
    let shared = recorded
        .iter()
        .enumerate()
        .filter(|_| max_age.is_some())
        .filter(|(_, s)| s.said != Said::Unverifiable)
        .map(|(i, s)| (Pick::Recorded(i), *s));
    local
        .into_iter()
        .chain(shared)
        .min_by_key(|(pick, s)| {
            let order = match pick {
                Pick::Local => usize::MAX,
                Pick::Recorded(i) => *i,
            };
            (s.age, !s.recorded, worst(s.said), order)
        })
        .map(|(pick, _)| pick)
}

#[cfg(test)]
mod tests {
    use super::super::aged;
    use super::super::tests::{CHECKERS, SAID};
    use super::super::{Observed, Verdict, decide};
    use super::{Checker, Pick, Said, Statement, newest, recordable, recorded_age};

    #[test]
    fn only_a_checkers_statement_about_the_token_is_recordable() {
        for checker in CHECKERS {
            let want = matches!(
                checker,
                Checker::Said(Said::Fresh | Said::Moved | Said::Changed | Said::Gone)
            );
            assert_eq!(recordable(checker), want, "{checker:?}");
        }
    }

    #[test]
    fn a_recorded_date_more_than_an_hour_ahead_is_unusable() {
        assert_eq!(recorded_age(10_000, 10_000), Some(0));
        assert_eq!(recorded_age(10_000, 4_000), Some(6_000));
        assert_eq!(recorded_age(10_000, 13_600), Some(0));
        assert_eq!(recorded_age(10_000, 13_601), None);
        assert_eq!(recorded_age(i64::MAX, i64::MIN), Some(i64::MAX));
        assert_eq!(recorded_age(i64::MAX, i64::MAX), Some(0));
    }

    /// Every statement over a few verdicts and ages.
    fn statements(recorded: bool) -> alloc::vec::Vec<Statement> {
        let mut all = alloc::vec::Vec::new();
        for said in SAID {
            for age in [0, 5, 9] {
                all.push(Statement {
                    said,
                    age,
                    recorded,
                });
            }
        }
        all
    }

    /// What a reader concludes from the pick: the kernel's verdict after
    /// ageing, as `stale` would.
    fn read(
        local: Option<Statement>,
        recorded: &[Statement],
        max: Option<i64>,
    ) -> (Option<Statement>, Verdict) {
        let chosen = newest(local, recorded, max).and_then(|p| match p {
            Pick::Local => local,
            Pick::Recorded(i) => recorded.get(i).copied(),
        });
        let checker = chosen.map_or(Checker::Absent, |s| {
            aged(Checker::Said(s.said), Some(s.age), max)
        });
        (chosen, decide(Observed::Checked(checker), false))
    }

    #[test]
    fn only_the_newest_usable_fresh_statement_within_its_age_is_fresh() {
        let locals: alloc::vec::Vec<Option<Statement>> = core::iter::once(None)
            .chain(statements(false).into_iter().map(Some))
            .collect();
        let shared = statements(true);
        let mut lists: alloc::vec::Vec<alloc::vec::Vec<Statement>> = alloc::vec![alloc::vec![]];
        for a in &shared {
            lists.push(alloc::vec![*a]);
            for b in &shared {
                lists.push(alloc::vec![*a, *b]);
            }
        }
        for local in &locals {
            for list in &lists {
                for max in [None, Some(5)] {
                    let (chosen, verdict) = read(*local, list, max);
                    let usable: alloc::vec::Vec<Statement> = local
                        .iter()
                        .copied()
                        .chain(list.iter().copied().filter(|_| max.is_some()))
                        .filter(|s| s.said != Said::Unverifiable)
                        .collect();
                    let Some(chosen) = chosen else {
                        assert!(usable.is_empty(), "{local:?} {list:?} {max:?}");
                        assert_eq!(verdict, Verdict::Unverifiable);
                        continue;
                    };
                    assert!(usable.contains(&chosen), "{chosen:?}");
                    for other in &usable {
                        // Nothing usable is newer; at the same age a receipt
                        // wins, and between receipts the worse verdict.
                        assert!(other.age >= chosen.age, "{other:?} newer than {chosen:?}");
                        if other.age == chosen.age && other.recorded {
                            assert!(chosen.recorded, "{chosen:?} beat receipt {other:?}");
                            assert!(super::worst(chosen.said) <= super::worst(other.said));
                        }
                    }
                    let within = max.is_none_or(|m| chosen.age <= m);
                    assert_eq!(
                        verdict == Verdict::Fresh,
                        chosen.said == Said::Fresh && within,
                        "{local:?} {list:?} {max:?}"
                    );
                    if max.is_none() {
                        assert!(!chosen.recorded, "a receipt read without max_age");
                    }
                }
            }
        }
    }

    #[test]
    fn newest_wins_and_a_tie_goes_to_the_receipt_then_the_worst() {
        let s = |said, age, recorded| Statement {
            said,
            age,
            recorded,
        };
        let max = Some(100);
        let old_fresh = s(Said::Fresh, 9, true);
        let new_gone = s(Said::Gone, 1, true);
        assert_eq!(
            newest(None, &[old_fresh, new_gone], max),
            Some(Pick::Recorded(1))
        );
        let (old_gone, new_fresh) = (s(Said::Gone, 9, true), s(Said::Fresh, 1, true));
        assert_eq!(
            newest(None, &[old_gone, new_fresh], max),
            Some(Pick::Recorded(1))
        );
        let (a, b) = (s(Said::Fresh, 3, true), s(Said::Gone, 3, true));
        assert_eq!(newest(None, &[a, b], max), Some(Pick::Recorded(1)));
        assert_eq!(newest(None, &[b, a], max), Some(Pick::Recorded(0)));
        assert_eq!(newest(None, &[a, a], max), Some(Pick::Recorded(0)));
        let local_fresh = s(Said::Fresh, 1, false);
        assert_eq!(
            newest(Some(local_fresh), &[old_gone], max),
            Some(Pick::Local)
        );
        let local_tie = s(Said::Fresh, 9, false);
        assert_eq!(
            newest(Some(local_tie), &[old_gone], max),
            Some(Pick::Recorded(0))
        );
        let local_unsure = s(Said::Unverifiable, 0, false);
        assert_eq!(
            newest(Some(local_unsure), &[old_fresh], max),
            Some(Pick::Recorded(0))
        );
        assert_eq!(newest(Some(local_unsure), &[], max), None);
        assert_eq!(
            newest(Some(local_tie), &[new_gone], None),
            Some(Pick::Local)
        );
        assert_eq!(newest(None, &[new_gone], None), None);
    }
}
