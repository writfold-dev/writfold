//! Pure citation judgments under explicit retained-byte and current-policy assumptions.
//! Inputs are not authority capabilities. Exact preimages, provenance, and access are
//! supplied by the host; this layer does no IO and cannot mint legacy receipts.
//! Search diagnoses text only: neither uniqueness nor an exhaustive miss proves lineage.
mod compare;
mod handles;
mod presentation;
mod resolution;
mod reuse;
mod scan;
pub use compare::*;
pub use handles::*;
pub use presentation::*;
pub use resolution::*;
pub use reuse::*;
pub use scan::{ScanResult, diagnose};
