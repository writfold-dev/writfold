//! Validated ledger history and conditional head selection (E05): the
//! typed adapter over `writfold_core::ledger`, whose Aeneas translation is
//! proved in `proofs/Writfold/Ledger.lean` (`history_check_spec`,
//! `ready_against_spec`, `Store.ready_spec`, `Store.append_refused`).
//!
//! Every decision is made by the proved core: this module only converts
//! nominal domain values to bytes and back and adapts the validator. Records
//! live inside their stream's selection, so Lean's "malformed selection" and
//! "unretained prefix" conjuncts hold by construction and their refusals are
//! never produced. Durable storage and the record codec are elsewhere.
use writfold_core::ledger as core;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_domain::{LedgerRecordId, LedgerSequence, StreamId};

/// One immutable record. The caller supplies the closed payload and its exact
/// historical premises; no digest or current lookup stands in for either.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record<P, H> {
    pub id: LedgerRecordId,
    pub stream: StreamId,
    pub sequence: LedgerSequence,
    pub predecessor: Option<LedgerRecordId>,
    pub profile: ReducerProfile,
    pub payload: P,
    pub historical: H,
    pub closure: Vec<BlobDigest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Head {
    pub stream: StreamId,
    pub sequence: LedgerSequence,
    pub record: LedgerRecordId,
    pub profile: ReducerProfile,
}

impl<P, H> Record<P, H> {
    pub fn head(&self) -> Head {
        Head {
            stream: self.stream.clone(),
            sequence: self.sequence,
            record: self.id.clone(),
            profile: self.profile,
        }
    }
}

/// Validates one record against the exact prior prefix (oldest first). It must
/// be a function of its arguments: origin, producer/approval bindings and
/// closed-action premises as recorded, never today's authority.
pub trait Validator<P, H> {
    fn validate(&self, prefix: &[Record<P, H>], record: &Record<P, H>) -> bool;
}

/// Why an append was refused. Each value names one `AppendReady` conjunct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    StaleHead,
    /// Never produced: selections are well formed by construction.
    MalformedSelection,
    UnlinkedPrefix,
    /// Never produced: selected records are retained by construction.
    UnretainedPrefix,
    ForeignStream,
    Sequence,
    Predecessor,
    DuplicateRecord,
    MissingMaterial,
    InvalidRecord,
    /// Selecting a new stream would overflow the inventory epoch.
    EpochOverflow,
}

fn refusal(r: core::Refusal) -> Refusal {
    match r {
        core::Refusal::StaleHead => Refusal::StaleHead,
        core::Refusal::UnlinkedPrefix => Refusal::UnlinkedPrefix,
        core::Refusal::ForeignStream => Refusal::ForeignStream,
        core::Refusal::Sequence => Refusal::Sequence,
        core::Refusal::Predecessor => Refusal::Predecessor,
        core::Refusal::DuplicateRecord => Refusal::DuplicateRecord,
        core::Refusal::MissingMaterial => Refusal::MissingMaterial,
        core::Refusal::InvalidRecord => Refusal::InvalidRecord,
        core::Refusal::EpochOverflow => Refusal::EpochOverflow,
    }
}

type Entry<P, H> = core::Entry<Record<P, H>>;

/// A sequence that does not fit `usize` can never be the next position.
fn position(sequence: LedgerSequence) -> usize {
    usize::try_from(sequence.get()).unwrap_or(0)
}

fn entry<P, H>(record: Record<P, H>) -> Entry<P, H> {
    core::Entry {
        id: record.id.as_str().as_bytes().to_vec(),
        stream: record.stream.as_str().as_bytes().to_vec(),
        sequence: position(record.sequence),
        predecessor: record
            .predecessor
            .as_ref()
            .map(|p| p.as_str().as_bytes().to_vec()),
        closure: record.closure.iter().map(|d| *d.as_digest()).collect(),
        payload: record,
    }
}

fn core_head(head: &Head) -> core::Head {
    core::Head {
        stream: head.stream.as_str().as_bytes().to_vec(),
        sequence: position(head.sequence),
        record: head.record.as_str().as_bytes().to_vec(),
    }
}

