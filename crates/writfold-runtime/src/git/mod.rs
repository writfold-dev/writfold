//! Exact local immutable Git reads and the KB's own Git calls.
//!
//! The caller admits an executable using its externally supplied profile digest.
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
mod gate;
mod kb;
mod kb_renames;
mod kb_upstream;
mod objects;
mod profile;
mod tree;

mod traversal;
pub use admission::{GitExecutable, GitRepository};
pub use error::GitError;
pub use gate::{
    GateEntry, edit_hash_bytes, gate_hash_paths, gate_ls_tree, gate_worktree_add,
    gate_worktree_remove,
};
pub use kb::{
    KbCommit, KbDirty, KbRepository, KbStatus, kb_batch_check, kb_commit, kb_config, kb_diff_tree,
    kb_log, kb_ls_files, kb_status,
};
pub use kb_renames::kb_renames;
pub use kb_upstream::{KbUpstream, kb_is_ancestor, kb_upstream};
pub use objects::{GitBlob, GitEntry};
use std::time::Duration;
pub use tree::{TreeBlob, pinned_blobs, pinned_tree};
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
