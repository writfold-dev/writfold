//! Tuft v0: the Writfold family's retrieval and projection language.
//!
//! This crate is pure: lexer, parser, typed AST, checker, IR, match location,
//! projection and rendering, with stable diagnostics and source spans. It
//! performs no IO. An adapter (`writfold tuft run`) lowers the IR's `find`
//! onto a storage backend at one pinned commit and its `retain` onto the
//! evidence store, then hands measured rows back to [`emit::emit`].
//!
//! Text and regex searches use exact bytes at one full commit id. Symbol
//! searches import an index with explicit partial coverage. `similar`, `rank`
//! and `follow` remain typed refusals (`T301`), never exact-match emulations.
#![forbid(unsafe_code)]

pub mod ast;
pub mod check;
mod compact;
pub mod cursor;
pub mod diag;
pub mod emit;
mod envelope;
pub mod ir;
pub mod lexer;
pub mod locate;
pub mod parser;
mod render;
pub mod scope;
pub mod view;

pub use diag::{Code, Deferred, Diagnostic, Span};
pub use ir::Plan;

/// Output format version (the cursor carries its own, see [`cursor`]).
pub const VERSION: &str = "tuft/0";

/// Lex, parse and check a stored program with full commit pins. Independent
/// lexer, parser and checker diagnostics are returned in source order.
pub fn compile(source: &[u8]) -> Result<Plan, Vec<Diagnostic>> {
    let program = parse_source(source)?;
    check::check(&program, source)
}

/// Check a one-shot program before its nonempty `at` ref is resolved by the
/// Git boundary. This plan must be pinned before execution or recording.
pub fn compile_unpinned(source: &[u8]) -> Result<Plan, Vec<Diagnostic>> {
    let program = parse_source(source)?;
    check::check_mode(&program, source, true)
}

/// Replace only the parsed `find`'s `at` string with a full commit id.
/// Comments, needles, whitespace and unrelated strings remain byte-exact.
pub fn pin(source: &[u8], commit: &str) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let program = parse_source(source)?;
    check::check_mode(&program, source, true)?;
    let at = program
        .bindings
        .iter()
        .find_map(|binding| {
            if let ast::Operation::Find(find) = &binding.operation {
                Some(find.at.span)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            vec![Diagnostic::new(
                Code::Pin,
                program.emit.span,
                "no find to pin",
            )]
        })?;
    if !check::full_commit(commit) {
        return Err(vec![Diagnostic::new(
            Code::Pin,
            at,
            "resolved commit must be a full 40- or 64-digit lowercase commit id",
        )]);
    }
    let mut pinned = Vec::with_capacity(source.len() - (at.hi - at.lo) + commit.len() + 2);
    pinned.extend_from_slice(&source[..at.lo]);
    pinned.push(b'"');
    pinned.extend_from_slice(commit.as_bytes());
    pinned.push(b'"');
    pinned.extend_from_slice(&source[at.hi..]);
    compile(&pinned)?;
    Ok(pinned)
}

fn parse_source(source: &[u8]) -> Result<ast::Program, Vec<Diagnostic>> {
    let lexer::Lexed {
        tokens,
        mut diagnostics,
    } = lexer::lex(source);
    if tokens.is_empty() {
        return Err(diagnostics);
    }
    match parser::parse(&tokens) {
        Ok(program) if diagnostics.is_empty() => Ok(program),
        result => {
            if let Err(errors) = result {
                diagnostics.extend(errors);
            }
            diagnostics.sort_by_key(|diagnostic| diagnostic.span);
            Err(diagnostics)
        }
    }
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
