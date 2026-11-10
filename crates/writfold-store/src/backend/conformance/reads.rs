use super::{
    AFTER, BEFORE, ConformanceFixture, FixtureRecord, Outcome, declared, exact, fail, key,
    not_cancelled, require,
};
use crate::backend::{
    Capability, ExactRead, HeadName, RecordChange, RecordKind, Refusal, StorageBackend,
    WriteRequest, content_digest,
};

/// Every record reads back with exact bytes, kind, recomputed digest and the
/// requested pin.
pub(super) fn dataset<B: StorageBackend>(
    backend: &mut B,
    pin: &B::Pin,
    records: &[FixtureRecord],
    check: &'static str,
) -> Outcome {
    for record in records {
        let read = backend
            .read_exact(pin, &exact(check, record.key)?, &not_cancelled())
            .map_err(|error| fail(check, format!("{}: {error}", record.key)))?;
        let digest = content_digest(record.content.as_bytes())
            .map_err(|error| fail(check, error.to_string()))?;
        require(
            check,
            read.pin() == pin,
            "result pin differs from the request",
        )?;
        require(check, read.key().as_str() == record.key, "key differs")?;
        require(check, read.kind().as_str() == record.kind, "kind differs")?;
        require(
            check,
            read.content() == record.content.as_bytes(),
            "bytes differ",
        )?;
        require(
            check,
            read.digest() == &digest,
            "digest is not SHA-256 of the bytes",
        )?;
    }
    Ok(())
}

/// The older pin keeps its own bytes after the head advanced, and a record
/// removed later is absent only at the later pin.
pub(super) fn history<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "pinned-history";
    let cancel = not_cancelled();
    let old = backend
        .read_exact(&f.before, &exact(CHECK, "doc:a")?, &cancel)
        .map_err(|error| fail(CHECK, error.to_string()))?;
    require(
        CHECK,
        old.content() == BEFORE[0].content.as_bytes(),
        "old pin read newer bytes",
    )?;
    let new = backend
        .read_exact(&f.after, &exact(CHECK, "doc:a")?, &cancel)
        .map_err(|error| fail(CHECK, error.to_string()))?;
    require(
        CHECK,
        new.content() == AFTER[0].content.as_bytes(),
        "new pin read older bytes",
    )?;
    backend
        .read_exact(&f.before, &exact(CHECK, "svc:b")?, &cancel)
        .map_err(|error| fail(CHECK, error.to_string()))?;
    let removed = backend.read_exact(&f.after, &exact(CHECK, "svc:b")?, &cancel);
    require(
        CHECK,
        removed == Err(Refusal::NotFound),
        "removed record not refused at new pin",
    )
}

/// A caller-held digest binds the read: equal bytes pass, other bytes refuse.
pub(super) fn digest_binding<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "digest-binding";
    let cancel = not_cancelled();
    let held =
        content_digest(BEFORE[0].content.as_bytes()).map_err(|e| fail(CHECK, e.to_string()))?;
    let bound = ExactRead::new(key(CHECK, "doc:a")?, Some(held));
    backend
        .read_exact(&f.before, &bound, &cancel)
        .map_err(|error| fail(CHECK, error.to_string()))?;
    let changed = backend.read_exact(&f.after, &bound, &cancel);
    require(
        CHECK,
        changed == Err(Refusal::DigestMismatch),
        "changed bytes were not refused",
    )
}

pub(super) fn missing_key<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "missing-key";
    let read = backend.read_exact(&f.before, &exact(CHECK, "svc:z")?, &not_cancelled());
    require(
        CHECK,
        read == Err(Refusal::NotFound),
        "absent key did not refuse NotFound",
    )
}

/// A pin naming no snapshot refuses; it is never an empty or latest success.
pub(super) fn missing_pin<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "missing-pin";
    let read = backend.read_exact(&f.missing, &exact(CHECK, "doc:a")?, &not_cancelled());
    require(
        CHECK,
        matches!(read, Err(ref error) if *error != Refusal::NotFound),
        "missing pin did not refuse as a pin or backend failure",
    )
}

/// `run` checks reader instances: one that does not declare `Write` refuses a
/// write as `Unavailable(Write)`. Writer instances are checked by `run_writes`,
/// so a reader that declares `Write` fails here rather than passing unchecked.
pub(super) fn write_admission<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "write-admission";
    if declared(backend, Capability::Write) {
        return Err(fail(
            CHECK,
            "run takes a reader instance; writer conformance is run_writes",
        ));
    }
    let change = RecordChange {
        key: key(CHECK, "doc:c")?,
        kind: RecordKind::new("document".to_owned()).map_err(|e| fail(CHECK, e.to_string()))?,
        content: Some(b"conformance write".to_vec()),
    };
    let stale = WriteRequest {
        head: HeadName::new("main".to_owned()).map_err(|e| fail(CHECK, e.to_string()))?,
        expected_old: f.before.clone(),
        changes: vec![change],
    };
    let outcome = backend.admit_write(&stale, None, &not_cancelled());
    require(
        CHECK,
        outcome == Err(Refusal::Unavailable(Capability::Write)),
        "undeclared write was not refused as Unavailable",
    )
}
