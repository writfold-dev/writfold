//! Writer conformance: expected-old success, stale refusal without change,
//! authority and ownership refusals. Concurrency and kill checks are in
//! `races.rs`. Every check reconciles through `observe`, so a refusal that
//! secretly wrote is caught.
use super::{
    AFTER, ConformanceFailure, ConformanceFixture, Outcome, WriterFixture, exact, fail, key,
    not_cancelled, require,
};
use crate::backend::{
    HeadName, RecordChange, RecordKind, Refusal, StorageBackend, WriteObservation, WriteOutcome,
    WriteRequest,
};

pub(super) const KIND: &str = "document";
pub(super) const WRITTEN: &str = "conformance write";

/// A one-change request on head `main` from `expected_old`.
pub(super) fn request<P>(
    check: &'static str,
    expected_old: &P,
    record: &str,
    content: &str,
) -> Result<WriteRequest<P>, ConformanceFailure>
where
    P: Clone,
{
    Ok(WriteRequest {
        head: HeadName::new("main".to_owned()).map_err(|e| fail(check, e.to_string()))?,
        expected_old: expected_old.clone(),
        changes: vec![RecordChange {
            key: key(check, record)?,
            kind: RecordKind::new(KIND.to_owned()).map_err(|e| fail(check, e.to_string()))?,
            content: Some(content.as_bytes().to_vec()),
        }],
    })
}

pub(super) fn authorize<B: StorageBackend>(
    check: &'static str,
    writes: &WriterFixture<'_, B>,
    request: &WriteRequest<B::Pin>,
) -> Result<B::WriteAuthority, ConformanceFailure> {
    (writes.authorize)(request).map_err(|error| fail(check, format!("authorize: {error}")))
}

pub(super) fn open<B: StorageBackend>(
    check: &'static str,
    writes: &WriterFixture<'_, B>,
) -> Result<B, ConformanceFailure> {
    (writes.open)().map_err(|error| fail(check, format!("open writer: {error}")))
}

pub(super) fn observe<B: StorageBackend>(
    check: &'static str,
    backend: &B,
    request: &WriteRequest<B::Pin>,
) -> Result<WriteObservation<B::Pin>, ConformanceFailure> {
    backend
        .observe(request, &not_cancelled())
        .map_err(|error| fail(check, format!("observe: {error}")))
}

/// `record` reads `content` at `pin`.
pub(super) fn reads<B: StorageBackend>(
    check: &'static str,
    backend: &B,
    pin: &B::Pin,
    record: &str,
    content: &str,
) -> Outcome {
    let read = backend
        .read_exact(pin, &exact(check, record)?, &not_cancelled())
        .map_err(|error| fail(check, format!("{record}: {error}")))?;
    require(
        check,
        read.content() == content.as_bytes() && read.pin() == pin,
        "written bytes do not read back at the new pin",
    )
}

/// The head is `head` and `request` is absent from its history.
pub(super) fn unchanged<B: StorageBackend>(
    check: &'static str,
    backend: &B,
    request: &WriteRequest<B::Pin>,
    head: &B::Pin,
) -> Outcome {
    let seen = observe(check, backend, request)?;
    require(
        check,
        &seen.head == head && seen.applied.is_none(),
        "a refused write changed the head or left its commit",
    )
}

/// Missing authority and authority for another request refuse, writing nothing.
pub(super) fn authority<B: StorageBackend>(
    backend: &mut B,
    writes: &WriterFixture<'_, B>,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "write-authority";
    let write = request(CHECK, &f.after, "doc:c", WRITTEN)?;
    let missing = backend.admit_write(&write, None, &not_cancelled());
    require(
        CHECK,
        matches!(missing, Err(Refusal::Unauthorized(_))),
        "a write without authority was not refused as Unauthorized",
    )?;
    let other = authorize(CHECK, writes, &request(CHECK, &f.before, "doc:c", WRITTEN)?)?;
    let mismatched = backend.admit_write(&write, Some(&other), &not_cancelled());
    require(
        CHECK,
        matches!(mismatched, Err(Refusal::Unauthorized(_))),
        "authority for another expected-old head was accepted",
    )?;
    unchanged(CHECK, backend, &write, &f.after)
}

/// A current expected-old head commits, the new head is observed, the new pin
/// reads the change and the old pin keeps its history. Returns the new head.
pub(super) fn expected_old<B: StorageBackend>(
    backend: &mut B,
    writes: &WriterFixture<'_, B>,
    f: &ConformanceFixture<B::Pin>,
) -> Result<B::Pin, ConformanceFailure> {
    const CHECK: &str = "write-expected-old";
    let write = request(CHECK, &f.after, "doc:c", WRITTEN)?;
    let grant = authorize(CHECK, writes, &write)?;
    let outcome = backend.admit_write(&write, Some(&grant), &not_cancelled());
    let Ok(WriteOutcome::Committed { new }) = outcome else {
        return Err(fail(CHECK, format!("expected Committed, got {outcome:?}")));
    };
    require(CHECK, new != f.after, "the committed head did not move")?;
    let seen = observe(CHECK, backend, &write)?;
    require(
        CHECK,
        seen.head == new && seen.applied.as_ref() == Some(&new),
        "observe does not report the committed head and its commit",
    )?;
    reads(CHECK, backend, &new, "doc:c", WRITTEN)?;
    reads(CHECK, backend, &new, AFTER[0].key, AFTER[0].content)?;
    let old = backend.read_exact(&f.after, &exact(CHECK, "doc:c")?, &not_cancelled());
    require(
        CHECK,
        old == Err(Refusal::NotFound),
        "the old pin reads the newer write",
    )?;
    Ok(new)
}

/// A stale expected-old head refuses with the observed head and writes nothing.
pub(super) fn stale<B: StorageBackend>(
    backend: &mut B,
    writes: &WriterFixture<'_, B>,
    f: &ConformanceFixture<B::Pin>,
    head: &B::Pin,
) -> Outcome {
    const CHECK: &str = "write-stale-no-change";
    let write = request(CHECK, &f.after, "doc:c", "stale write")?;
    let grant = authorize(CHECK, writes, &write)?;
    let outcome = backend.admit_write(&write, Some(&grant), &not_cancelled());
    require(
        CHECK,
        matches!(&outcome, Ok(WriteOutcome::Stale { observed }) if observed == head),
        "a stale expected-old head was not refused with the observed head",
    )?;
    unchanged(CHECK, backend, &write, head)?;
    reads(CHECK, backend, head, "doc:c", WRITTEN)
}

/// While one writer is live a second cannot open, and a pin in a realm the
/// writer does not own refuses before any write.
pub(super) fn ownership<B: StorageBackend>(
    backend: &mut B,
    writes: &WriterFixture<'_, B>,
    head: &B::Pin,
) -> Outcome {
    const CHECK: &str = "write-ownership";
    let second = (writes.open)();
    require(
        CHECK,
        matches!(second, Err(Refusal::NotOwner(_))),
        "a second writer opened while the first held the realm",
    )?;
    drop(second);
    let foreign = request(CHECK, &writes.foreign, "doc:f", "foreign write")?;
    let grant = (writes.authorize)(&foreign).ok();
    let outcome = backend.admit_write(&foreign, grant.as_ref(), &not_cancelled());
    require(
        CHECK,
        matches!(outcome, Err(Refusal::NotOwner(_))),
        "a write to a realm the writer does not own was not refused",
    )?;
    let probe = request(CHECK, head, "doc:f", "foreign write")?;
    unchanged(CHECK, backend, &probe, head)
}
