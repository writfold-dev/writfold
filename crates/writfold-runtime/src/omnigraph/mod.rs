//! Experimental owned native boundary, not public writer admission.
//!
//! HTTP observations are never semantic receipts. In particular, a 4xx body
//! still needs wire-level disposition and precondition validation. Requests are
//! issued once; an uncertain result retains this server and its realm lease.
mod admission;
mod bootstrap;
mod chunked;
mod http;
mod request;
mod server;

pub use admission::{Credential, NativeExecutable, NativeRoot};
pub use bootstrap::{BootstrapFailure, BootstrapGraph, BootstrapObservation};
pub use request::{Parameter, Scalar, StoredQuery};
pub use server::NativeServer;

use crate::{fs::FsError, process::ProcessError};
use std::process::ExitStatus;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use writfold_domain::ByteCount;

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    #[error(transparent)]
    Filesystem(#[from] FsError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("native executable byte identity mismatch")]
    ExecutableMismatch,
    #[error("unsupported native boundary input: {0}")]
    Input(&'static str),
    #[error("native boundary budget exceeded: {0}")]
    Budget(&'static str),
    #[error("native boundary deadline expired")]
    Timeout,
    #[error("native boundary cancelled")]
    Cancelled,
    #[error("owned native server exited")]
    Exited,
    #[error("invalid native HTTP or startup protocol: {0}")]
    Protocol(&'static str),
    #[error(transparent)]
    Bootstrap(#[from] Box<BootstrapFailure>),
    #[error("native binding failed ({cause}); containment also failed ({cleanup})")]
    Containment {
        cause: Box<NativeError>,
        cleanup: ProcessError,
    },
}

/// Independent request/header/body and cumulative process-output bounds.
/// Response headers also have a 65,536-byte/128-field ceiling; chunked bodies
/// have at most 65,536 data chunks and no extensions or trailers in this profile.
#[derive(Clone, Copy, Debug)]
pub struct NativeLimits {
    pub request_header: ByteCount,
    pub request_body: ByteCount,
    pub response_header: ByteCount,
    pub response_body: ByteCount,
    pub stdout: ByteCount,
    pub stderr: ByteCount,
    pub shutdown_grace: Duration,
}

/// One caller-owned total deadline and cancellation signal, shared by all
/// encoding, connection and response steps of an operation.
#[derive(Clone, Copy, Debug)]
pub struct NativeCall<'a> {
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

/// Bounded native response headers and exact transfer-decoded payload bytes.
/// Even 200 is only a transport observation, not proof of application success.
#[derive(Debug)]
pub struct RawResponse {
    pub status: u16,
    pub headers: Vec<u8>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub enum TransportOutcome {
    /// No HTTP request bytes were dispatched.
    Unissued(NativeError),
    /// Some bytes may have escaped. Do not retry or infer no effect.
    Unknown(NativeError),
    Observed(RawResponse),
}

/// Produced only after owned-group cleanup, independently of request outcomes.
#[derive(Debug)]
pub struct ShutdownObservation {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub forced_kill: bool,
}

#[cfg(test)]
mod tests;
