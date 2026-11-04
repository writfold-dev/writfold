//! Pure typed values for Writfold's preserved finite-witness semantics.
//!
//! Identities cannot cross namespaces merely because their strings match:
//! ```compile_fail
//! use writfold_domain::{ClaimId, ObservationId};
//! fn lookup_observation(_: ObservationId) {}
//! let claim: ClaimId = "same".parse().unwrap();
//! lookup_observation(claim);
//! ```
//! Runtime quantities also cannot cross units implicitly:
//! ```compile_fail
//! use writfold_domain::{ByteCount, DurationSeconds};
//! fn wait_for(_: DurationSeconds) {}
//! let bytes = ByteCount::new(12).unwrap();
//! wait_for(bytes);
//! ```
//! A semantic time is not a bounded operating-system timestamp:
//! ```compile_fail
//! use writfold_domain::{EpochSeconds, Integer};
//! fn semantic_time(_: Integer) {}
//! let os_time = EpochSeconds::new(12).unwrap();
//! semantic_time(os_time);
//! ```

pub mod admission;
pub mod authored;
pub mod engineering;
pub mod equality;
pub mod evidence;
mod identity_grammar;
pub mod ids;
pub mod integer;
pub mod knowledge;
pub mod language;
mod records;
pub mod request_generation;
pub mod storage;
pub mod text;
pub mod units;
pub mod workflow;

pub use admission::*;
pub use authored::*;
pub use evidence::*;
pub use ids::*;
pub use integer::Integer;
pub use knowledge::*;
pub use language::*;
pub use request_generation::RequestGeneration;
pub use storage::*;
pub use units::*;
pub use workflow::*;

#[cfg(verus_keep_ghost)]
use vstd::prelude::View;

::vstd::prelude::verus! {
#[inline]
pub const fn semantic_version() -> (value: &'static str)
    ensures value@ == "2.0.0"@,
{
    "2.0.0"
}
pub const SCHEMA_VERSION: u32 = 2;
}
