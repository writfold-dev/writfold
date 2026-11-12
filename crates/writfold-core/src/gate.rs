//! Gate obligations behind `writfold kb gate`: which paths a gate's scope
//! covers, what one evaluation's ending means, and the obligation state
//! derived from a gate's receipts. Nothing here reads a repository or a
//! receipt; the caller gathers the facts. State is never stored: it is this
//! function of the definition and the receipts, recomputed on every read.
//!
//! Proof candidate, not in the Charon `start_from` list: no theorem is
//! claimed. Property tests (here and in `writfold-kb`) check that a receipt
//! about another subject is never `Satisfied` and that only a clean exit
//! with status 0 is a pass.

/// How one evaluation ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ending {
    /// The command exited with this status.
    Exited(i32),
    /// It was killed by a signal it did not ask for (a crash).
    Signaled,
    /// It ran past its timeout and was stopped.
    TimedOut,
    /// It could not be started or supervised.
    Error,
}

/// The outcome a receipt records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pass,
    Fail,
    /// A timeout, crash or error: never evidence of success.
    Unknown,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Unknown => "unknown",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pass" => Some(Outcome::Pass),
            "fail" => Some(Outcome::Fail),
            "unknown" => Some(Outcome::Unknown),
            _ => None,
        }
    }
}

/// Only exit status 0 is a pass; any other exit status is a failure; every
/// other ending is unknown. `subject_moved` (an in-scope path changed while
/// the command ran) makes any ending unknown: the result is about no single
/// subject.
pub fn outcome(ending: Ending, subject_moved: bool) -> Outcome {
    if subject_moved {
        return Outcome::Unknown;
    }
    match ending {
        Ending::Exited(0) => Outcome::Pass,
        Ending::Exited(_) => Outcome::Fail,
        Ending::Signaled | Ending::TimedOut | Ending::Error => Outcome::Unknown,
    }
}

/// A gate's obligation on one subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// No receipt.
    Open,
    /// The newest receipt about the current subject passed.
    Satisfied,
    /// Receipts exist, but none is about the current definition and the
    /// current content of the paths in scope.
    Stale,
    /// The newest receipt about the current subject failed.
    Failed,
    /// The newest receipt about the current subject is unknown.
    Unknown,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Open => "open",
            State::Satisfied => "satisfied",
            State::Stale => "stale",
            State::Failed => "failed",
            State::Unknown => "unknown",
        }
    }
}

/// What one valid receipt says, compared with the present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub outcome: Outcome,
    /// The receipt's definition digest equals the current definition's.
    pub same_definition: bool,
    /// The receipt's scope digest equals the present content in scope.
    pub same_scope: bool,
}

/// The state from a gate's valid receipts, oldest first, and the index of
/// the receipt that decides it: the newest receipt about the current
/// subject, else the newest receipt (which is then stale).
pub fn state(receipts: &[Evidence]) -> (State, Option<usize>) {
    let mut index = receipts.len();
    while index > 0 {
        index -= 1;
        let evidence = receipts[index];
        if evidence.same_definition && evidence.same_scope {
            let state = match evidence.outcome {
                Outcome::Pass => State::Satisfied,
                Outcome::Fail => State::Failed,
                Outcome::Unknown => State::Unknown,
            };
            return (state, Some(index));
        }
    }
    match receipts.len() {
        0 => (State::Open, None),
        n => (State::Stale, Some(n - 1)),
    }
}

/// Whether `path` (repository-relative, `/`-separated) is in a scope: some
/// pattern matches it and no `!`-prefixed pattern does. Patterns are
/// anchored at the top of the repository: `*` and `?` match within one
/// segment, and a `**` segment matches any number of segments, none
/// included.
pub fn in_scope(patterns: &[&str], path: &str) -> bool {
    let mut included = false;
    for pattern in patterns {
        match pattern.strip_prefix('!') {
            Some(excluded) if glob(excluded, path) => return false,
            Some(_) => {}
            None => included = included || glob(pattern, path),
        }
    }
    included
}

/// One anchored pattern against one path.
pub fn glob(pattern: &str, path: &str) -> bool {
    let wanted: alloc::vec::Vec<&str> = pattern.split('/').collect();
    let parts: alloc::vec::Vec<&str> = path.split('/').collect();
    segments(&wanted, &parts)
}

fn segments(wanted: &[&str], parts: &[&str]) -> bool {
    match wanted.split_first() {
        None => parts.is_empty(),
        Some((&"**", rest)) => (0..=parts.len()).any(|skip| segments(rest, &parts[skip..])),
        Some((first, rest)) => match parts.split_first() {
            Some((part, others)) => segment(first, part) && segments(rest, others),
            None => false,
        },
    }
}

