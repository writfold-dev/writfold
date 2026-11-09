//! Backend-independent conformance suite for `StorageBackend`.
//!
//! A test provisions the fixed dataset below into a real disposable instance of
//! its backend, commits `BEFORE` then `AFTER`, and passes the three pins. Every
//! adapter (Dolt now, Omnigraph later) must pass the same checks. Failures are
//! returned as values; the suite never panics, so callers decide how to report.
mod lookups;
mod reads;

use super::{Capability, ExactRead, LookupRequest, RecordKey, StorageBackend};
use std::fmt;
use std::sync::atomic::AtomicBool;

/// One record of the conformance dataset. Keys are in strict binary order.
#[derive(Clone, Copy, Debug)]
pub struct FixtureRecord {
    pub key: &'static str,
    pub kind: &'static str,
    pub content: &'static str,
}

const fn record(key: &'static str, kind: &'static str, content: &'static str) -> FixtureRecord {
    FixtureRecord { key, kind, content }
}

/// First committed state.
pub const BEFORE: &[FixtureRecord] = &[
    record("doc:a", "document", "café\r\nnaïve"),
    record("doc:b", "document", "service A depends on B"),
    record("svc:a", "service", "service A"),
    record("svc:b", "service", "'; DROP TABLE wf_nodes; --"),
];

/// Second committed state: `doc:a` changed, `svc:b` removed, `svc:c` added.
pub const AFTER: &[FixtureRecord] = &[
    record("doc:a", "document", "new evidence\r\n"),
    record("doc:b", "document", "service A depends on B"),
    record("svc:a", "service", "service A"),
    record("svc:c", "service", "service C"),
];

/// Pins supplied by the provisioning test. `missing` names a snapshot that does
/// not exist in the backend; it must refuse, never read as empty or latest.
#[derive(Clone, Debug)]
pub struct ConformanceFixture<P> {
    pub before: P,
    pub after: P,
    pub missing: P,
}

/// Every check the suite runs, in order. A report lists only checks that ran.
pub const CHECKS: &[&str] = &[
    "exact-read-before",
    "exact-read-after",
    "pinned-history",
    "digest-binding",
    "missing-key",
    "missing-pin",
    "exact-lookup",
    "zero-results-complete",
    "paging-and-coverage",
    "cursor-binding",
    "lexical-lookup",
    "unsupported-refused",
    "write-admission",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceFailure {
    pub check: &'static str,
    pub detail: String,
}

impl fmt::Display for ConformanceFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "conformance check {} failed: {}",
            self.check, self.detail
        )
    }
}

impl std::error::Error for ConformanceFailure {}

pub(super) type Outcome = Result<(), ConformanceFailure>;

pub(super) fn fail(check: &'static str, detail: impl Into<String>) -> ConformanceFailure {
    ConformanceFailure {
        check,
        detail: detail.into(),
    }
}

pub(super) fn require(check: &'static str, condition: bool, detail: &str) -> Outcome {
    if condition {
        Ok(())
    } else {
        Err(fail(check, detail))
    }
}

type Check<B> = (
    &'static str,
    fn(&mut B, &ConformanceFixture<<B as StorageBackend>::Pin>) -> Outcome,
);

/// Run every check against one provisioned backend. Returns the names of the
/// checks that ran and passed, which equals `CHECKS` on success.
pub fn run<B: StorageBackend>(
    backend: &mut B,
    fixture: &ConformanceFixture<B::Pin>,
) -> Result<Vec<&'static str>, ConformanceFailure> {
    let checks: [Check<B>; 13] = [
        ("exact-read-before", |b, f| {
            reads::dataset(b, &f.before, BEFORE, "exact-read-before")
        }),
        ("exact-read-after", |b, f| {
            reads::dataset(b, &f.after, AFTER, "exact-read-after")
        }),
        ("pinned-history", |b, f| reads::history(b, f)),
        ("digest-binding", |b, f| reads::digest_binding(b, f)),
        ("missing-key", |b, f| reads::missing_key(b, f)),
        ("missing-pin", |b, f| reads::missing_pin(b, f)),
        ("exact-lookup", |b, f| lookups::exact(b, f)),
        ("zero-results-complete", |b, f| lookups::zero_results(b, f)),
        ("paging-and-coverage", |b, f| lookups::paging(b, f)),
        ("cursor-binding", |b, f| lookups::cursor_binding(b, f)),
        ("lexical-lookup", |b, f| lookups::lexical(b, f)),
        ("unsupported-refused", |b, f| lookups::unsupported(b, f)),
        ("write-admission", |b, f| reads::write_admission(b, f)),
    ];
    let mut passed = Vec::with_capacity(checks.len());
    for (name, check) in checks {
        check(backend, fixture)?;
        passed.push(name);
    }
    Ok(passed)
}

pub(super) fn key(check: &'static str, value: &str) -> Result<RecordKey, ConformanceFailure> {
    RecordKey::new(value.to_owned()).map_err(|error| fail(check, error.to_string()))
}

pub(super) fn exact(check: &'static str, value: &str) -> Result<ExactRead, ConformanceFailure> {
    Ok(ExactRead::new(key(check, value)?, None))
}

pub(super) fn not_cancelled() -> AtomicBool {
    AtomicBool::new(false)
}

pub(super) fn declared<B: StorageBackend>(backend: &B, capability: Capability) -> bool {
    backend.capabilities().contains(&capability)
}

pub(super) fn request(
    check: &'static str,
    request: Result<LookupRequest, super::Refusal>,
) -> Result<LookupRequest, ConformanceFailure> {
    request.map_err(|error| fail(check, error.to_string()))
}
