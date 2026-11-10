//! Dolt adapter for the storage boundary: the interim primary backend.
//!
//! Every call reads one complete, fully checked projection through the existing
//! DINT01 exact reader (`crate::dolt::read_snapshot`): exact pin admission,
//! retention-root checks before and after, bounded rows and bytes, one deadline
//! and cancellation. The snapshot is complete or the call refuses, so lookup
//! coverage is measured over the whole pinned `wf_nodes` projection.
//!
//! A record is one `wf_nodes` row: key = `id`, kind = `kind`, content = the UTF-8
//! bytes of `label`. `home_path` and `wf_edges` are not exposed in B0; graph
//! `follow` waits for Omnigraph. A writer instance (`DoltBackend::writer`)
//! holds the realm's exclusive writer lease and adds the conditional commit in
//! `dolt/write.rs`; a reader instance refuses writes as `Unavailable`.
mod origin;
mod ownership;
mod retain;
mod write;

pub use ownership::{DoltWriteApproval, WriterId};

use super::BackendKind;
use super::{
    Capability, ExactRead, ExactRecord, LookupCursor, LookupPage, LookupRequest, Origin, RecordKey,
    RecordKind, Refusal, ScanRecord, StorageBackend, WriteObservation, WriteOutcome, WriteRequest,
    admit_lookup, scan,
};
use crate::dolt::{Error, ReadBudgets, read_snapshot};
use std::sync::atomic::AtomicBool;
use writfold_domain::ByteCount;
use writfold_domain::dolt::{DoltReadPin, DoltSnapshot};
use writfold_runtime::dolt::{DoltDatabase, DoltError, DoltExecutable, DoltWriterSlot};
use writfold_runtime::fs::FsError;
use writfold_wire::dolt_operation::DoltTarget;

const CAPABILITIES: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
];
const WRITER_CAPABILITIES: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
    Capability::Write,
];
const CLAIM_BYTES: u64 = 4096;

/// One admitted database, one admitted executable and fixed read budgets;
/// a writer instance also holds the realm's writer lease.
#[derive(Debug)]
pub struct DoltBackend<'a> {
    database: &'a DoltDatabase,
    executable: &'a DoltExecutable<'a>,
    budgets: ReadBudgets,
    writer: Option<write::Writer<'a>>,
}

impl<'a> DoltBackend<'a> {
    /// A reader: exact reads, lookups and `observe`; writes are `Unavailable`.
    pub fn new(
        database: &'a DoltDatabase,
        executable: &'a DoltExecutable<'a>,
        budgets: ReadBudgets,
    ) -> Self {
        Self {
            database,
            executable,
            budgets,
            writer: None,
        }
    }

    /// The realm's single writer for `target`. Takes the exclusive writer lease
    /// in `slot`'s database without waiting and records `owner` as its holder;
    /// another live writer refuses as `NotOwner`. The lease lasts until drop.
    pub fn writer(
        slot: &'a DoltWriterSlot<'a>,
        executable: &'a DoltExecutable<'a>,
        budgets: ReadBudgets,
        owner: WriterId,
        target: DoltTarget,
    ) -> Result<Self, Refusal> {
        if executable.generation() != target.producer() {
            return Err(Refusal::Pin(
                "writer target producer differs from the admitted executable".into(),
            ));
        }
        let limit = ByteCount::new(CLAIM_BYTES)
            .map_err(|_| Refusal::InvalidRequest("claim byte budget"))?;
        let claim = slot
            .claim(&ownership::claim_record(&owner, &target), limit)
            .map_err(|error| match error {
                DoltError::Filesystem(FsError::LockBusy) => {
                    Refusal::NotOwner("another writer holds this realm's writer lease")
                }
                other => Refusal::Backend(other.to_string()),
            })?;
        Ok(Self {
            database: slot.database(),
            executable,
            budgets,
            writer: Some(write::Writer {
                claim,
                owner,
                target,
            }),
        })
    }

