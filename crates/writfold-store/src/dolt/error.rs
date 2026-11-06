use thiserror::Error;
use writfold_domain::IdentityError;
use writfold_protocol::dolt::ReadRefusal;
use writfold_runtime::dolt::DoltError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Source,
    Target,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Dolt runtime refused the read: {0}")]
    Runtime(#[from] DoltError),
    #[error("invalid Dolt read projection: {0}")]
    Wire(#[from] writfold_wire::dolt::Error),
    #[error("Dolt read pin mismatch: {0}")]
    Pin(#[from] ReadRefusal),
    #[error("invalid generated physical contract digest: {0}")]
    Digest(#[from] IdentityError),
    #[error("Dolt physical schema digest is not supported")]
    Schema,
    #[error("Dolt query contract digest is not supported")]
    Query,
    #[error("Dolt retained root does not name the exact requested commit")]
    RetentionRoot,
    #[error("Dolt edge row {edge_index} has a missing {endpoint:?} node")]
    DanglingEdge {
        edge_index: usize,
        endpoint: Endpoint,
    },
    #[error("Dolt row limit cannot include an overflow probe")]
    RowLimitOverflow,
    #[error("Dolt total read deadline exceeds the monotonic clock range")]
    DeadlineOverflow,
    #[error("Dolt total read deadline elapsed")]
    Deadline,
    #[error("Dolt read was cancelled")]
    Cancelled,
}
