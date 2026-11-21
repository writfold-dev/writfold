//! Writfold's pure decision kernels, shaped for the Charon/Aeneas translation.
//!
//! No IO, no floats, no trait objects, no dependencies, `no_std` plus
//! `alloc`. The `proofs` gate translates the functions named in
//! `[package.metadata.charon] start_from` to Lean 4 (`proofs/Extracted/`,
//! committed) and proves the kernel theorems over that translation
//! (`proofs/Writfold/`). The theorems are about this code, not a hand-written
//! Lean copy of it. Production crates call these functions. Every module
//! has its decisions in `start_from` (docs/writfold/aeneas.md lists them);
//! what stays outside is text plumbing, rendering and traversal (`command`'s
//! reader, `find`'s name forms, `diag`'s rendering, `walk`'s traversal),
//! tested, not proved.
//!
//! Translation rules for the functions in `start_from`: owned data and no
//! nested borrows; no derives reached (equality a theorem depends on is an
//! explicit `same` function or `matches!`); no integer casts; explicit
//! `while` loops instead of iterator adaptors.
#![no_std]
#![forbid(unsafe_code)]
// The workspace lint set, stated here because the proofs gate translates this
// crate in a shadow workspace that does not carry `workspace.lints`.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]
// Crate-level, not on the items: an attribute inside an extracted module
// would shift the source lines recorded in proofs/Extracted. Only lints that
// fire are listed (an unfulfilled expectation warns); a new Aeneas-shaped
// construct that trips another lint needs its own reasoned expectation.
#![expect(
    clippy::redundant_pattern_matching,
    clippy::question_mark,
    clippy::manual_filter,
    clippy::manual_map,
    reason = "explicit matches translate to plain Lean without depending on \
              further modelled library functions"
)]

extern crate alloc;

pub mod bind;
pub mod bytes;
pub mod checklist;
pub mod command;
pub mod decision;
pub mod delivery;
pub mod diag;
pub mod find;
pub mod gate;
pub mod ground;
pub mod grounding;
pub mod guard;
pub mod idle;
pub mod investigation;
pub mod observe;
pub mod order;
pub mod pin;
pub mod plan;
pub mod pointer;
pub mod policy;
pub mod scope;
pub mod store;
pub mod subject;
pub mod verdict;
pub mod walk;
