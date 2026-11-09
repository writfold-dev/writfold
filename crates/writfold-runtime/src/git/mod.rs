//! Exact local immutable Git reads and explicitly leased native publication.
//!
//! The caller admits an executable using its externally supplied profile digest
//! (the maintained tool profile is `qualification/evaluation-profile.json`).
//! Matching that digest establishes identity, not independent tool qualification.
//! The native Git, dynamic loader/libraries, Linux procfs and managed repository
//! metadata/object directories are trusted and must remain administratively
//! controlled during each operation. Admission pins directory and executable
//! descriptors, not a snapshot of an adversarially mutable filesystem. This is
//! not a general filesystem sandbox. Standalone loose and ordinary non-promisor
//! packed object stores are supported, with checked regular pack/index entries.
//! The managed config grammar admits core format/boolean settings and harmless
//! remote/branch/user metadata. Linked worktrees, alternates, includes, partial
//! clones, unknown pack entries and other config keys/extensions fail.
//! Each lookup hashes the raw commit and every tree on the selected path; each
//! blob read also hashes its raw bytes with the repository's SHA-1/SHA-256 format.
//! No retained read creates timestamps, validation receipts or authority.
//! Writer admission is separate: a private owner-controlled repository, permanent
//! ManagedLocks namespace and cooperating writer discipline are required. New
//! initialization is bare SHA-256 only; historical SHA-1 reads are unchanged.
//! Blobs and prepared commits have permanent retention refs; only expected-old
//! `refs/kctx/knowledge` CAS publishes. Unknown native outcomes require exact
//! operation/history reconciliation, never blind retries or head-mismatch guesses.
//! Native fsync observations cover process crashes, not a power-loss guarantee.
//!
//! The caller explicitly enables permanent process-wide subreaper supervision
//! before executable admission. Each command owns a process group; termination
//! kills that group, reaps its descendants and leader, and drains owned pipes.
//! Pipe EOF or leader exit alone never establishes descendant quiescence.
//! This requires procfs, unchanged subreaper/SIGCHLD handling, no independent
//! reaping of the owned group and no descendants escaping their process group.
//! Arbitrary hostile executables require a stronger sandbox, not this API.
mod admission;
mod command;
mod error;
mod ledger;
mod objects;
mod preparation;
mod profile;
mod publication;
mod trial;
mod write_objects;
mod writer;
mod writer_profile;
pub use preparation::{GitManifestEntry, GitPrepareInput, GitPrepared};
pub use writer::{GitWriteError, GitWriter, GitWriterLease};

mod traversal;
pub use admission::{GitExecutable, GitRepository};
pub use error::GitError;
pub use ledger::{LedgerCommit, ledger_ref};
pub use objects::{GitBlob, GitEntry};
use std::time::Duration;
pub use trial::{trial_worktree_add, trial_worktree_remove};
use writfold_domain::ByteCount;

/// Per-command output limits, per-blob bytes, and a total read deadline.
#[derive(Clone, Copy, Debug)]
pub struct GitBudgets {
    stdout: ByteCount,
    stderr: ByteCount,
    blob: ByteCount,
    timeout: Duration,
}

impl GitBudgets {
    pub fn new(stdout: ByteCount, stderr: ByteCount, blob: ByteCount, timeout: Duration) -> Self {
        Self {
            stdout,
            stderr,
            blob,
            timeout,
        }
    }
}
