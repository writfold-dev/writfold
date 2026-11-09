//! Validated ledger history and conditional head selection, mirroring Lean
//! `historyCheck`, `AppendReady` and `append` (E05).
//!
//! Records live only inside their stream's selection: Lean's `append` always
//! retains and selects together and a refusal changes nothing, so the retained
//! records are exactly the union of the selections. That makes Lean's
//! "malformed selection" and "unretained prefix" conjuncts true by
//! construction; every other conjunct is checked here, in Lean's order.
use crate::bytes::{contains_digest, same, same_option};
use alloc::vec::Vec;

pub struct Entry<P> {
    pub id: Vec<u8>,
    pub stream: Vec<u8>,
    pub sequence: usize,
    pub predecessor: Option<Vec<u8>>,
    pub payload: P,
    pub closure: Vec<[u8; 32]>,
}

pub struct Head {
    pub stream: Vec<u8>,
    pub sequence: usize,
    pub record: Vec<u8>,
}

/// Judges `entry` against exactly `history[..upto]` (oldest first). It must
/// be a function of its arguments: never today's authority or a clock.
pub trait Validator<P> {
    fn validate(&self, history: &[Entry<P>], upto: usize, entry: &Entry<P>) -> bool;
}

pub enum Refusal {
    StaleHead,
    UnlinkedPrefix,
    ForeignStream,
    Sequence,
    Predecessor,
    DuplicateRecord,
    MissingMaterial,
    InvalidRecord,
    /// Selecting a new stream would overflow the inventory epoch.
    EpochOverflow,
}

pub struct Selection<P> {
    pub stream: Vec<u8>,
    pub entries: Vec<Entry<P>>,
}

pub struct Store<P> {
    pub selections: Vec<Selection<P>>,
    pub material: Vec<[u8; 32]>,
    pub inventory_epoch: usize,
}

/// Whether `entries[position]` is on `stream`, numbered `position + 1`, linked
/// to its predecessor, and valid against the entries before it.
pub fn entry_ok<P, V: Validator<P>>(
    validator: &V,
    stream: &[u8],
    entries: &[Entry<P>],
    position: usize,
) -> bool {
    let entry = &entries[position];
    if !same(&entry.stream, stream) || entry.sequence != position + 1 {
        return false;
    }
    let linked = if position == 0 {
        same_option(&entry.predecessor, &None)
    } else {
        match &entry.predecessor {
            None => false,
            Some(predecessor) => same(predecessor, &entries[position - 1].id),
        }
    };
    linked && validator.validate(entries, position, entry)
}

/// Lean `historyCheck`: every entry passes `entry_ok`.
pub fn history_check<P, V: Validator<P>>(
    validator: &V,
    stream: &[u8],
    entries: &[Entry<P>],
) -> bool {
    let mut position = 0;
    while position < entries.len() {
        if !entry_ok(validator, stream, entries, position) {
            return false;
        }
        position += 1;
    }
    true
}

