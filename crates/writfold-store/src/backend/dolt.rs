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
//! `follow` waits for Omnigraph. Writes are `Unavailable`: no typed Rust Dolt
//! writer with conditional heads exists yet (DINT02 has not run).
use super::BackendKind;
use super::{
    Capability, ExactRead, ExactRecord, LookupCursor, LookupPage, LookupRequest, RecordKey,
    RecordKind, Refusal, ScanRecord, StorageBackend, WriteOutcome, WriteRequest, admit_lookup,
    scan,
};
use crate::dolt::{Error, ReadBudgets, read_snapshot};
use std::sync::atomic::AtomicBool;
use writfold_domain::dolt::{DoltReadPin, DoltSnapshot};
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable};

const CAPABILITIES: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
];

/// One admitted database, one admitted executable and fixed read budgets.
#[derive(Debug)]
pub struct DoltBackend<'a> {
    database: &'a DoltDatabase,
    executable: &'a DoltExecutable<'a>,
    budgets: ReadBudgets,
}

impl<'a> DoltBackend<'a> {
    pub fn new(
        database: &'a DoltDatabase,
        executable: &'a DoltExecutable<'a>,
        budgets: ReadBudgets,
    ) -> Self {
        Self {
            database,
            executable,
            budgets,
        }
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

    fn kind(&self) -> BackendKind {
        BackendKind::Dolt
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
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
        admit_lookup(pin, request, cursor, CAPABILITIES)?;
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
        _request: &WriteRequest<DoltReadPin>,
        _cancelled: &AtomicBool,
    ) -> Result<WriteOutcome<DoltReadPin>, Refusal> {
        Err(Refusal::Unavailable(Capability::Write))
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
