//! Read-only pinned-Git-tree adapter for the storage boundary (Tuft v0, T0).
//!
//! A record is one blob of the tree of a full commit id: key = repository
//! path, kind = `blob` (Git's object type), content = the exact blob bytes.
//! The adapter is scoped to a byte prefix of paths (empty: the whole tree).
//! Records outside the scope do not exist for it, so measured coverage counts
//! files in scope. Every lookup runs the shared pure `scan` over the complete
//! pinned scope: no index, no ranking, no fuzzy or semantic matching. Writes
//! are `Unavailable`; this adapter never writes a repository.
//!
//! The listing is cached per pin; contents load once per pin (in parallel,
//! bounded by `max_bytes`) the first time a lookup needs them. A lone
//! `read_exact` before any lookup reads just its blob.
use super::BackendKind;
use super::{
    Capability, ExactRead, ExactRecord, LookupCursor, LookupPage, LookupRequest, RecordKey,
    RecordKind, Refusal, ScanRecord, StorageBackend, WriteOutcome, WriteRequest, admit_lookup,
    scan,
};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use writfold_domain::GitObjectId;
use writfold_runtime::git::{
    GitBudgets, GitError, GitExecutable, TreeBlob, pinned_blobs, pinned_tree,
};
use writfold_runtime::process::ProcessError;

const CAPABILITIES: &[Capability] = &[
    Capability::ExactRead,
    Capability::ExactLookup,
    Capability::LexicalLookup,
];
const KIND: &str = "blob";

struct Snapshot {
    pin: GitObjectId,
    blobs: Vec<TreeBlob>,
    contents: Mutex<Option<Arc<Vec<Vec<u8>>>>>,
}

/// One repository, one path scope, fixed budgets.
pub struct GitTreeBackend<'a> {
    executable: &'a GitExecutable<'a>,
    git_dir: PathBuf,
    scope: String,
    budgets: GitBudgets,
    max_bytes: u64,
    cache: Mutex<Option<Arc<Snapshot>>>,
}

impl<'a> GitTreeBackend<'a> {
    /// `max_bytes` bounds the bytes of all blobs in scope; a larger scope
    /// refuses rather than searching part of it.
    pub fn new(
        executable: &'a GitExecutable<'a>,
        git_dir: PathBuf,
        scope: String,
        budgets: GitBudgets,
        max_bytes: u64,
    ) -> Self {
        Self {
            executable,
            git_dir,
            scope,
            budgets,
            max_bytes,
            cache: Mutex::new(None),
        }
    }

    fn snapshot(
        &self,
        pin: &GitObjectId,
        cancelled: &AtomicBool,
    ) -> Result<Arc<Snapshot>, Refusal> {
        let mut cache = self.cache.lock().map_err(|_| poisoned())?;
        if let Some(snapshot) = cache.as_ref().filter(|s| &s.pin == pin) {
            return Ok(Arc::clone(snapshot));
        }
        let mut blobs = pinned_tree(self.executable, &self.git_dir, pin, self.budgets, cancelled)
            .map_err(refusal)?;
        blobs.retain(|b| b.path.as_bytes().starts_with(self.scope.as_bytes()));
        let snapshot = Arc::new(Snapshot {
            pin: pin.clone(),
            blobs,
            contents: Mutex::new(None),
        });
        *cache = Some(Arc::clone(&snapshot));
        Ok(snapshot)
    }

    /// Load every blob in scope at `pin` now (one parallel read) instead of
    /// one read per `read_exact`; for callers that will read the whole scope.
    pub fn preload(&self, pin: &GitObjectId, cancelled: &AtomicBool) -> Result<(), Refusal> {
        let snapshot = self.snapshot(pin, cancelled)?;
        self.contents(&snapshot, cancelled).map(drop)
    }

