//! Exact read-only Dolt graph projection IO for the disposable managed profile.
//!
//! Only the closed pinned SELECT vocabulary is available. Admission never
//! initializes a database, lock entry or HOME, and never checks out, migrates,
//! writes application tables or performs GC. Native SQL may perform its own
//! administrative housekeeping: this is a read-only application API, not an OS
//! write sandbox. The caller must supply private owner-only database, HOME and
//! coordination directories and an existing single-link database.lock.
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

pub use database::{DoltDatabase, DoltLease};
pub use error::DoltError;
pub use executable::DoltExecutable;
pub use query::{DoltQuery, QUERY_CONTRACT, query_contract_digest};
