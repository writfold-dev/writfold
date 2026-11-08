//! Validated ledger history and conditional head selection, refining
//! `Engineering/LedgerHistory.lean` and `Engineering/LedgerAppend.lean` (E05).
//!
//! Pure and IO-free. `append` re-checks the complete selected prefix (linkage,
//! retention and validator replay) before every append, and changes nothing on
//! refusal. Durable atomic compare-and-select, authenticated historical inputs
//! and the canonical record codec are storage/wire obligations outside this
//! module; this is tested Rust, not a proved refinement of the Lean algebra.
use std::collections::{BTreeMap, BTreeSet};
use writfold_domain::engineering::BlobDigest;
use writfold_domain::engineering::ledger::ReducerProfile;
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
    MalformedSelection,
    UnlinkedPrefix,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Selection<P, H> {
    head: Head,
    /// Oldest first; Lean lists the same prefix newest first.
    entries: Vec<Record<P, H>>,
}

/// Retained records and material plus one selected prefix per stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Store<P, H> {
    records: Vec<Record<P, H>>,
    material: BTreeSet<BlobDigest>,
    selections: BTreeMap<StreamId, Selection<P, H>>,
    inventory_epoch: u64,
}

impl<P, H> Default for Store<P, H> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            material: BTreeSet::new(),
            selections: BTreeMap::new(),
            inventory_epoch: 0,
        }
    }
}

/// Lean `historyCheck`: every entry is on `stream`, numbered 1..n, linked to
/// its predecessor, and valid against exactly the prefix before it.
pub fn history_check<P, H>(
    validator: &impl Validator<P, H>,
    stream: &StreamId,
    entries: &[Record<P, H>],
) -> bool {
    let mut predecessor: Option<&LedgerRecordId> = None;
    for (position, record) in entries.iter().enumerate() {
        let expected = u64::try_from(position).ok().and_then(|p| p.checked_add(1));
        if &record.stream != stream
            || Some(record.sequence.get()) != expected
            || record.predecessor.as_ref() != predecessor
            || !validator.validate(entries.get(..position).unwrap_or(&[]), record)
        {
            return false;
        }
        predecessor = Some(&record.id);
    }
    true
}

impl<P: Clone + Eq, H: Clone + Eq> Store<P, H> {
    pub fn current_head(&self, stream: &StreamId) -> Option<&Head> {
        self.selections.get(stream).map(|s| &s.head)
    }
    pub fn current_prefix(&self, stream: &StreamId) -> &[Record<P, H>] {
        self.selections
            .get(stream)
            .map_or(&[], |s| s.entries.as_slice())
    }
    pub fn inventory_epoch(&self) -> u64 {
        self.inventory_epoch
    }
    pub fn retains_record(&self, id: &LedgerRecordId) -> bool {
        self.records.iter().any(|r| &r.id == id)
    }
    pub fn retains_material(&self, blob: &BlobDigest) -> bool {
        self.material.contains(blob)
    }

    fn retained(&self, entries: &[Record<P, H>]) -> bool {
        entries.iter().all(|r| {
            self.records.iter().any(|old| old == r)
                && r.closure.iter().all(|o| self.material.contains(o))
        })
    }

    /// Lean `AppendReady`, checked in order; the first failing conjunct names the refusal.
    pub fn ready(
        &self,
        validator: &impl Validator<P, H>,
        stream: &StreamId,
        expected: Option<&Head>,
        record: &Record<P, H>,
        available: &[BlobDigest],
    ) -> Result<(), Refusal> {
        let prefix = self.current_prefix(stream);
        let head = self.current_head(stream);
        let rules = [
            (expected == head, Refusal::StaleHead),
            (
                prefix.last().map(Record::head).as_ref() == head,
                Refusal::MalformedSelection,
            ),
            (
                history_check(validator, stream, prefix),
                Refusal::UnlinkedPrefix,
            ),
            (self.retained(prefix), Refusal::UnretainedPrefix),
            (&record.stream == stream, Refusal::ForeignStream),
            (
                Some(record.sequence.get())
                    == u64::try_from(prefix.len())
                        .ok()
                        .and_then(|n| n.checked_add(1)),
                Refusal::Sequence,
            ),
            (
                record.predecessor.as_ref() == prefix.last().map(|r| &r.id),
                Refusal::Predecessor,
            ),
            (!self.retains_record(&record.id), Refusal::DuplicateRecord),
            (
                record
                    .closure
                    .iter()
                    .all(|o| available.contains(o) || self.material.contains(o)),
                Refusal::MissingMaterial,
            ),
            (validator.validate(prefix, record), Refusal::InvalidRecord),
        ];
        match rules.into_iter().find(|(ok, _)| !ok) {
            Some((_, refusal)) => Err(refusal),
            None => Ok(()),
        }
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
        self.ready(validator, stream, expected, &record, available)?;
        let epoch = if self.selections.contains_key(stream) {
            self.inventory_epoch
        } else {
            self.inventory_epoch
                .checked_add(1)
                .ok_or(Refusal::EpochOverflow)?
        };
        let mut entries = self.current_prefix(stream).to_vec();
        let head = record.head();
        self.records.push(record.clone());
        self.material.extend(available.iter().copied());
        entries.push(record);
        self.inventory_epoch = epoch;
        self.selections.insert(
            stream.clone(),
            Selection {
                head: head.clone(),
                entries,
            },
        );
        Ok(head)
    }
}
