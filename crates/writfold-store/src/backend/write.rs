//! Single-writer conditional-head requests, outcomes and reconciliation.
use super::{HeadName, RecordKey, RecordKind};

/// `content: None` deletes the record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordChange {
    pub key: RecordKey,
    pub kind: RecordKind,
    pub content: Option<Vec<u8>>,
}

/// Apply `changes` only if `head` still names `expected_old`. The request's
/// content (head, expected-old pin and ordered changes) is its identity: the
/// backend records it with the commit, and `observe` finds it by it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteRequest<P> {
    pub head: HeadName,
    pub expected_old: P,
    pub changes: Vec<RecordChange>,
}

/// `Committed` names the observed new head. `Stale` names the head actually
/// found and guarantees nothing was written. `Unknown` means the backend cannot
/// say whether the write happened (the process died, was killed or timed out
/// after it might have committed). The caller reconciles with `observe` on the
/// same backend; it never replays the write or retries it elsewhere.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteOutcome<P> {
    Committed { new: P },
    Stale { observed: P },
    Unknown,
}

/// A read-only reconciliation of one write request against the current head.
/// `applied` is the pin of the commit that carries exactly this request, if it
/// is on the head's history; `None` means the write did not happen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteObservation<P> {
    pub head: P,
    pub applied: Option<P>,
}

/// The realm's storage origin: the head before the first write this boundary
/// made, read-only. `written` says whether such a write is on the head's
/// history. A bootstrap publication writes only from an unwritten origin, and
/// its recovery finds its request from the origin again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin<P> {
    pub pin: P,
    pub written: bool,
}
