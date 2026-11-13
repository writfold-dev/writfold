//! The citation verdict behind `writfold kb stale`: from what a check
//! observed (blob equality at the target commit, path presence, where the
//! cited span now sits, expiry, a recorded state) to exactly one verdict.
//! Nothing here reads a repository; the caller gathers the facts.
//!
//! Proof candidate, not yet in the Charon `start_from` list: no theorem is
//! claimed about it. Its property tests (in `writfold-kb`) check that a
//! fresh verdict means the cited lines are unchanged after normalisation
//! and that a moved verdict names a range holding the cited span digest.

/// Where the cited span sits in the file at the target commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Located {
    /// The span digest matches at the cited lines.
    Same,
    /// It matches first at these one-based lines.
    At(u32, u32),
    /// No window of the cited height matches.
    Absent,
    /// The file is longer than the search limit.
    Unsearched,
}

/// Slide a window of the cited height over a file of `lines` lines:
/// the cited position first, then every position from the top.
/// `matches(start, height)` says whether the zero-based window
/// `start..start + height` carries the cited span digest.
pub fn locate(
    lines: usize,
    first: u32,
    last: u32,
    limit: usize,
    matches: impl Fn(usize, usize) -> bool,
) -> Located {
    if lines > limit {
        return Located::Unsearched;
    }
    if first == 0 || last < first {
        return Located::Absent;
    }
    let (Ok(first), Ok(last)) = (usize::try_from(first), usize::try_from(last)) else {
        return Located::Absent;
    };
    let height = last - first + 1;
    if height > lines {
        return Located::Absent;
    }
    let cited = first - 1;
    if cited + height <= lines && matches(cited, height) {
        return Located::Same;
    }
    let mut start = 0;
    while start + height <= lines {
        if matches(start, height) {
            let to = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
            return Located::At(to(start + 1), to(start + height));
        }
        start += 1;
    }
    Located::Absent
}

/// What a code citation's path holds at the target commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// The cited blob.
    SameBlob,
    /// Another blob: where the span sits in it, or `None` when the citation
    /// carries no span digest to search for.
    OtherBlob(Option<Located>),
    /// Nothing; `renamed` when the cited blob sits at another path.
    Missing { renamed: bool },
}

/// Everything a check can observe about one citation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observed {
    /// The token does not parse.
    Malformed,
    /// The repository, path or a required field could not be resolved.
    Unresolved,
    /// A code citation checked at the target commit.
    Code(Target),
    /// A gate citation: whether its commit is the target commit.
    Gate { at_target: bool },
    /// A record or proposal citation: `None` when the record is gone,
    /// `Some(None)` when it has no status, else whether the cited state
    /// equals the current one (an absent cited state counts as equal).
    State(Option<Option<bool>>),
    /// Any other kind: `seen + ttl` passed, or `None` when unreadable.
    Timed(Option<bool>),
    /// Offline: only whether `seen + ttl` passed.
    Offline { due: bool },
    /// A kind declared immutable: what it cites never changes.
    Immutable,
}

/// Exactly one verdict per citation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    Fresh,
    /// Same content at other lines.
    Moved,
    /// Same blob at another path (an exact rename).
    Renamed,
    Changed,
    Gone,
    Expired,
    Unverifiable,
    /// An authored `stale=` mark, passed through.
    Marked,
    /// Past `seen + ttl`, reported offline.
    Due,
}

/// The verdict before an authored mark is considered.
pub fn observed(fact: Observed) -> Verdict {
    match fact {
        Observed::Malformed | Observed::Unresolved => Verdict::Unverifiable,
        Observed::Code(Target::SameBlob) => Verdict::Fresh,
        Observed::Code(Target::OtherBlob(None)) => Verdict::Changed,
        Observed::Code(Target::OtherBlob(Some(located))) => match located {
            Located::Same => Verdict::Fresh,
            Located::At(_, _) => Verdict::Moved,
            Located::Absent | Located::Unsearched => Verdict::Changed,
        },
        Observed::Code(Target::Missing { renamed: true }) => Verdict::Renamed,
        Observed::Code(Target::Missing { renamed: false }) => Verdict::Gone,
        Observed::Gate { at_target: true } => Verdict::Fresh,
        Observed::Gate { at_target: false } => Verdict::Changed,
        Observed::State(None) => Verdict::Gone,
        Observed::State(Some(None)) => Verdict::Unverifiable,
        Observed::State(Some(Some(true))) => Verdict::Fresh,
        Observed::State(Some(Some(false))) => Verdict::Changed,
        Observed::Timed(Some(true)) => Verdict::Expired,
        Observed::Timed(Some(false)) => Verdict::Fresh,
        Observed::Timed(None) => Verdict::Unverifiable,
        Observed::Offline { due: true } => Verdict::Due,
        Observed::Offline { due: false } => Verdict::Fresh,
        Observed::Immutable => Verdict::Fresh,
    }
}

/// The lifetime a time-based citation is judged by.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lifetime<T> {
    /// Its kind is declared immutable: fresh at any age.
    Immutable,
    /// The token's own `ttl=`.
    Token(T),
    /// No `ttl=` on the token: its kind's declared default.
    Default(T),
    /// Neither: it never expires.
    Never,
}

/// Choose the lifetime: an immutable kind first, then the token's own
/// `ttl=`, then the kind's default, else never.
pub fn lifetime<T>(immutable: bool, token: Option<T>, default: Option<T>) -> Lifetime<T> {
    match (immutable, token, default) {
        (true, _, _) => Lifetime::Immutable,
        (false, Some(ttl), _) => Lifetime::Token(ttl),
        (false, None, Some(ttl)) => Lifetime::Default(ttl),
        (false, None, None) => Lifetime::Never,
    }
}

/// The final verdict: an authored mark wins over every verdict except
/// changed, gone and expired, which say more.
pub fn decide(fact: Observed, marked: bool) -> Verdict {
    with_mark(observed(fact), marked)
}

/// Apply an authored mark to a verdict already reached.
pub fn with_mark(verdict: Verdict, marked: bool) -> Verdict {
    match verdict {
        Verdict::Changed | Verdict::Gone | Verdict::Expired => verdict,
        _ if marked => Verdict::Marked,
        _ => verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::{Lifetime, Located, Observed, Target, Verdict, decide, lifetime, locate};

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
            Observed::Code(Target::OtherBlob(None)),
            Observed::Code(Target::Missing { renamed: true }),
            Observed::Code(Target::Missing { renamed: false }),
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
        facts.extend(
            located
                .iter()
                .map(|l| Observed::Code(Target::OtherBlob(Some(*l)))),
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
                    | Observed::Code(Target::OtherBlob(Some(Located::Same)))
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
}
