//! In-memory reference backend for calibrating the conformance suite, with
//! deliberately broken variants. Writers share one realm state; each writer
//! instance holds the realm's single writer slot until it is dropped.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use writfold_domain::citation::Coverage;
use writfold_store::backend::conformance::{AFTER, BEFORE, FixtureRecord};
use writfold_store::backend::{
    BackendKind, Capability, ExactRead, ExactRecord, LookupCursor, LookupPage, LookupRequest,
    MatchMode, Order, RecordKind, Refusal, ScanRecord, StorageBackend, WriteObservation,
    WriteOutcome, WriteRequest, admit_lookup, scan,
};

/// `realm` 0 is the realm writers own; `snapshot` indexes its commits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryPin {
    pub realm: u8,
    pub snapshot: u32,
}

pub const fn pin(snapshot: u32) -> MemoryPin {
    MemoryPin { realm: 0, snapshot }
}

/// Memory write authority: names the expected-old head it authorizes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grant(pub MemoryPin);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mutation {
    Faithful,
    IgnorePin,
    LatestOnMissing,
    FuzzyAsSubstring,
    TruncateWithoutCursor,
    IgnoreCursorBinding,
    WriteNotRefused,
    WriteWithoutAuthority,
    CommitUnobservable,
    IgnoreExpectedOld,
    NoLeaseRacyCompareAndSwap,
    ForeignRealmAccepted,
    CancelledAfterCommit,
}

type Rows = Vec<(String, String, Vec<u8>)>;

struct Commit {
    rows: Rows,
    parent: Option<u32>,
    write: Option<WriteRequest<MemoryPin>>,
}

pub struct Realm {
    commits: Vec<Commit>,
    head: u32,
    owned: bool,
}

pub type Shared = Arc<Mutex<Realm>>;

fn fixture_rows(records: &[FixtureRecord]) -> Rows {
    records
        .iter()
        .map(|r| {
            (
                r.key.to_owned(),
                r.kind.to_owned(),
                r.content.as_bytes().to_vec(),
            )
        })
        .collect()
}

/// A realm whose commits 0 and 1 are `BEFORE` and `AFTER`, head at `AFTER`.
pub fn realm() -> Shared {
    let commit = |rows, parent| Commit {
        rows,
        parent,
        write: None,
    };
    Arc::new(Mutex::new(Realm {
        commits: vec![
            commit(fixture_rows(BEFORE), None),
            commit(fixture_rows(AFTER), Some(0)),
        ],
        head: 1,
        owned: false,
    }))
}

pub struct Memory {
    mutation: Mutation,
    shared: Shared,
    writer: bool,
}

const CAPABILITIES: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
];
const WRITER: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
    Capability::Write,
];

impl Memory {
    pub fn reader(mutation: Mutation, shared: Shared) -> Self {
        Self {
            mutation,
            shared,
            writer: false,
        }
    }

    /// Takes the realm's single writer slot; a held slot refuses `NotOwner`.
    pub fn writer(mutation: Mutation, shared: Shared) -> Result<Self, Refusal> {
        let mut realm = lock(&shared)?;
        if realm.owned && mutation != Mutation::NoLeaseRacyCompareAndSwap {
            return Err(Refusal::NotOwner("writer slot held"));
        }
        realm.owned = true;
        drop(realm);
        Ok(Self {
            mutation,
            shared,
            writer: true,
        })
    }

    fn rows<'r>(&self, realm: &'r Realm, pin: &MemoryPin) -> Result<&'r Rows, Refusal> {
        let head = &realm.commits[realm.head as usize].rows;
        if self.mutation == Mutation::IgnorePin {
            return Ok(head);
        }
        match realm.commits.get(pin.snapshot as usize) {
            Some(commit) if pin.realm == 0 => Ok(&commit.rows),
            _ if self.mutation == Mutation::LatestOnMissing => Ok(head),
            _ => Err(Refusal::Pin("no such snapshot".into())),
        }
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        if self.writer
            && let Ok(mut realm) = self.shared.lock()
        {
            realm.owned = false;
        }
    }
}

fn lock(shared: &Shared) -> Result<MutexGuard<'_, Realm>, Refusal> {
    shared
        .lock()
        .map_err(|_| Refusal::Backend("poisoned".into()))
}

fn scan_rows(rows: &Rows) -> Vec<ScanRecord<'_>> {
    rows.iter()
        .map(|(key, kind, content)| ScanRecord { key, kind, content })
        .collect()
}

impl StorageBackend for Memory {
    type Pin = MemoryPin;
    type WriteAuthority = Grant;

    fn kind(&self) -> BackendKind {
        BackendKind::Dolt
    }

