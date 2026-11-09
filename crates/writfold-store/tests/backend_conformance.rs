//! Calibrates the storage conformance suite itself: an in-memory reference
//! backend passes, and each deliberately broken backend fails the check that
//! names its defect. The real Dolt run is `backend_dolt.rs`.
use std::sync::atomic::AtomicBool;
use writfold_domain::citation::Coverage;
use writfold_store::backend::conformance::{
    self, AFTER, BEFORE, CHECKS, ConformanceFixture, FixtureRecord,
};
use writfold_store::backend::{
    BackendKind, Capability, ExactRead, ExactRecord, LookupCursor, LookupPage, LookupRequest,
    MatchMode, Order, RecordKey, RecordKind, Refusal, ScanRecord, StorageBackend, WriteOutcome,
    WriteRequest, admit_lookup, scan,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MemoryPin(u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mutation {
    Faithful,
    IgnorePin,
    LatestOnMissing,
    FuzzyAsSubstring,
    TruncateWithoutCursor,
    IgnoreCursorBinding,
    WriteNotRefused,
}

struct Memory(Mutation);

const CAPABILITIES: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
];

impl Memory {
    fn records(&self, pin: &MemoryPin) -> Result<&'static [FixtureRecord], Refusal> {
        match (pin.0, self.0) {
            (_, Mutation::IgnorePin) => Ok(AFTER),
            (0, _) => Ok(BEFORE),
            (1, _) => Ok(AFTER),
            (_, Mutation::LatestOnMissing) => Ok(AFTER),
            _ => Err(Refusal::Pin("no such snapshot".into())),
        }
    }
}

fn rows(records: &[FixtureRecord]) -> Vec<ScanRecord<'_>> {
    records
        .iter()
        .map(|record| ScanRecord {
            key: record.key,
            kind: record.kind,
            content: record.content.as_bytes(),
        })
        .collect()
}

impl StorageBackend for Memory {
    type Pin = MemoryPin;

    fn kind(&self) -> BackendKind {
        BackendKind::Dolt
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }

    fn read_exact(
        &self,
        pin: &MemoryPin,
        request: &ExactRead,
        _: &AtomicBool,
    ) -> Result<ExactRecord<MemoryPin>, Refusal> {
        let records = self.records(pin)?;
        let record = records
            .iter()
            .find(|record| record.key == request.key().as_str())
            .ok_or(Refusal::NotFound)?;
        let kind = RecordKind::new(record.kind.to_owned())?;
        let read = ExactRecord::new(
            *pin,
            request.key().clone(),
            kind,
            record.content.as_bytes().to_vec(),
        )?;
        if request
            .expected()
            .is_some_and(|digest| digest != read.digest())
        {
            return Err(Refusal::DigestMismatch);
        }
        Ok(read)
    }

    fn lookup(
        &self,
        pin: &MemoryPin,
        request: &LookupRequest,
        cursor: Option<&LookupCursor<MemoryPin>>,
        _: &AtomicBool,
    ) -> Result<LookupPage<MemoryPin>, Refusal> {
        let records = rows(self.records(pin)?);
        match self.0 {
            Mutation::FuzzyAsSubstring if request.mode() == MatchMode::Fuzzy => {
                let emulated = LookupRequest::new(
                    request.field(),
                    MatchMode::Substring,
                    Order::Key,
                    request.needle().to_owned(),
                    request.limit(),
                )?;
                return scan(pin, &records, &emulated, None);
            }
            Mutation::IgnoreCursorBinding => {
                let rebound = cursor
                    .map(|cursor| LookupCursor::new(*pin, request.clone(), cursor.after().clone()));
                return scan(pin, &records, request, rebound.as_ref());
            }
            _ => admit_lookup(pin, request, cursor, CAPABILITIES)?,
        }
        let page = scan(pin, &records, request, cursor)?;
        if self.0 == Mutation::TruncateWithoutCursor && page.next().is_some() {
            let total = records.len();
            let claimed = Coverage::Measured {
                examined: total,
                total: total as u128,
            };
            return Ok(LookupPage::new(*pin, page.hits().to_vec(), claimed, None));
        }
        Ok(page)
    }

    fn admit_write(
        &mut self,
        _: &WriteRequest<MemoryPin>,
        _: &AtomicBool,
    ) -> Result<WriteOutcome<MemoryPin>, Refusal> {
        if self.0 == Mutation::WriteNotRefused {
            return Ok(WriteOutcome::Unknown);
        }
        Err(Refusal::Unavailable(Capability::Write))
    }
}

fn fixture() -> ConformanceFixture<MemoryPin> {
    ConformanceFixture {
        before: MemoryPin(0),
        after: MemoryPin(1),
        missing: MemoryPin(9),
    }
}

#[test]
fn faithful_reference_backend_passes_every_check() -> Result<(), Box<dyn std::error::Error>> {
    let passed = conformance::run(&mut Memory(Mutation::Faithful), &fixture())?;
    assert_eq!(passed, CHECKS);
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
    ] {
        let failure = conformance::run(&mut Memory(mutation), &fixture());
        assert!(
            matches!(&failure, Err(failure) if failure.check == check),
            "{mutation:?}: {failure:?}"
        );
    }
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
