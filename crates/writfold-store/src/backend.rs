//! Swappable storage-backend boundary (milestone B0, `docs/writfold/storage-backend.md`).
//!
//! One narrow trait covers exact pinned reads, exact/lexical lookup with measured
//! coverage and a pinned cursor, and single-writer admission against an expected
//! old head. Each backend keeps its own pin type: a Dolt pin is never cast into an
//! Omnigraph pin or back. Fuzzy, similarity, semantic and ranked queries are typed
//! refusals here; no backend emulates them with lexical matching.
//!
//! Results are navigation data bound to an exact pin. They are not knowledge
//! truth, publication, current authority or an EngineeringLedger head.
pub mod conformance;
pub mod dolt;
pub mod git;
mod refusal;
mod scan;
mod types;
mod write;

pub use refusal::{Capability, Refusal};
pub use scan::{ScanRecord, admit_lookup, content_digest, scan};
pub use types::{
    ExactRead, ExactRecord, HeadName, LookupCursor, LookupField, LookupHit, LookupPage,
    LookupRequest, MatchMode, Order, RecordKey, RecordKind,
};
pub use write::{RecordChange, WriteObservation, WriteOutcome, WriteRequest};

use std::fmt;
use std::sync::atomic::AtomicBool;

/// The backend family that produced a result. Pins stay separately typed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendKind {
    /// Interim primary backend (owner decision, 9 November 2026).
    Dolt,
    /// Target backend once P01O/WF24 admits it; no adapter exists yet.
    Omnigraph,
    /// Read-only blobs of one pinned Git commit (Tuft v0 `find`); no writes.
    GitTree,
}

/// A storage backend behind the Writfold boundary.
///
/// Every successful result names the exact pin it was read at, and that pin is
/// equal to the pin the caller supplied. A missing pin or missing history is a
/// refusal, never an empty or latest-state success. One backend instance per
/// realm is the only writer: `admit_write` takes `&mut self`, a writer instance
/// holds the realm's exclusive writer lease, there are no dual writes, and an
/// `Unknown` write outcome is reconciled with `observe` against this backend,
/// never retried through another one.
pub trait StorageBackend {
    /// Backend-specific exact pin. It is never converted to another backend's pin.
    type Pin: Clone + Eq + fmt::Debug;

    /// Typed evidence a write needs before any IO (for Dolt, a verified signed
    /// approval). It is checked against the request, never inferred.
    type WriteAuthority;

    fn kind(&self) -> BackendKind;

    /// Capabilities this backend actually implements. Anything absent is refused
    /// with `Unsupported` (never emulated) or `Unavailable` (defined, not built).
    fn capabilities(&self) -> &'static [Capability];

    /// Retained bytes of one record at an exact pin, optionally bound to the
    /// digest the caller already holds (for example from a citation binding).
    fn read_exact(
        &self,
        pin: &Self::Pin,
        request: &ExactRead,
        cancelled: &AtomicBool,
    ) -> Result<ExactRecord<Self::Pin>, Refusal>;

    /// Exact or lexical lookup at an exact pin with bounded hits, measured
    /// coverage and a continuation cursor bound to the pin and the request.
    fn lookup(
        &self,
        pin: &Self::Pin,
        request: &LookupRequest,
        cursor: Option<&LookupCursor<Self::Pin>>,
        cancelled: &AtomicBool,
    ) -> Result<LookupPage<Self::Pin>, Refusal>;

    /// Single-writer admission: apply `changes` only if `head` still names
    /// `expected_old`, as one atomic conditional commit. A stale head returns the
    /// observed pin and writes nothing. Missing or mismatched authority, a realm
    /// this instance does not own and a non-writer instance refuse before IO.
    fn admit_write(
        &mut self,
        request: &WriteRequest<Self::Pin>,
        authority: Option<&Self::WriteAuthority>,
        cancelled: &AtomicBool,
    ) -> Result<WriteOutcome<Self::Pin>, Refusal>;

    /// Reconcile one write request without writing: the current head and the
    /// commit carrying exactly this request, if present. Never retries.
    fn observe(
        &self,
        request: &WriteRequest<Self::Pin>,
        cancelled: &AtomicBool,
    ) -> Result<WriteObservation<Self::Pin>, Refusal> {
        let _ = (request, cancelled);
        Err(Refusal::Unavailable(Capability::Write))
    }

    /// Ensure the retention root of `pin`, the current head, so an exact read
    /// of a head left by an interrupted write is admitted. Idempotent; writes
    /// no record and never moves the head. Writer instances only.
    fn retain(&mut self, pin: &Self::Pin, cancelled: &AtomicBool) -> Result<(), Refusal> {
        let _ = (pin, cancelled);
        Err(Refusal::Unavailable(Capability::Write))
    }
}