    fn contents(
        &self,
        snapshot: &Snapshot,
        cancelled: &AtomicBool,
    ) -> Result<Arc<Vec<Vec<u8>>>, Refusal> {
        let mut slot = snapshot.contents.lock().map_err(|_| poisoned())?;
        if let Some(contents) = slot.as_ref() {
            return Ok(Arc::clone(contents));
        }
        let ids: Vec<GitObjectId> = snapshot.blobs.iter().map(|b| b.blob.clone()).collect();
        let contents = pinned_blobs(
            self.executable,
            &self.git_dir,
            &ids,
            self.budgets,
            cancelled,
        )
        .map_err(refusal)?;
        let total: u64 = contents.iter().map(|c| c.len() as u64).sum();
        if total > self.max_bytes {
            return Err(Refusal::Backend(format!(
                "pinned scope holds {total} bytes, over the {} byte budget; narrow the scope",
                self.max_bytes
            )));
        }
        let contents = Arc::new(contents);
        *slot = Some(Arc::clone(&contents));
        Ok(contents)
    }
}

impl StorageBackend for GitTreeBackend<'_> {
    type Pin = GitObjectId;

    fn kind(&self) -> BackendKind {
        BackendKind::GitTree
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }

    fn read_exact(
        &self,
        pin: &GitObjectId,
        request: &ExactRead,
        cancelled: &AtomicBool,
    ) -> Result<ExactRecord<GitObjectId>, Refusal> {
        let snapshot = self.snapshot(pin, cancelled)?;
        let wanted = request.key().as_str().as_bytes();
        let index = snapshot
            .blobs
            .binary_search_by(|b| b.path.as_bytes().cmp(wanted))
            .map_err(|_| Refusal::NotFound)?;
        let loaded = snapshot.contents.lock().map_err(|_| poisoned())?.clone();
        let content = match loaded.as_ref().and_then(|c| c.get(index)) {
            Some(bytes) => bytes.clone(),
            None => {
                let id = snapshot
                    .blobs
                    .get(index)
                    .map(|b| b.blob.clone())
                    .ok_or(Refusal::NotFound)?;
                pinned_blobs(
                    self.executable,
                    &self.git_dir,
                    &[id],
                    self.budgets,
                    cancelled,
                )
                .map_err(refusal)?
                .pop()
                .ok_or_else(|| Refusal::Backend("blob read returned nothing".into()))?
            }
        };
        let record = ExactRecord::new(pin.clone(), request.key().clone(), kind()?, content)?;
        if request.expected().is_some_and(|e| e != record.digest()) {
            return Err(Refusal::DigestMismatch);
        }
        Ok(record)
    }

    fn lookup(
        &self,
        pin: &GitObjectId,
        request: &LookupRequest,
        cursor: Option<&LookupCursor<GitObjectId>>,
        cancelled: &AtomicBool,
    ) -> Result<LookupPage<GitObjectId>, Refusal> {
        admit_lookup(pin, request, cursor, CAPABILITIES)?;
        let snapshot = self.snapshot(pin, cancelled)?;
        // Every hit carries the digest of its content, so every lookup needs
        // the bytes, whatever field it matches.
        let contents = self.contents(&snapshot, cancelled)?;
        if contents.len() != snapshot.blobs.len() {
            return Err(Refusal::Backend(
                "blob count differs from the listing".into(),
            ));
        }
        let records: Vec<ScanRecord<'_>> = snapshot
            .blobs
            .iter()
            .zip(contents.iter())
            .map(|(b, content)| ScanRecord {
                key: &b.path,
                kind: KIND,
                content,
            })
            .collect();
        scan(&snapshot.pin, &records, request, cursor)
    }

    fn admit_write(
        &mut self,
        _request: &WriteRequest<GitObjectId>,
        _cancelled: &AtomicBool,
    ) -> Result<WriteOutcome<GitObjectId>, Refusal> {
        Err(Refusal::Unavailable(Capability::Write))
    }
}

fn kind() -> Result<RecordKind, Refusal> {
    RecordKind::new(KIND.to_owned())
}

fn poisoned() -> Refusal {
    Refusal::Backend("Git tree cache lock poisoned".into())
}

fn refusal(error: GitError) -> Refusal {
    match error {
        GitError::Malformed("pinned commit") => {
            Refusal::Pin("not a commit in this repository".into())
        }
        GitError::Process(ProcessError::Cancelled) => Refusal::Cancelled,
        GitError::Process(ProcessError::Deadline) => Refusal::Deadline,
        other => Refusal::Backend(other.to_string()),
    }
}

/// Record key of a repository path; paths beyond the boundary's key bound refuse.
pub fn path_key(path: &str) -> Result<RecordKey, Refusal> {
    RecordKey::new(path.to_owned())
}
