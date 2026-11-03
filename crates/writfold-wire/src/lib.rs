//! Bounded retained JSON, Python-compatible canonical bytes and approval messages.
#![forbid(unsafe_code)]

mod approval;
mod canonical;
pub mod operation;
mod parser;
pub mod policy;
pub mod reference;
pub mod rpc;
pub mod semantic;
pub mod signatures;
pub mod storage;
pub mod storage_operation;

pub use approval::{ApprovalDomain, OperationVariant};
pub use canonical::canonical;
use serde_json::Value;
use thiserror::Error;

/// Implementation ceiling, not a protocol default. Parsing, encoding and owned
/// value destruction use explicit work stacks, independent of native stack depth.
pub const MAX_SUPPORTED_DEPTH: usize = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    bytes: usize,
    depth: usize,
    container_entries: usize,
    nodes: usize,
}

impl Limits {
    /// Depth counts open arrays/objects; scalar roots have depth zero.
    /// Nodes count all values, including the root (not object keys).
    pub fn new(
        bytes: usize,
        depth: usize,
        container_entries: usize,
        nodes: usize,
    ) -> Result<Self, Error> {
        if depth > MAX_SUPPORTED_DEPTH {
            return Err(Error::UnsupportedDepth { requested: depth });
        }
        Ok(Self {
            bytes,
            depth,
            container_entries,
            nodes,
        })
    }

    pub fn bytes(self) -> usize {
        self.bytes
    }
    pub fn depth(self) -> usize {
        self.depth
    }
    pub fn container_entries(self) -> usize {
        self.container_entries
    }
    pub fn nodes(self) -> usize {
        self.nodes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Budget {
    Bytes,
    Depth,
    ContainerEntries,
    Nodes,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("requested depth {requested} exceeds the implementation ceiling")]
    UnsupportedDepth { requested: usize },
    #[error("{budget:?} budget exceeded at byte {offset}")]
    Limit { budget: Budget, offset: usize },
    #[error("invalid UTF-8 at byte {offset}")]
    Unicode { offset: usize },
    #[error("invalid JSON at byte {offset}: {reason}")]
    Syntax { offset: usize, reason: String },
    #[error("duplicate JSON key {key:?} at byte {offset}")]
    DuplicateKey { key: String, offset: usize },
    #[error("unsupported nonfinite JSON number at byte {offset}")]
    UnsupportedNumber { offset: usize },
    #[error("trailing JSON input at byte {offset}")]
    TrailingInput { offset: usize },
    #[error("invalid operation envelope: {reason}")]
    Envelope { reason: &'static str },
}

/// Raw boundary only: no domain validity or authenticity is implied.
/// No mutable value access or unchecked deserialization is provided.
#[derive(Debug)]
pub struct RawJson {
    value: Value,
}

impl RawJson {
    pub fn parse(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        parser::parse(bytes, limits).map(|value| Self { value })
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

impl Drop for RawJson {
    fn drop(&mut self) {
        parser::drop_value(std::mem::take(&mut self.value));
    }
}
