//! Tuft v0: the Writfold family's retrieval and projection language.
//!
//! This crate is pure: lexer, parser, typed AST, checker, IR, match location,
//! projection and rendering, with stable diagnostics and source spans. It
//! performs no IO. An adapter (`writfold tuft run`) lowers the IR's `find`
//! onto a storage backend at one pinned commit and its `retain` onto the
//! evidence store, then hands measured rows back to [`emit::emit`].
//!
//! v0 is exact: `find text` and `find rx` over one full commit id. `similar`,
//! `rank` and `follow` parse but are refused by type (`T301`) until Omnigraph
//! exists; they are never emulated with exact matching.
#![forbid(unsafe_code)]

pub mod ast;
pub mod check;
pub mod diag;
pub mod emit;
pub mod ir;
pub mod lexer;
pub mod locate;
pub mod parser;
mod render;
pub mod view;

pub use diag::{Code, Deferred, Diagnostic, Span};
pub use ir::Plan;

/// Output and cursor format version.
pub const VERSION: &str = "tuft/0";

/// Lex, parse and check a program. On failure every diagnostic is returned
/// in source order (lexing and parsing stop at their first error).
pub fn compile(source: &[u8]) -> Result<Plan, Vec<Diagnostic>> {
    let tokens = lexer::lex(source).map_err(|d| vec![d])?;
    let program = parser::parse(&tokens).map_err(|d| vec![d])?;
    check::check(&program, source)
}

/// SHA-256 of `bytes` as lowercase hexadecimal.
pub fn digest_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
