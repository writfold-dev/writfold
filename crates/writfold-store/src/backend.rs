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
mod refusal;
mod scan;
mod types;

pub use refusal::{Capability, Refusal};
pub use scan::{ScanRecord, admit_lookup, content_digest, scan};
pub use types::{
    ExactRead, ExactRecord, HeadName, LookupCursor, LookupField, LookupHit, LookupPage,
    LookupRequest, MatchMode, Order, RecordChange, RecordKey, RecordKind, WriteOutcome,
    WriteRequest,
};

use std::fmt;
use std::sync::atomic::AtomicBool;

/// The backend family that produced a result. Pins stay separately typed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendKind {
    /// Interim primary backend (owner decision, 9 November 2026).
    Dolt,
    /// Target backend once P01O/WF24 admits it; no adapter exists yet.
    Omnigraph,
}

/// A storage backend behind the Writfold boundary.
///
/// Every successful result names the exact pin it was read at, and that pin is
/// equal to the pin the caller supplied. A missing pin or missing history is a
/// refusal, never an empty or latest-state success. One backend instance per
/// realm is the only writer: `admit_write` takes `&mut self`, there are no dual
/// writes, and an `Unknown` write outcome is reconciled against this backend,
/// never retried through another one.
pub trait StorageBackend {
    /// Backend-specific exact pin. It is never converted to another backend's pin.
    type Pin: Clone + Eq + fmt::Debug;

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
    /// `expected_old`. A stale head returns the observed pin and writes nothing.
    fn admit_write(
        &mut self,
        request: &WriteRequest<Self::Pin>,
        cancelled: &AtomicBool,
    ) -> Result<WriteOutcome<Self::Pin>, Refusal>;
}
