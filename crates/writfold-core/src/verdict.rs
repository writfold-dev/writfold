//! The citation verdict behind `writfold kb stale`: from what a check
//! observed (blob equality at the target commit, path presence, where the
//! cited span now sits, expiry, a recorded state) to exactly one verdict.
//! Nothing here reads a repository; the caller gathers the facts.
//!
//! The decisions are named one by one in the Charon `start_from` list
//! (the derives and the closure-driven [`locate`] are not translated) and
//! proved in `proofs/Writfold/Verdict.lean` (docs/writfold/aeneas.md).
//! `writfold-kb`'s property tests check that a fresh verdict means the
//! cited lines are unchanged after normalisation and that a moved verdict
//! names a range holding the cited span digest.

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

/// What a code citation's path holds at the target commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// The cited blob.
    SameBlob,
    /// Another blob. The flag says whether the cited blob is known here
    /// (from `blob=`, or from `rev=` when this clone holds that commit), so
    /// that a blob was compared at all; then where the span sits in the
    /// file, or `None` when the citation carries no span digest (#90).
    OtherBlob(bool, Option<Located>),
    /// Nothing at the path, and how following it ended.
    Missing(Absence),
}

/// How a path absent at the target commit was followed (#90).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Absence {
    /// The cited blob sits at another path: an exact rename.
    Renamed,
    /// Git found a rename with edits from the cited commit; where the span
    /// sits in the new file, or `None` without a span digest.
    Followed(Option<Located>),
    /// The follow ran and found no rename: the path was removed.
    NoRename,
    /// The cited commit is not in this clone (a shallow or single-branch
    /// clone, an unfetched or rewritten commit): nothing to follow from.
    RevAbsent,
    /// The citation has no `rev=` to follow from.
    NoRev,
    /// Not followed: over the per-run limit of cited commits.
    Capped,
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
    /// A kind declared `checked`: what its external checker said.
    Checked(Checker),
}

/// The status an external checker gave one token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Said {
    Fresh,
    Moved,
    Changed,
    Gone,
    Unverifiable,
}

/// What became of one token sent to (or meant for) an external checker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Checker {
    /// No helper for the kind was found.
    Absent,
    /// The helper failed: a timeout, a non-zero exit, a malformed, missing
    /// or duplicate reply, an unknown status, or a refused handshake.
    Failed,
    /// The token's format version is not one the helper reads.
    Version,
    /// A well-formed reply to this token.
    Said(Said),
    /// A cached reply older than the kind's declared `max_age` (or whose
    /// age cannot be read): no longer evidence of anything.
    Expired,
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
        Observed::Code(Target::OtherBlob(known, located)) => other_blob(known, located),
        Observed::Code(Target::Missing(absence)) => missing(absence),
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
        Observed::Checked(checker) => match checker {
            Checker::Said(Said::Fresh) => Verdict::Fresh,
            Checker::Said(Said::Moved) => Verdict::Moved,
            Checker::Said(Said::Changed) => Verdict::Changed,
            Checker::Said(Said::Gone) => Verdict::Gone,
            Checker::Said(Said::Unverifiable)
            | Checker::Absent
            | Checker::Failed
            | Checker::Version
            | Checker::Expired => Verdict::Unverifiable,
        },
    }
}

/// Another blob at the cited path. A located span decides from the bytes
/// at the target alone. Otherwise only a compared blob says changed: with
/// no cited blob known there is no evidence of a change (#90).
pub fn other_blob(known: bool, located: Option<Located>) -> Verdict {
    match located {
        Some(Located::Same) => Verdict::Fresh,
        Some(Located::At(_, _)) => Verdict::Moved,
        Some(Located::Absent) => Verdict::Changed,
        _ => {
            if known {
                Verdict::Changed
            } else {
                Verdict::Unverifiable
            }
        }
    }
}

/// A path absent at the target. Gone needs a completed follow; a rename is
/// never fresh; a follow that could not run is no evidence (#90).
pub fn missing(absence: Absence) -> Verdict {
    match absence {
        Absence::Renamed => Verdict::Renamed,
        Absence::Followed(located) => match located {
            Some(Located::Same) => Verdict::Moved,
            Some(Located::At(_, _)) => Verdict::Moved,
            _ => Verdict::Gone,
        },
        Absence::NoRename => Verdict::Gone,
        Absence::RevAbsent => Verdict::Unverifiable,
        Absence::NoRev => Verdict::Unverifiable,
        Absence::Capped => Verdict::Unverifiable,
    }
}