/// The index of `stream`'s selection, if it has one.
pub fn find<P>(selections: &[Selection<P>], stream: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < selections.len() {
        if same(&selections[i].stream, stream) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Whether any entry of one selection has this id.
pub fn entries_contain<P>(entries: &[Entry<P>], id: &[u8]) -> bool {
    let mut j = 0;
    while j < entries.len() {
        if same(&entries[j].id, id) {
            return true;
        }
        j += 1;
    }
    false
}

/// Whether any selected record anywhere has this id. (One loop per function:
/// Aeneas does not translate a `return` inside nested loops.)
pub fn retains_record<P>(selections: &[Selection<P>], id: &[u8]) -> bool {
    let mut i = 0;
    while i < selections.len() {
        if entries_contain(&selections[i].entries, id) {
            return true;
        }
        i += 1;
    }
    false
}

/// Whether `expected` names exactly the last entry of `entries` (or both are empty).
pub fn head_matches<P>(entries: &[Entry<P>], expected: &Option<Head>) -> bool {
    match expected {
        None => entries.is_empty(),
        Some(head) => {
            if entries.is_empty() {
                false
            } else {
                let last = &entries[entries.len() - 1];
                same(&head.stream, &last.stream)
                    && head.sequence == last.sequence
                    && same(&head.record, &last.id)
            }
        }
    }
}

/// Every closure digest is available or already retained.
pub fn material_ok(closure: &[[u8; 32]], available: &[[u8; 32]], material: &[[u8; 32]]) -> bool {
    let mut i = 0;
    while i < closure.len() {
        if !contains_digest(available, &closure[i]) && !contains_digest(material, &closure[i]) {
            return false;
        }
        i += 1;
    }
    true
}

/// The predecessor `record` must name: the last entry's id, or none.
pub fn predecessor_ok<P>(entries: &[Entry<P>], record: &Entry<P>) -> bool {
    if entries.is_empty() {
        same_option(&record.predecessor, &None)
    } else {
        match &record.predecessor {
            None => false,
            Some(predecessor) => same(predecessor, &entries[entries.len() - 1].id),
        }
    }
}

/// Lean `AppendReady` against the selected prefix `entries`, in Lean's order;
/// the first failing conjunct names the refusal.
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is one AppendReady input"
)]
pub fn ready_against<P, V: Validator<P>>(
    validator: &V,
    selections: &[Selection<P>],
    material: &[[u8; 32]],
    entries: &[Entry<P>],
    stream: &[u8],
    expected: &Option<Head>,
    record: &Entry<P>,
    available: &[[u8; 32]],
) -> Result<(), Refusal> {
    if !head_matches(entries, expected) {
        return Err(Refusal::StaleHead);
    }
    if !history_check(validator, stream, entries) {
        return Err(Refusal::UnlinkedPrefix);
    }
    if !same(&record.stream, stream) {
        return Err(Refusal::ForeignStream);
    }
    // `sequence - 1 == len`, written so it cannot overflow even for a
    // selection of `usize::MAX` entries (the proof found `len + 1` could).
    if record.sequence == 0 || record.sequence - 1 != entries.len() {
        return Err(Refusal::Sequence);
    }
    if !predecessor_ok(entries, record) {
        return Err(Refusal::Predecessor);
    }
    if retains_record(selections, &record.id) {
        return Err(Refusal::DuplicateRecord);
    }
    if !material_ok(&record.closure, available, material) {
        return Err(Refusal::MissingMaterial);
    }
    if !validator.validate(entries, entries.len(), record) {
        return Err(Refusal::InvalidRecord);
    }
    Ok(())
}

impl<P> Store<P> {
    pub fn new() -> Self {
        Store {
            selections: Vec::new(),
            material: Vec::new(),
            inventory_epoch: 0,
        }
    }

    /// Lean `AppendReady` for `stream`.
    pub fn ready<V: Validator<P>>(
        &self,
        validator: &V,
        stream: &[u8],
        expected: &Option<Head>,
        record: &Entry<P>,
        available: &[[u8; 32]],
    ) -> Result<(), Refusal> {
        let empty: Vec<Entry<P>> = Vec::new();
        let entries = match find(&self.selections, stream) {
            Some(i) => &self.selections[i].entries,
            None => &empty,
        };
        ready_against(
            validator,
            &self.selections,
            &self.material,
            entries,
            stream,
            expected,
            record,
            available,
        )
    }

    /// Lean `append`: on success the record is retained and selected and the
    /// available material retained; on refusal nothing changes.
    pub fn append<V: Validator<P>>(
        &mut self,
        validator: &V,
        stream: &[u8],
        expected: &Option<Head>,
        record: Entry<P>,
        available: &[[u8; 32]],
    ) -> Result<(), Refusal> {
        self.ready(validator, stream, expected, &record, available)?;
        let fresh = find(&self.selections, stream).is_none();
        if fresh && self.inventory_epoch == usize::MAX {
            return Err(Refusal::EpochOverflow);
        }
        let mut i = 0;
        while i < available.len() {
            self.material.push(available[i]);
            i += 1;
        }
        match find(&self.selections, stream) {
            Some(index) => self.selections[index].entries.push(record),
            None => {
                let mut entries = Vec::new();
                entries.push(record);
                self.selections.push(Selection {
                    stream: stream.to_vec(),
                    entries,
                });
                self.inventory_epoch += 1;
            }
        }
        Ok(())
    }
}
