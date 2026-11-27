//! The commit a code or gate citation is judged against (#179, RFC
//! 2026-11-27-citation-target): a decision, not a convention.
//!
//! - [`choose`] picks the target from what the local map pins and how each
//!   of the clone's remotes relates to the cited repository: a resolving
//!   pin decides, a missing one is ambiguous; without remotes the clone's
//!   `HEAD` is judged, as before; otherwise only the remotes at the
//!   strongest naming level present are candidates, a remote that names
//!   another repository never is, and candidates without a default branch
//!   or at different commits are ambiguous rather than guessed between.
//!
//! What a chosen target can support is `crate::referent` (#196), which
//! knows nothing of Git: this module is the Git provider's resolver.
//!
//! Reading remotes and their URLs, normalising URLs, finding a remote's
//! tip and the ancestry walk are the caller's (tested). In the Charon
//! `start_from` list; `proofs/Writfold/Target.lean` proves [`choose`]
//! for every input (docs/writfold/aeneas.md).

/// How a remote's URL relates to the cited repository.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Names {
    /// Equal to the citation's `url=` after normalisation.
    Url,
    /// Its path ends in a qualified `repo=` (`org/service`).
    Path,
    /// Only its last segment equals an unqualified `repo=`.
    Base,
    /// The clone was named by the map or a path, `repo=` is unqualified,
    /// and this is its only remote.
    Sole,
    /// It names another repository.
    Other,
    /// No URL, or one that does not parse.
    Unread,
}

/// The remote's default branch as this clone knows it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tip {
    /// `At(k)`: `k` indexes the distinct commits the caller saw.
    At(u32),
    NoHead,
}

/// One remote of the clone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Remote {
    pub names: Names,
    pub tip: Tip,
}

/// What the local map says about the clone's ref.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pinned {
    No,
    Resolves,
    Missing,
}

/// Why no target was chosen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Why {
    NoRemoteNames,
    Disagree,
    NoDefaultBranch,
    PinMissing,
}

/// The chosen target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    /// The map's ref.
    Pinned,
    /// This remote's default branch.
    Remote(usize),
    /// The clone's own `HEAD` (a clone without remotes).
    Head,
    Ambiguous(Why),
}

/// A naming level: 4 for `Url` down to 1 for `Sole`; 0 is never a
/// candidate.
pub fn rank(names: Names) -> u8 {
    match names {
        Names::Url => 4,
        Names::Path => 3,
        Names::Base => 2,
        Names::Sole => 1,
        Names::Other => 0,
        Names::Unread => 0,
    }
}

/// The strongest naming level among the remotes; 0 when none names the
/// repository.
pub fn best(remotes: &[Remote]) -> u8 {
    let mut top = 0;
    let mut i = 0;
    while i < remotes.len() {
        let level = rank(remotes[i].names);
        if level > top {
            top = level;
        }
        i += 1;
    }
    top
}

/// The commit a tip is at, if any.
pub fn tip_commit(tip: Tip) -> Option<u32> {
    match tip {
        Tip::At(k) => Some(k),
        Tip::NoHead => None,
    }
}

/// Whether a tip is at commit `k`.
pub fn tip_is(tip: Tip, k: u32) -> bool {
    match tip {
        Tip::At(j) => j == k,
        Tip::NoHead => false,
    }
}

/// Whether some remote at `level` has no default branch.
pub fn headless_at(remotes: &[Remote], level: u8) -> bool {
    let mut i = 0;
    while i < remotes.len() {
        let r = remotes[i];
        if rank(r.names) == level && tip_commit(r.tip).is_none() {
            return true;
        }
        i += 1;
    }
    false
}

/// The first remote at `level` and its tip; `None` when there is none or
/// it has no default branch.
pub fn first_at(remotes: &[Remote], level: u8) -> Option<(usize, u32)> {
    let mut i = 0;
    while i < remotes.len() {
        let r = remotes[i];
        if rank(r.names) == level {
            return match tip_commit(r.tip) {
                Some(k) => Some((i, k)),
                None => None,
            };
        }
        i += 1;
    }
    None
}

/// Whether every remote at `level` is at commit `k`.
pub fn agree_at(remotes: &[Remote], level: u8, k: u32) -> bool {
    let mut i = 0;
    while i < remotes.len() {
        let r = remotes[i];
        if rank(r.names) == level && !tip_is(r.tip, k) {
            return false;
        }
        i += 1;
    }
    true
}

/// The target, in the RFC's order: the pin, no remotes, the candidates at
/// the strongest level, a candidate without a default branch, and
/// candidates that disagree.
pub fn choose(pinned: Pinned, remotes: &[Remote]) -> Choice {
    match pinned {
        Pinned::Resolves => return Choice::Pinned,
        Pinned::Missing => return Choice::Ambiguous(Why::PinMissing),
        Pinned::No => {}
    }
    if remotes.is_empty() {
        return Choice::Head;
    }
    let level = best(remotes);
    if level == 0 {
        return Choice::Ambiguous(Why::NoRemoteNames);
    }
    if headless_at(remotes, level) {
        return Choice::Ambiguous(Why::NoDefaultBranch);
    }
    match first_at(remotes, level) {
        None => Choice::Ambiguous(Why::NoDefaultBranch),
        Some((i, k)) => match agree_at(remotes, level, k) {
            true => Choice::Remote(i),
            false => Choice::Ambiguous(Why::Disagree),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn r(names: Names, tip: Option<u32>) -> Remote {
        Remote {
            names,
            tip: tip.map_or(Tip::NoHead, Tip::At),
        }
    }

    #[test]
    fn the_fork_layout_judges_upstream() {
        let fork = vec![r(Names::Other, Some(0)), r(Names::Path, Some(1))];
        assert_eq!(choose(Pinned::No, &fork), Choice::Remote(1));
        let only_fork = vec![r(Names::Other, Some(0))];
        assert_eq!(
            choose(Pinned::No, &only_fork),
            Choice::Ambiguous(Why::NoRemoteNames)
        );
    }

    #[test]
    fn the_table_in_order() {
        let none: Vec<Remote> = Vec::new();
        assert_eq!(choose(Pinned::No, &none), Choice::Head);
        assert_eq!(choose(Pinned::Resolves, &none), Choice::Pinned);
        assert_eq!(
            choose(Pinned::Missing, &[r(Names::Url, Some(0))]),
            Choice::Ambiguous(Why::PinMissing)
        );
        // The strongest level decides; weaker candidates are ignored.
        let mixed = [r(Names::Base, Some(5)), r(Names::Url, Some(1))];
        assert_eq!(choose(Pinned::No, &mixed), Choice::Remote(1));
        let agree = [r(Names::Path, Some(2)), r(Names::Path, Some(2))];
        assert_eq!(choose(Pinned::No, &agree), Choice::Remote(0));
        let disagree = [r(Names::Path, Some(2)), r(Names::Path, Some(3))];
        assert_eq!(
            choose(Pinned::No, &disagree),
            Choice::Ambiguous(Why::Disagree)
        );
        // A headless candidate wins over a disagreement, wherever it is.
        let headless = [
            r(Names::Path, Some(2)),
            r(Names::Path, Some(3)),
            r(Names::Path, None),
        ];
        assert_eq!(
            choose(Pinned::No, &headless),
            Choice::Ambiguous(Why::NoDefaultBranch)
        );
        let unread = [r(Names::Unread, Some(0)), r(Names::Sole, Some(0))];
        assert_eq!(choose(Pinned::No, &unread), Choice::Remote(1));
    }
}
