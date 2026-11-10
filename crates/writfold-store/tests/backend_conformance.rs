//! Calibrates the storage conformance suite itself: an in-memory reference
//! backend passes the read and writer checks, and each deliberately broken
//! backend fails the check that names its defect. The real Dolt run is
//! `backend_dolt.rs`.
mod memory_backend;

use memory_backend::{Grant, Memory, MemoryPin, Mutation, Shared, pin, realm};
use writfold_store::backend::conformance::{
    self, CHECKS, ConformanceFailure, ConformanceFixture, WRITE_CHECKS, WriterFixture,
};
use writfold_store::backend::{
    LookupRequest, MatchMode, Order, RecordKey, RecordKind, Refusal, WriteRequest,
};

fn fixture() -> ConformanceFixture<MemoryPin> {
    ConformanceFixture {
        before: pin(0),
        after: pin(1),
        missing: pin(9),
    }
}

fn grant(request: &WriteRequest<MemoryPin>) -> Result<Grant, Refusal> {
    Ok(Grant(request.expected_old))
}

/// Reads with a reader instance, then writes with writer instances.
fn run_all(mutation: Mutation) -> Result<Vec<&'static str>, ConformanceFailure> {
    let shared: Shared = realm();
    let mut passed = conformance::run(&mut Memory::reader(mutation, shared.clone()), &fixture())?;
    let open = || Memory::writer(mutation, shared.clone());
    let writes = WriterFixture {
        open: &open,
        authorize: &grant,
        foreign: MemoryPin {
            realm: 7,
            snapshot: 1,
        },
    };
    passed.extend(conformance::run_writes(&fixture(), &writes)?.passed);
    Ok(passed)
}

#[test]
fn faithful_reference_backend_passes_every_check() -> Result<(), Box<dyn std::error::Error>> {
    let passed = run_all(Mutation::Faithful)?;
    assert_eq!(passed, [CHECKS, WRITE_CHECKS].concat());
    Ok(())
}

#[test]
fn each_broken_backend_fails_the_check_naming_its_defect() {
    for (mutation, check) in [
        (Mutation::IgnorePin, "exact-read-before"),
        (Mutation::LatestOnMissing, "missing-pin"),
        (Mutation::FuzzyAsSubstring, "unsupported-refused"),
        (Mutation::TruncateWithoutCursor, "paging-and-coverage"),
        (Mutation::IgnoreCursorBinding, "cursor-binding"),
        (Mutation::WriteNotRefused, "write-admission"),
        (Mutation::WriteWithoutAuthority, "write-authority"),
        (Mutation::CommitUnobservable, "write-expected-old"),
        (Mutation::IgnoreExpectedOld, "write-stale-no-change"),
        (Mutation::NoLeaseRacyCompareAndSwap, "write-concurrent"),
        (Mutation::ForeignRealmAccepted, "write-ownership"),
        (Mutation::CancelledAfterCommit, "write-kill-reconcile"),
    ] {
        let failure = run_all(mutation);
        assert!(
            matches!(&failure, Err(failure) if failure.check == check),
            "{mutation:?}: {failure:?}"
        );
    }
}

#[test]
fn a_writer_instance_is_not_accepted_as_a_reader() {
    let shared = realm();
    let writer = Memory::writer(Mutation::Faithful, shared);
    let failure = writer.map(|mut writer| conformance::run(&mut writer, &fixture()));
    assert!(
        matches!(&failure, Ok(Err(failure)) if failure.check == "write-admission"),
        "{failure:?}"
    );
}

#[test]
fn request_bounds_refuse_before_any_backend_call() {
    for (needle, limit) in [("", 1), ("x", 0), ("x", 1025), ("a\0b", 1)] {
        assert!(matches!(
            LookupRequest::new(
                writfold_store::backend::LookupField::Key,
                MatchMode::Exact,
                Order::Key,
                needle.to_owned(),
                limit
            ),
            Err(Refusal::InvalidRequest(_))
        ));
    }
    assert!(RecordKey::new(String::new()).is_err());
    assert!(RecordKey::new("k".repeat(1025)).is_err());
    assert!(RecordKind::new("k".repeat(65)).is_err());
}