/// What a checker's `describe` reply claims, as far as one kind needs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Described {
    /// It speaks the protocol version Writfold sent.
    pub protocol: bool,
    /// It lists the kind among its `kinds`.
    pub lists_kind: bool,
    /// It lists the operation Writfold needs (`check` or `make`).
    pub has_op: bool,
}

/// The handshake's outcome for one kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Handshake {
    Agreed,
    /// Another protocol version (or no describe reply at all).
    Protocol,
    /// The kind is not listed.
    Kind,
    /// The operation is not listed.
    Op,
}

/// Whether a helper may be asked about a kind: the protocol first, then
/// the kind, then the operation. Only [`Handshake::Agreed`] lets a reply
/// count; every other outcome makes each token [`Checker::Failed`].
pub fn handshake(described: Option<Described>) -> Handshake {
    match described {
        None => Handshake::Protocol,
        Some(d) => {
            if !d.protocol {
                Handshake::Protocol
            } else if !d.lists_kind {
                Handshake::Kind
            } else if !d.has_op {
                Handshake::Op
            } else {
                Handshake::Agreed
            }
        }
    }
}

/// What one token's exchange came to: the batch's handshake and ending,
/// whether the token's version is readable, and the replies to its id.
/// A reply counts only after an agreed handshake, a clean exit, a
/// readable version and exactly one well-formed reply.
pub fn exchange(
    handshake: Handshake,
    clean_exit: bool,
    readable: bool,
    replies: &[Option<Said>],
) -> Checker {
    if !matches!(handshake, Handshake::Agreed) || !clean_exit {
        return Checker::Failed;
    }
    if !readable {
        return Checker::Version;
    }
    if replies.len() != 1 {
        return Checker::Failed;
    }
    reply(replies[0])
}

/// One well-formed reply is what it says; a malformed one is a failure.
pub fn reply(reply: Option<Said>) -> Checker {
    match reply {
        Some(said) => Checker::Said(said),
        None => Checker::Failed,
    }
}

/// A cached checker reply as a cache reader sees it: unchanged without a
/// `max_age`, else [`Checker::Expired`] once its age in seconds exceeds
/// it or cannot be read (`None`).
pub fn aged(cached: Checker, age: Option<i64>, max_age: Option<i64>) -> Checker {
    match max_age {
        None => cached,
        Some(max) => match age {
            Some(age) => {
                if age <= max {
                    cached
                } else {
                    Checker::Expired
                }
            }
            None => Checker::Expired,
        },
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
    if immutable {
        return Lifetime::Immutable;
    }
    match token {
        Some(ttl) => Lifetime::Token(ttl),
        None => match default {
            Some(ttl) => Lifetime::Default(ttl),
            None => Lifetime::Never,
        },
    }
}

/// The final verdict: an authored mark wins over every verdict except
/// changed, gone and expired, which say more.
pub fn decide(fact: Observed, marked: bool) -> Verdict {
    with_mark(observed(fact), marked)
}

/// Apply an authored mark to a verdict already reached.
pub fn with_mark(reached: Verdict, marked: bool) -> Verdict {
    match reached {
        Verdict::Changed | Verdict::Gone | Verdict::Expired => reached,
        _ => {
            if marked {
                Verdict::Marked
            } else {
                reached
            }
        }
    }
}

/// Whether a time-based citation seen at `seen` with a lifetime of `ttl`
/// seconds has expired at `now`: `now - seen > ttl`, over the integers.
/// The difference is never formed where it would overflow: past either
/// end of `i64` its sign alone decides.
pub fn expired(now: i64, seen: i64, ttl: i64) -> bool {
    if seen < 0 && now > i64::MAX + seen {
        return true;
    }
    if seen > 0 && now < i64::MIN + seen {
        return false;
    }
    now - seen > ttl
}

mod locate;
pub use locate::{Place, Window, locate, place, window};
mod recorded;
pub use recorded::{
    CLOCK_SKEW, Pick, Statement, before, best_recorded, better, newest, recordable, recorded_age,
    usable, worst,
};

#[cfg(test)]
mod tests;
