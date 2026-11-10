use thiserror::Error;

/// Named boundary capabilities. A backend lists the ones it implements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    ExactRead,
    ExactLookup,
    LexicalLookup,
    /// Edit-distance or similarity matching. Deferred until Omnigraph.
    Fuzzy,
    /// Embedding or hybrid matching. Deferred until Omnigraph.
    Semantic,
    /// Relevance ranking. Deferred until Omnigraph.
    Ranking,
    /// Graph relation traversal. Deferred until Omnigraph.
    Follow,
    /// Single-writer conditional-head admission.
    Write,
}

/// Typed refusals. None of them is an empty success or a latest-state fallback.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum Refusal {
    #[error("{0:?} is not supported by this backend and is never emulated")]
    Unsupported(Capability),
    #[error("{0:?} is defined by the boundary but unavailable in this backend")]
    Unavailable(Capability),
    #[error("invalid storage request: {0}")]
    InvalidRequest(&'static str),
    #[error("no record has this exact key at the pinned snapshot")]
    NotFound,
    #[error("retained bytes do not match the expected digest")]
    DigestMismatch,
    #[error("cursor was issued for a different pin or request")]
    CursorMismatch,
    #[error("writer does not own this realm: {0}")]
    NotOwner(&'static str),
    #[error("write is not authorized: {0}")]
    Unauthorized(&'static str),
    #[error("pin refused: {0}")]
    Pin(String),
    #[error("backend refused: {0}")]
    Backend(String),
    #[error("storage deadline elapsed")]
    Deadline,
    #[error("storage request was cancelled")]
    Cancelled,
}
