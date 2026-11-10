//! Bounded JSON-RPC replies for transport/session control and, when negotiated,
//! the `writfold.trial/1` methods.
//!
//! Encoding is pure: metadata authenticates nothing, no method is dispatched,
//! and the caller must commit lifecycle changes only after encoding succeeds.
//! Absent IDs are notifications and cannot be encoded; explicit null is legal.
//! The byte budget includes the final LF. A failure returns no partial reply.
pub(crate) mod bounded;
mod codec;
mod fault;
mod trial;

pub use codec::{encode_error, encode_success};
pub use trial::{
    CheckSummary, EvidenceListPayload, EvidenceShowPayload, PreparePayload, ResolvedCitation,
    RetainPayload, RunPayload, ShowPayload, SpliceSummary, TrialCategory, TrialOutcome,
    TrialRefusal, encode_trial_refusal, encode_trial_success,
};
use writfold_domain::{ContentDigest, InstanceId, SessionIncarnation};

pub const PROFILE: &str = "writfold.agent/1";
pub const MAX_RESPONSE_BYTES: usize = 65_536;
/// Exact closed legacy parameter schemas, not a data-service advertisement.
pub const METHOD_SCHEMAS: &str = include_str!("../../../contracts/agent-stdio/1/methods.json");
/// The method advertisement for a session that negotiated `writfold.trial/1`:
/// the control schemas plus the six trial methods.
pub const TRIAL_METHOD_SCHEMAS: &str =
    include_str!("../../../contracts/agent-stdio/1/trial-methods.json");
/// Self-describing pinned control-profile contract; data-service requirements
/// remain outside its implemented method set.
pub const CONTRACT: &[u8] = include_bytes!("../../../contracts/agent-stdio/1/contract.json");

/// Borrowed invocation metadata, not durable ownership or authorization.
#[derive(Clone, Copy, Debug)]
pub struct ServerMetadata<'a> {
    version: &'a str,
    instance: &'a InstanceId,
    incarnation: &'a SessionIncarnation,
    contract_digest: &'a ContentDigest,
}

impl<'a> ServerMetadata<'a> {
    pub fn new(
        version: &'a str,
        instance: &'a InstanceId,
        incarnation: &'a SessionIncarnation,
        contract_digest: &'a ContentDigest,
    ) -> Self {
        Self {
            version,
            instance,
            incarnation,
            contract_digest,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Success {
    /// Initialization without negotiated capabilities beyond notifications.
    Initialize,
    /// Initialization that negotiated `writfold.trial/1`.
    InitializeTrial,
    Shutdown,
}

/// Closed faults with static public diagnostics, never raw input values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    Parse,
    Oversized,
    InvalidRequest,
    InvalidParams,
    MethodNotFound,
    InitializationRequired,
    AlreadyInitialized,
    ShutDown,
    UnsupportedCapability,
    GenerationExhausted,
}

#[derive(Clone, Copy, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ResponseError {
    #[error("notifications have no response ID")]
    AbsentId,
    #[error("response exceeds the LF-inclusive output budget")]
    Limit,
    #[error("response output allocation failed")]
    Allocation,
    #[error("response JSON encoding failed")]
    Encoding,
}

impl From<bounded::Failure> for ResponseError {
    fn from(failure: bounded::Failure) -> Self {
        match failure {
            bounded::Failure::Limit => Self::Limit,
            bounded::Failure::Allocation => Self::Allocation,
            bounded::Failure::Encoding => Self::Encoding,
        }
    }
}
