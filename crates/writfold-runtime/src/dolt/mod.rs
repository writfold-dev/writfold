//! Exact Dolt graph projection IO for the disposable managed profile.
//!
//! Readers get only the closed pinned SELECT vocabulary (`DoltQuery`). The one
//! realm writer additionally gets the closed conditional-commit vocabulary
//! (`DoltSql`, `write.rs`) behind an exclusive recorded writer lease
//! (`writer.rs`). Admission never initializes a database, lock entry or HOME,
//! and never checks out, migrates or performs GC. Native SQL may perform its own
//! administrative housekeeping; this is an application API, not an OS write
//! sandbox. The caller must supply private owner-only database, HOME and
//! coordination directories and an existing single-link database.lock (and
//! writer.lock for a writer).
//!
//! The qualified executable, its loader/libraries, procfs and managed filesystem
//! are trusted. All readers, writers, retention management and recovery must
//! share the same stable flock entry. Hostile same-user filesystem mutation,
//! processes escaping their group, independent child reaping and uncoordinated
//! native server startup are excluded. Directory/executable metadata checks do
//! not prove absence of an adversarial ABA change. Server discovery markers
//! are refused before/after every command; no fictitious --read-only native
//! flag is used. The environment is cleared, HOME is private, event/version
//! checks are disabled, and every SQL invocation disables automatic GC.
//!
//! Exact executable identity is not independent qualification, and graph data
//! returned by this boundary is not knowledge truth or publication authority.
mod config;
mod database;
mod error;
mod executable;
mod query;
mod write;
mod writer;

pub use database::{DoltDatabase, DoltLease};
pub use error::DoltError;
pub use executable::DoltExecutable;
pub use query::{DoltQuery, QUERY_CONTRACT, query_contract_digest};
pub use write::{DoltNodeChange, DoltRealmRow, DoltSql, DoltWrite, WRITE_HEAD};
pub use writer::{DoltWriterClaim, DoltWriterSlot};
