//! Durable validated ledger (`writfold-local-ledger/1`): records retained as
//! commits in the writer repository, stream heads selected by Git's atomic
//! expected-old update, record material retained in the object store.
//!
//! Every load replays the complete selected stream through the kernel's
//! `history_check` and requires every closure object to be retained, so a
//! tampered, reordered or truncated history refuses instead of being trusted.
//! Every append first rebuilds the kernel `Store` from that validated history
//! and asks the kernel's `ready` whether the record may follow it.
use std::sync::atomic::AtomicBool;
use writfold_domain::StreamId;
use writfold_domain::engineering::BlobDigest;
use writfold_kernel::ledger::{Head, Record, Refusal, Store, Validator, history_check};
use writfold_runtime::git::{GitBudgets, GitError, GitExecutable, GitWriteError, GitWriterLease};
use writfold_store::objects::{ObjectError, ObjectStore};
use writfold_wire::Limits;
use writfold_wire::engineering::ledger_record::{self as codec, Part};

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error(transparent)]
    Objects(#[from] ObjectError),
    #[error(transparent)]
    Codec(#[from] codec::Error),
    #[error("retained stream history does not validate")]
    InvalidHistory,
    #[error("retained stream exceeds {0} records")]
    TooLong(usize),
    #[error("record material is not retained")]
    UnretainedMaterial,
    #[error("append refused: {0:?}")]
    Refused(Refusal),
    #[error("the stream head moved; reload and decide again")]
    Conflict,
    #[error("the head update may have happened; reconcile before any retry: {0}")]
    Unknown(GitError),
}

/// Everything one ledger operation needs, borrowed for its duration.
pub struct Ledger<'a, V> {
    pub lease: &'a GitWriterLease<'a>,
    pub executable: &'a GitExecutable<'a>,
    pub objects: &'a ObjectStore,
    pub validator: &'a V,
    pub limits: Limits,
    pub budgets: GitBudgets,
    pub max_records: usize,
    pub cancelled: &'a AtomicBool,
}

/// A validated stream, oldest record first, and the commit selected as head.
#[derive(Debug)]
pub struct Loaded<P, H> {
    pub head: Option<writfold_domain::GitObjectId>,
    pub records: Vec<Record<P, H>>,
}

impl<V> Ledger<'_, V> {
    pub fn load<P, H>(&self, stream: &StreamId) -> Result<Loaded<P, H>, LedgerError>
    where
        P: Part + Clone + Eq,
        H: Part + Clone + Eq,
        V: Validator<P, H>,
    {
        let head = self
            .lease
            .ledger_head(self.executable, stream, self.budgets, self.cancelled)?;
        let mut records = Vec::new();
        let mut next = head.clone();
        while let Some(commit) = next {
            if records.len() == self.max_records {
                return Err(LedgerError::TooLong(self.max_records));
            }
            let read =
                self.lease
                    .ledger_commit(self.executable, &commit, self.budgets, self.cancelled)?;
            records.push(codec::decode::<P, H>(&read.record, self.limits)?);
            next = read.parent;
        }
        records.reverse();
        if !history_check(self.validator, stream, &records) {
            return Err(LedgerError::InvalidHistory);
        }
        for record in &records {
            if !self.retained(&record.closure)? {
                return Err(LedgerError::UnretainedMaterial);
            }
        }
        Ok(Loaded { head, records })
    }

    /// Append `record` if the kernel admits it after `expected`, then select it
    /// with one atomic expected-old head update.
    pub fn append<P, H>(
        &self,
        stream: &StreamId,
        expected: Option<&Head>,
        record: Record<P, H>,
    ) -> Result<Head, LedgerError>
    where
        P: Part + Clone + Eq,
        H: Part + Clone + Eq,
        V: Validator<P, H>,
    {
        let loaded = self.load::<P, H>(stream)?;
        let mut store = Store::default();
        for old in &loaded.records {
            let head = store.current_head(stream);
            store
                .append(
                    self.validator,
                    stream,
                    head.as_ref(),
                    old.clone(),
                    &old.closure,
                )
                .map_err(|_| LedgerError::InvalidHistory)?;
        }
        let available: Vec<BlobDigest> = if self.retained(&record.closure)? {
            record.closure.clone()
        } else {
            Vec::new()
        };
        store
            .ready(self.validator, stream, expected, &record, &available)
            .map_err(LedgerError::Refused)?;
        let bytes = codec::encode(&record, self.limits)?;
        match self.lease.ledger_select(
            self.executable,
            stream,
            loaded.head.as_ref(),
            &bytes,
            self.budgets,
            self.cancelled,
        ) {
            Ok(_) => Ok(record.head()),
            Err(GitWriteError::Conflict { .. }) => Err(LedgerError::Conflict),
            Err(GitWriteError::Unissued(error)) => Err(LedgerError::Git(error)),
            Err(GitWriteError::Unknown(error)) => Err(LedgerError::Unknown(error)),
            Err(GitWriteError::PreparedExists(_)) => {
                Err(LedgerError::Git(GitError::ObjectMismatch))
            }
        }
    }

    fn retained(&self, closure: &[BlobDigest]) -> Result<bool, LedgerError> {
        for digest in closure {
            if !self.objects.retains(digest)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
