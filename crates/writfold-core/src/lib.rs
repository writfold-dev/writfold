//! Writfold's pure decision kernels, shaped for the Charon/Aeneas translation.
//!
//! No IO, no floats, no `dyn`, no dependencies, `no_std` plus `alloc`. The
//! `proofs` gate translates this crate to Lean 4 (`proofs/Extracted/`,
//! committed) and proves the kernel theorems over that translation
//! (`proofs/Writfold/`). The theorems are about this code, not a hand-written
//! Lean copy of it. Production crates call these functions.
//!
//! Translation rules (recorded once, applied everywhere): owned data and no
//! nested borrows; no derives in translated modules (equality a theorem
//! depends on is an explicit `same` function); no integer casts; explicit
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
#![allow(
    clippy::redundant_pattern_matching,
    clippy::match_like_matches_macro,
    clippy::new_without_default,
    clippy::needless_range_loop,
    clippy::manual_memcpy,
    clippy::explicit_counter_loop,
    clippy::needless_return,
    clippy::vec_init_then_push,
    reason = "explicit matches, indexed loops and pushes translate to plain Lean \
              without depending on further modelled library functions"
)]

extern crate alloc;

pub mod authority;
pub mod bytes;
pub mod convenience;
pub mod ledger;
pub mod trial;