    fn snapshot(&self, pin: &DoltReadPin, cancelled: &AtomicBool) -> Result<DoltSnapshot, Refusal> {
        let snapshot = read_snapshot(
            self.database,
            self.executable,
            pin.clone(),
            self.budgets,
            cancelled,
        )
        .map_err(refusal)?;
        if snapshot.pin() != pin {
            return Err(Refusal::Pin("snapshot pin differs from the request".into()));
        }
        Ok(snapshot)
    }
}

impl StorageBackend for DoltBackend<'_> {
    type Pin = DoltReadPin;
    type WriteAuthority = DoltWriteApproval;

    fn kind(&self) -> BackendKind {
        BackendKind::Dolt
    }

    fn capabilities(&self) -> &'static [Capability] {
        if self.writer.is_some() {
            WRITER_CAPABILITIES
        } else {
            CAPABILITIES
        }
    }

    fn read_exact(
        &self,
        pin: &DoltReadPin,
        request: &ExactRead,
        cancelled: &AtomicBool,
    ) -> Result<ExactRecord<DoltReadPin>, Refusal> {
        let snapshot = self.snapshot(pin, cancelled)?;
        let wanted = request.key().as_str().as_bytes();
        let index = snapshot
            .nodes()
            .binary_search_by(|node| node.id().as_str().as_bytes().cmp(wanted))
            .map_err(|_| Refusal::NotFound)?;
        let node = snapshot
            .nodes()
            .get(index)
            .ok_or_else(|| Refusal::Backend("node index outside snapshot".into()))?;
        let record = ExactRecord::new(
            snapshot.pin().clone(),
            RecordKey::new(node.id().as_str().to_owned())
                .map_err(|_| Refusal::Backend("Dolt node id outside the boundary".into()))?,
            RecordKind::new(node.kind().as_str().to_owned())
                .map_err(|_| Refusal::Backend("Dolt node kind outside the boundary".into()))?,
            node.label().as_bytes().to_vec(),
        )?;
        if request
            .expected()
            .is_some_and(|expected| expected != record.digest())
        {
            return Err(Refusal::DigestMismatch);
        }
        Ok(record)
    }

    fn lookup(
        &self,
        pin: &DoltReadPin,
        request: &LookupRequest,
        cursor: Option<&LookupCursor<DoltReadPin>>,
        cancelled: &AtomicBool,
    ) -> Result<LookupPage<DoltReadPin>, Refusal> {
        admit_lookup(pin, request, cursor, self.capabilities())?;
        let snapshot = self.snapshot(pin, cancelled)?;
        let records: Vec<ScanRecord<'_>> = snapshot
            .nodes()
            .iter()
            .map(|node| ScanRecord {
                key: node.id().as_str(),
                kind: node.kind().as_str(),
                content: node.label().as_bytes(),
            })
            .collect();
        scan(snapshot.pin(), &records, request, cursor)
    }

    fn admit_write(
        &mut self,
        request: &WriteRequest<DoltReadPin>,
        authority: Option<&DoltWriteApproval>,
        cancelled: &AtomicBool,
    ) -> Result<WriteOutcome<DoltReadPin>, Refusal> {
        write::admit(self, request, authority, cancelled)
    }

    fn observe(
        &self,
        request: &WriteRequest<DoltReadPin>,
        cancelled: &AtomicBool,
    ) -> Result<WriteObservation<DoltReadPin>, Refusal> {
        write::observe(self, request, cancelled)
    }

    fn origin(&self, cancelled: &AtomicBool) -> Result<Origin<DoltReadPin>, Refusal> {
        origin::origin(self, cancelled)
    }

    fn retain(&mut self, pin: &DoltReadPin, cancelled: &AtomicBool) -> Result<(), Refusal> {
        retain::retain(self, pin, cancelled)
    }
}

fn refusal(error: Error) -> Refusal {
    match error {
        Error::Cancelled => Refusal::Cancelled,
        Error::Deadline => Refusal::Deadline,
        Error::Pin(_) | Error::Schema | Error::Query | Error::RetentionRoot => {
            Refusal::Pin(error.to_string())
        }
        other => Refusal::Backend(other.to_string()),
    }
}
