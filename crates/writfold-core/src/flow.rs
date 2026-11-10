//! The edit-flow validator of `writfold-trial-worktree/1`: one run is one
//! ledger stream, exactly `Prepared` → `Approved` → `Claimed` → `Applied` or
//! `Abandoned`. Each record carries its retained material in its closure;
//! the claim and `Applied` name exactly the candidate `Prepared` named; an
//! abandonment says why. Order and bindings only: signatures, grants and the
//! file guard are checked by `apply` before `Claimed` is appended.
use crate::bytes::{contains_digest, same};
use alloc::vec::Vec;

/// A retained object: its digest and exact byte length.
pub struct Blob {
    pub digest: [u8; 32],
    pub bytes: u64,
}

/// One record's payload, reduced to what the validator reads.
pub enum Event {
    Prepared {
        path: Vec<u8>,
        source: Blob,
        recipe: Blob,
        candidate: Blob,
        report: Blob,
    },
    Approved {
        grant: Blob,
        approval: Blob,
    },
    Claimed {
        candidate: Blob,
    },
    Applied {
        candidate: Blob,
    },
    Abandoned {
        reason: Vec<u8>,
    },
}

/// The same retained object: same digest and same length.
pub fn same_blob(left: &Blob, right: &Blob) -> bool {
    same(&left.digest, &right.digest) && left.bytes == right.bytes
}

/// Every blob the payload names is in the record's closure.
pub fn carried(event: &Event, closure: &[[u8; 32]]) -> bool {
    match event {
        Event::Prepared {
            source,
            recipe,
            candidate,
            report,
            ..
        } => {
            contains_digest(closure, &source.digest)
                && contains_digest(closure, &recipe.digest)
                && contains_digest(closure, &candidate.digest)
                && contains_digest(closure, &report.digest)
        }
        Event::Approved { grant, approval } => {
            contains_digest(closure, &grant.digest) && contains_digest(closure, &approval.digest)
        }
        Event::Claimed { candidate } => contains_digest(closure, &candidate.digest),
        Event::Applied { candidate } => contains_digest(closure, &candidate.digest),
        Event::Abandoned { .. } => true,
    }
}

pub fn is_prepared(event: &Event) -> bool {
    match event {
        Event::Prepared { .. } => true,
        _ => false,
    }
}

pub fn is_approved(event: &Event) -> bool {
    match event {
        Event::Approved { .. } => true,
        _ => false,
    }
}

pub fn is_claimed(event: &Event) -> bool {
    match event {
        Event::Claimed { .. } => true,
        _ => false,
    }
}

/// Whether `first` prepared exactly this candidate.
pub fn prepared(first: &Event, candidate: &Blob) -> bool {
    match first {
        Event::Prepared { candidate: c, .. } => same_blob(c, candidate),
        _ => false,
    }
}

/// Whether `event` is the next record of a run whose records so far are `prefix`.
pub fn follows(prefix: &[Event], event: &Event) -> bool {
    match event {
        Event::Prepared { path, .. } => prefix.is_empty() && !path.is_empty(),
        Event::Approved { .. } => prefix.len() == 1 && is_prepared(&prefix[0]),
        Event::Claimed { candidate } => {
            prefix.len() == 2 && is_approved(&prefix[1]) && prepared(&prefix[0], candidate)
        }
        Event::Applied { candidate } => {
            prefix.len() == 3 && is_claimed(&prefix[2]) && prepared(&prefix[0], candidate)
        }
        Event::Abandoned { reason } => {
            prefix.len() == 3 && is_claimed(&prefix[2]) && !reason.is_empty()
        }
    }
}

/// The edit-flow rule: the record carries its material and follows its prefix.
pub fn validate(prefix: &[Event], event: &Event, closure: &[[u8; 32]]) -> bool {
    carried(event, closure) && follows(prefix, event)
}