/// `*` and `?` within one segment, by characters, with one backtrack point.
fn segment(pattern: &str, text: &str) -> bool {
    let p: alloc::vec::Vec<char> = pattern.chars().collect();
    let t: alloc::vec::Vec<char> = text.chars().collect();
    let (mut i, mut j) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while j < t.len() {
        if i < p.len() && (p[i] == '?' || p[i] == t[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some((i, j));
            i += 1;
        } else if let Some((at, from)) = star {
            i = at + 1;
            j = from + 1;
            star = Some((at, from + 1));
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

#[cfg(test)]
mod tests {
    use super::{Ending, Evidence, Outcome, State, glob, in_scope, outcome, state};

    #[test]
    fn globs_are_anchored_segment_patterns() {
        assert!(glob("**", "a/b/c.rs") && glob("**", "x"));
        assert!(glob("crates/**", "crates/a/src/lib.rs") && glob("crates/**", "crates"));
        assert!(!glob("crates/**", "xcrates/a"));
        assert!(glob("**/*.md", "README.md") && glob("**/*.md", "docs/a/b.md"));
        assert!(!glob("*.md", "docs/a.md") && glob("*.md", "a.md"));
        assert!(glob("src/*.rs", "src/lib.rs") && !glob("src/*.rs", "src/a/lib.rs"));
        assert!(glob("a?c", "abc") && !glob("a?c", "ac"));
        assert!(glob("*a*b", "xxaxxb") && !glob("*a*b", "xxaxxbx"));
        assert!(glob("Cargo.toml", "Cargo.toml") && !glob("Cargo.toml", "x/Cargo.toml"));
        assert!(glob("a/**/b", "a/b") && glob("a/**/b", "a/x/y/b") && !glob("a/**/b", "a/x/c"));
    }

    #[test]
    fn exclusions_win_and_nothing_is_in_an_empty_scope() {
        let scope = ["**", "!docs/**", "!README.md"];
        assert!(in_scope(&scope, "crates/a.rs") && in_scope(&scope, "x/README.md"));
        assert!(!in_scope(&scope, "docs/x.md") && !in_scope(&scope, "README.md"));
        assert!(!in_scope(&[], "a") && !in_scope(&["!a"], "b"));
    }

    #[test]
    fn only_a_clean_zero_exit_passes() {
        for moved in [false, true] {
            for ending in [
                Ending::Exited(0),
                Ending::Exited(1),
                Ending::Exited(-1),
                Ending::Exited(255),
                Ending::Signaled,
                Ending::TimedOut,
                Ending::Error,
            ] {
                let got = outcome(ending, moved);
                let pass = ending == Ending::Exited(0) && !moved;
                assert_eq!(got == Outcome::Pass, pass, "{ending:?} moved={moved}");
                let fail = matches!(ending, Ending::Exited(n) if n != 0) && !moved;
                assert_eq!(got == Outcome::Fail, fail, "{ending:?} moved={moved}");
            }
        }
    }

    /// Every sequence of up to three receipts over every combination: the
    /// state is satisfied only when the newest receipt about the current
    /// subject passed, and stale exactly when receipts exist but none is
    /// about the current subject.
    #[test]
    fn a_receipt_about_another_subject_never_satisfies() {
        let outcomes = [Outcome::Pass, Outcome::Fail, Outcome::Unknown];
        let mut all = alloc::vec::Vec::new();
        for outcome in outcomes {
            for same_definition in [false, true] {
                for same_scope in [false, true] {
                    all.push(Evidence {
                        outcome,
                        same_definition,
                        same_scope,
                    });
                }
            }
        }
        assert_eq!(state(&[]), (State::Open, None));
        let n = all.len();
        for a in 0..n {
            for b in 0..=n {
                for c in 0..=n {
                    let mut seq = alloc::vec![all[a]];
                    seq.extend(all.get(b).copied());
                    seq.extend(all.get(c).copied());
                    let (got, at) = state(&seq);
                    let current = seq.iter().rposition(|e| e.same_definition && e.same_scope);
                    match current {
                        None => assert_eq!((got, at), (State::Stale, Some(seq.len() - 1))),
                        Some(i) => {
                            assert_eq!(at, Some(i));
                            let pass = seq[i].outcome == Outcome::Pass;
                            assert_eq!(got == State::Satisfied, pass);
                        }
                    }
                }
            }
        }
    }
}