    fn capabilities(&self) -> &'static [Capability] {
        if self.writer { WRITER } else { CAPABILITIES }
    }

    fn read_exact(
        &self,
        pin: &MemoryPin,
        request: &ExactRead,
        _: &AtomicBool,
    ) -> Result<ExactRecord<MemoryPin>, Refusal> {
        let realm = lock(&self.shared)?;
        let (_, kind, content) = self
            .rows(&realm, pin)?
            .iter()
            .find(|(key, _, _)| key == request.key().as_str())
            .ok_or(Refusal::NotFound)?;
        let kind = RecordKind::new(kind.clone())?;
        let read = ExactRecord::new(*pin, request.key().clone(), kind, content.clone())?;
        if request.expected().is_some_and(|d| d != read.digest()) {
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
        let realm = lock(&self.shared)?;
        let records = scan_rows(self.rows(&realm, pin)?);
        match self.mutation {
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
            _ => admit_lookup(pin, request, cursor, self.capabilities())?,
        }
        let page = scan(pin, &records, request, cursor)?;
        if self.mutation == Mutation::TruncateWithoutCursor && page.next().is_some() {
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
        request: &WriteRequest<MemoryPin>,
        authority: Option<&Grant>,
        cancelled: &AtomicBool,
    ) -> Result<WriteOutcome<MemoryPin>, Refusal> {
        if !self.writer {
            return match self.mutation {
                Mutation::WriteNotRefused => Ok(WriteOutcome::Unknown),
                _ => Err(Refusal::Unavailable(Capability::Write)),
            };
        }
        if request.expected_old.realm != 0 && self.mutation != Mutation::ForeignRealmAccepted {
            return Err(Refusal::NotOwner("foreign realm"));
        }
        if authority != Some(&Grant(request.expected_old))
            && self.mutation != Mutation::WriteWithoutAuthority
        {
            return Err(Refusal::Unauthorized("no grant for this expected-old head"));
        }
        if cancelled.load(Ordering::SeqCst) {
            return Err(Refusal::Cancelled);
        }
        let new = match self.commit(request)? {
            Ok(new) => new,
            Err(observed) => return Ok(WriteOutcome::Stale { observed }),
        };
        // The commit is durable; its acknowledgement can still be lost.
        for _ in 0..30 {
            if cancelled.load(Ordering::SeqCst) {
                return match self.mutation {
                    Mutation::CancelledAfterCommit => Err(Refusal::Cancelled),
                    _ => Ok(WriteOutcome::Unknown),
                };
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(WriteOutcome::Committed { new })
    }

    fn observe(
        &self,
        request: &WriteRequest<MemoryPin>,
        _: &AtomicBool,
    ) -> Result<WriteObservation<MemoryPin>, Refusal> {
        let realm = lock(&self.shared)?;
        let mut applied = None;
        let mut cursor = Some(realm.head);
        while let Some(index) = cursor {
            let commit = &realm.commits[index as usize];
            if commit.write.as_ref() == Some(request) {
                applied = Some(pin(index));
            }
            cursor = commit.parent;
        }
        if self.mutation == Mutation::CommitUnobservable {
            applied = None;
        }
        Ok(WriteObservation {
            head: pin(realm.head),
            applied,
        })
    }
}

impl Memory {
    /// `Ok(new)` or, when the head is not the expected-old one, `Err(observed)`.
    fn commit(
        &self,
        request: &WriteRequest<MemoryPin>,
    ) -> Result<Result<MemoryPin, MemoryPin>, Refusal> {
        let mut realm = lock(&self.shared)?;
        let current = realm.head;
        if current != request.expected_old.snapshot && self.mutation != Mutation::IgnoreExpectedOld
        {
            return Ok(Err(pin(current)));
        }
        if self.mutation == Mutation::NoLeaseRacyCompareAndSwap {
            drop(realm);
            std::thread::sleep(Duration::from_millis(50));
            realm = lock(&self.shared)?;
        }
        let parent = realm.head;
        let mut rows = realm.commits[parent as usize].rows.clone();
        for change in &request.changes {
            rows.retain(|(key, _, _)| key != change.key.as_str());
            if let Some(content) = &change.content {
                let entry = (
                    change.key.as_str().to_owned(),
                    change.kind.as_str().to_owned(),
                    content.clone(),
                );
                rows.push(entry);
            }
        }
        rows.sort();
        realm.commits.push(Commit {
            rows,
            parent: Some(parent),
            write: Some(request.clone()),
        });
        let new =
            u32::try_from(realm.commits.len() - 1).map_err(|_| Refusal::Backend("full".into()))?;
        realm.head = new;
        Ok(Ok(pin(new)))
    }
}
