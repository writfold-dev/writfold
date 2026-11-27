//! Exhaustive tests of the verdict kernel.
use super::{
    Absence, Checker, Described, Handshake, Lifetime, Located, Observed, Said, Target, Verdict,
    aged, decide, exchange, handshake, lifetime, locate, missing, observed, other_blob,
};

#[test]
fn locate_tries_the_cited_lines_first() {
    let file = [1u8, 2, 3, 1, 2, 3];
    let at = |want: [u8; 2]| move |s: usize, h: usize| h == 2 && file[s..s + h] == want;
    assert_eq!(locate(6, 4, 5, 100, at([1, 2])), Located::Same);
    assert_eq!(locate(6, 2, 3, 100, at([1, 2])), Located::At(1, 2));
    assert_eq!(locate(6, 2, 3, 100, at([3, 3])), Located::Absent);
    assert_eq!(locate(6, 3, 2, 100, at([1, 2])), Located::Absent);
    assert_eq!(locate(6, 0, 1, 100, at([1, 2])), Located::Absent);
    assert_eq!(locate(6, 6, 7, 100, at([1, 2])), Located::At(1, 2));
    assert_eq!(locate(6, 1, 2, 5, at([1, 2])), Located::Unsearched);
}

const ABSENCES: [Absence; 8] = [
    Absence::Renamed,
    Absence::Followed(None),
    Absence::NoRename,
    Absence::RevAbsent,
    Absence::NoRev,
    Absence::Capped,
    Absence::Unfetched,
    Absence::Followed(Some(Located::Unfetched)),
];

/// #182: bytes this clone lacks decide nothing, for either value of known.
#[test]
fn unfetched_bytes_never_decide() {
    for known in [true, false] {
        assert_eq!(
            other_blob(known, Some(Located::Unfetched)),
            Verdict::Unverifiable
        );
    }
    assert_eq!(missing(Absence::Unfetched), Verdict::Unverifiable);
    assert_eq!(
        missing(Absence::Followed(Some(Located::Unfetched))),
        Verdict::Unverifiable
    );
}

/// #90: no stale verdict from missing history, and a negative verdict has
/// a witness (the theorems are in proofs/Writfold/Verdict.lean).
#[test]
fn absence_of_history_is_never_evidence() {
    for a in [Absence::RevAbsent, Absence::NoRev, Absence::Capped] {
        assert_eq!(
            observed(Observed::Code(Target::Missing(a))),
            Verdict::Unverifiable
        );
    }
    assert_eq!(
        observed(Observed::Code(Target::OtherBlob(false, None))),
        Verdict::Unverifiable
    );
    assert_eq!(
        observed(Observed::Code(Target::OtherBlob(
            false,
            Some(Located::Unsearched)
        ))),
        Verdict::Unverifiable
    );
    assert_eq!(
        observed(Observed::Code(Target::OtherBlob(
            false,
            Some(Located::Absent)
        ))),
        Verdict::Changed
    );
    assert_eq!(
        observed(Observed::Code(Target::Missing(Absence::NoRename))),
        Verdict::Gone
    );
    assert_eq!(
        observed(Observed::Code(Target::Missing(Absence::Followed(Some(
            Located::At(2, 3)
        ))))),
        Verdict::Moved
    );
}

#[test]
fn every_observation_has_one_verdict_and_marks_never_hide_a_change() {
    let located = [
        Located::Same,
        Located::At(3, 4),
        Located::Absent,
        Located::Unsearched,
    ];
    let mut facts = alloc::vec![
        Observed::Malformed,
        Observed::Unresolved,
        Observed::Code(Target::SameBlob),
        Observed::Code(Target::OtherBlob(true, None)),
        Observed::Code(Target::OtherBlob(false, None)),
        Observed::State(None),
        Observed::State(Some(None)),
    ];
    for flag in [true, false] {
        facts.push(Observed::Gate { at_target: flag });
        facts.push(Observed::State(Some(Some(flag))));
        facts.push(Observed::Timed(Some(flag)));
        facts.push(Observed::Offline { due: flag });
    }
    facts.push(Observed::Timed(None));
    facts.push(Observed::Immutable);
    facts.extend(CHECKERS.iter().map(|c| Observed::Checked(*c)));
    for known in [true, false] {
        facts.extend(
            located
                .iter()
                .map(|l| Observed::Code(Target::OtherBlob(known, Some(*l)))),
        );
    }
    facts.extend(ABSENCES.iter().map(|a| Observed::Code(Target::Missing(*a))));
    facts.extend(
        located
            .iter()
            .map(|l| Observed::Code(Target::Missing(Absence::Followed(Some(*l))))),
    );
    for fact in facts {
        let plain = decide(fact, false);
        let marked = decide(fact, true);
        assert_ne!(plain, Verdict::Marked);
        if matches!(plain, Verdict::Changed | Verdict::Gone | Verdict::Expired) {
            assert_eq!(marked, plain, "{fact:?}");
        } else {
            assert_eq!(marked, Verdict::Marked, "{fact:?}");
        }
        let fresh_code = matches!(
            fact,
            Observed::Code(Target::SameBlob)
                | Observed::Code(Target::OtherBlob(_, Some(Located::Same)))
        );
        if let Observed::Code(_) = fact {
            assert_eq!(plain == Verdict::Fresh, fresh_code, "{fact:?}");
        }
    }
}