fn digests(blobs: &[BlobDigest]) -> Vec<[u8; 32]> {
    blobs.iter().map(|d| *d.as_digest()).collect()
}

/// The kernel validator, seen through the core's positional signature.
struct Adapter<'a, V>(&'a V);

impl<P: Clone, H: Clone, V: Validator<P, H>> core::Validator<Record<P, H>> for Adapter<'_, V> {
    fn validate(&self, history: &[Entry<P, H>], upto: usize, entry: &Entry<P, H>) -> bool {
        let prefix: Vec<Record<P, H>> = history
            .get(..upto)
            .unwrap_or(&[])
            .iter()
            .map(|e| e.payload.clone())
            .collect();
        self.0.validate(&prefix, &entry.payload)
    }
}

/// Lean `historyCheck`: every entry is on `stream`, numbered 1..n, linked to
/// its predecessor, and valid against exactly the prefix before it.
pub fn history_check<P: Clone, H: Clone>(
    validator: &impl Validator<P, H>,
    stream: &StreamId,
    entries: &[Record<P, H>],
) -> bool {
    let entries: Vec<Entry<P, H>> = entries.iter().cloned().map(entry).collect();
    core::history_check(&Adapter(validator), stream.as_str().as_bytes(), &entries)
}

/// Retained records and material plus one selected prefix per stream.
pub struct Store<P, H> {
    inner: core::Store<Record<P, H>>,
}

impl<P, H> Default for Store<P, H> {
    fn default() -> Self {
        Self {
            inner: core::Store::new(),
        }
    }
}

impl<P: Clone + Eq, H: Clone + Eq> Store<P, H> {
    fn selection(&self, stream: &StreamId) -> Option<&core::Selection<Record<P, H>>> {
        core::find(&self.inner.selections, stream.as_str().as_bytes())
            .and_then(|i| self.inner.selections.get(i))
    }
    pub fn current_head(&self, stream: &StreamId) -> Option<Head> {
        self.selection(stream)
            .and_then(|s| s.entries.last())
            .map(|e| e.payload.head())
    }
    pub fn current_prefix(&self, stream: &StreamId) -> Vec<Record<P, H>> {
        self.selection(stream)
            .map(|s| s.entries.iter().map(|e| e.payload.clone()).collect())
            .unwrap_or_default()
    }
    pub fn inventory_epoch(&self) -> u64 {
        u64::try_from(self.inner.inventory_epoch).unwrap_or(u64::MAX)
    }
    pub fn retains_record(&self, id: &LedgerRecordId) -> bool {
        core::retains_record(&self.inner.selections, id.as_str().as_bytes())
    }
    pub fn retains_material(&self, blob: &BlobDigest) -> bool {
        writfold_core::bytes::contains_digest(&self.inner.material, blob.as_digest())
    }

    /// Lean `AppendReady`, decided by the proved core in Lean's order.
    pub fn ready(
        &self,
        validator: &impl Validator<P, H>,
        stream: &StreamId,
        expected: Option<&Head>,
        record: &Record<P, H>,
        available: &[BlobDigest],
    ) -> Result<(), Refusal> {
        self.inner
            .ready(
                &Adapter(validator),
                stream.as_str().as_bytes(),
                &expected.map(core_head),
                &entry(record.clone()),
                &digests(available),
            )
            .map_err(refusal)
    }

    /// Lean `append`: retain, then select. On refusal the store is unchanged,
    /// including material a rejected caller staged.
    pub fn append(
        &mut self,
        validator: &impl Validator<P, H>,
        stream: &StreamId,
        expected: Option<&Head>,
        record: Record<P, H>,
        available: &[BlobDigest],
    ) -> Result<Head, Refusal> {
        let head = record.head();
        self.inner
            .append(
                &Adapter(validator),
                stream.as_str().as_bytes(),
                &expected.map(core_head),
                entry(record),
                &digests(available),
            )
            .map_err(refusal)?;
        Ok(head)
    }
}