#[test]
fn the_lifetime_is_immutable_then_the_token_then_the_default() {
    assert_eq!(decide(Observed::Immutable, false), Verdict::Fresh);
    for immutable in [true, false] {
        for token in [Some(1), None] {
            for default in [Some(2), None] {
                let chosen = lifetime(immutable, token, default);
                let expected = match (immutable, token, default) {
                    (true, _, _) => Lifetime::Immutable,
                    (false, Some(t), _) => Lifetime::Token(t),
                    (false, None, Some(d)) => Lifetime::Default(d),
                    (false, None, None) => Lifetime::Never,
                };
                assert_eq!(chosen, expected, "{immutable} {token:?} {default:?}");
                // A token's own ttl= always wins over a declared default.
                if !immutable && token.is_some() {
                    assert_eq!(chosen, Lifetime::Token(1));
                }
                // Without a declaration nothing changes: the token, else never.
                if !immutable && default.is_none() {
                    assert_eq!(chosen, token.map_or(Lifetime::Never, Lifetime::Token));
                }
            }
        }
    }
}

pub(super) const SAID: [Said; 5] = [
    Said::Fresh,
    Said::Moved,
    Said::Changed,
    Said::Gone,
    Said::Unverifiable,
];
pub(super) const CHECKERS: [Checker; 9] = [
    Checker::Absent,
    Checker::Expired,
    Checker::Failed,
    Checker::Version,
    Checker::Said(Said::Fresh),
    Checker::Said(Said::Moved),
    Checker::Said(Said::Changed),
    Checker::Said(Said::Gone),
    Checker::Said(Said::Unverifiable),
];

#[test]
fn only_a_checker_saying_fresh_is_fresh() {
    for checker in CHECKERS {
        let verdict = decide(Observed::Checked(checker), false);
        assert_eq!(
            verdict == Verdict::Fresh,
            checker == Checker::Said(Said::Fresh),
            "{checker:?}"
        );
        if !matches!(checker, Checker::Said(_)) {
            assert_eq!(verdict, Verdict::Unverifiable, "{checker:?}");
        }
    }
}

#[test]
fn only_a_fresh_reply_within_its_max_age_is_fresh() {
    let ages = [-5, 0, 6, 7, 8, i64::MAX].map(Some);
    for checker in CHECKERS {
        for age in ages.into_iter().chain([None]) {
            for max in [None, Some(0), Some(7)] {
                let got = aged(checker, age, max);
                let within = max.is_none_or(|m| age.is_some_and(|a| a <= m));
                assert_eq!(got, if within { checker } else { Checker::Expired });
                let fresh = decide(Observed::Checked(got), false) == Verdict::Fresh;
                assert_eq!(
                    fresh,
                    checker == Checker::Said(Said::Fresh) && within,
                    "{checker:?} {age:?} {max:?}"
                );
            }
        }
    }
}

#[test]
fn the_handshake_checks_protocol_then_kind_then_op() {
    assert_eq!(handshake(None), Handshake::Protocol);
    for bits in 0..8u8 {
        let d = Described {
            protocol: bits & 1 != 0,
            lists_kind: bits & 2 != 0,
            has_op: bits & 4 != 0,
        };
        let want = if !d.protocol {
            Handshake::Protocol
        } else if !d.lists_kind {
            Handshake::Kind
        } else if !d.has_op {
            Handshake::Op
        } else {
            Handshake::Agreed
        };
        assert_eq!(handshake(Some(d)), want, "{d:?}");
    }
}

#[test]
fn a_reply_counts_only_after_everything_else_held() {
    let shakes = [
        Handshake::Agreed,
        Handshake::Protocol,
        Handshake::Kind,
        Handshake::Op,
    ];
    let mut replies: alloc::vec::Vec<alloc::vec::Vec<Option<Said>>> =
        alloc::vec![alloc::vec![], alloc::vec![None], alloc::vec![None, None]];
    for said in SAID {
        replies.push(alloc::vec![Some(said)]);
        replies.push(alloc::vec![Some(said), Some(said)]);
        replies.push(alloc::vec![Some(said), None]);
    }
    for shake in shakes {
        for clean in [true, false] {
            for readable in [true, false] {
                for r in &replies {
                    let got = exchange(shake, clean, readable, r);
                    let want = match (shake, clean, readable, r.as_slice()) {
                        (Handshake::Agreed, true, false, _) => Checker::Version,
                        (Handshake::Agreed, true, true, [Some(s)]) => Checker::Said(*s),
                        _ => Checker::Failed,
                    };
                    assert_eq!(got, want, "{shake:?} {clean} {readable} {r:?}");
                    let fresh = decide(Observed::Checked(got), false) == Verdict::Fresh;
                    assert_eq!(
                        fresh,
                        shake == Handshake::Agreed
                            && clean
                            && readable
                            && r.as_slice() == [Some(Said::Fresh)],
                    );
                }
            }
        }
    }
}
