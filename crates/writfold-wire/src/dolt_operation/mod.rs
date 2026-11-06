//! Checked dolt transitions over unchanged workflow-3 operations.
//! These records bind supplied inputs, not backend state or publication authority.
mod binding;
mod codec;
mod encode;
mod records;

use crate::{Budget, Error, Limits, RawJson, operation::Operation};
pub use records::{DoltBase, DoltTarget};
use writfold_domain::PlanDigest;

pub const DOLT_OPERATION_NAMESPACE: &str = "writfold-dolt-operation";
pub const DOLT_OPERATION_SCHEMA: u64 = 1;
pub const DOLT_BACKEND: &str = "dolt-integration";
pub const DOLT_PLAN_DOMAIN: &[u8] = b"writfold-dolt-plan-v1\0";
pub const DOLT_OPERATION_DOMAIN: &[u8] = b"writfold-dolt-operation-v1\0";

#[derive(Debug, thiserror::Error)]
pub enum DoltOperationError {
    #[error(transparent)]
    Wire(#[from] crate::Error),
    #[error(transparent)]
    Operation(#[from] crate::operation::BoundaryError),
    #[error(transparent)]
    Pin(#[from] crate::dolt::Error),
    #[error(transparent)]
    Shape(#[from] crate::semantic::InvalidRequest),
    #[error("unsupported dolt operation namespace")]
    UnsupportedNamespace,
    #[error("unsupported dolt operation schema")]
    UnsupportedVersion,
    #[error("unsupported dolt target backend")]
    UnsupportedBackend,
    #[error("dolt operation requires workflow schema 3")]
    UnsupportedOperation,
    #[error("dolt binding refused: {0}")]
    Binding(&'static str),
    #[error("dolt operation canonical identity differs")]
    Identity,
}

type Result<T, E = DoltOperationError> = std::result::Result<T, E>;

/// Immutable, noncircular approval input. No candidate native commit is signed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltOperation {
    id: PlanDigest,
    operation: Operation,
    base: Option<DoltBase>,
    target: DoltTarget,
    mapping: Option<PlanDigest>,
    message: Vec<u8>,
}

impl DoltOperation {
    /// All limits apply to the complete envelope. The byte budget includes the
    /// approval-domain prefix, for both original decode input and canonical output.
    /// A mapping is structural provenance, not a claim of kernel migration approval.
    pub fn prepare(
        operation: Operation,
        base: Option<DoltBase>,
        target: DoltTarget,
        mapping: Option<PlanDigest>,
        limits: Limits,
    ) -> Result<Self> {
        let input_limits = envelope_limits(limits)?;
        binding::check(&operation, base.as_ref(), &target, mapping.as_ref())?;
        let (id, message) = encode::message(
            &operation,
            base.as_ref(),
            &target,
            mapping.as_ref(),
            limits.bytes(),
        )?;
        // Enforce structural budgets on the assembled envelope, not independently
        // on its children. RawJson destruction remains iterative on all paths.
        RawJson::parse(&message[DOLT_OPERATION_DOMAIN.len()..], input_limits)?;
        Ok(Self {
            id,
            operation,
            base,
            target,
            mapping,
            message,
        })
    }

    /// Duplicate-aware, bounded decoding; no old-operation or old-domain fallback.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self> {
        codec::decode(bytes, limits)
    }
    pub fn id(&self) -> &PlanDigest {
        &self.id
    }
    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    pub fn base(&self) -> &Option<DoltBase> {
        &self.base
    }
    pub fn target(&self) -> &DoltTarget {
        &self.target
    }
    pub fn mapping(&self) -> &Option<PlanDigest> {
        &self.mapping
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.message[DOLT_OPERATION_DOMAIN.len()..]
    }
    pub fn approval_message(&self) -> &[u8] {
        &self.message
    }
}

fn envelope_limits(limits: Limits) -> Result<Limits> {
    let bytes = limits
        .bytes()
        .checked_sub(DOLT_OPERATION_DOMAIN.len())
        .ok_or(Error::Limit {
            budget: Budget::Bytes,
            offset: limits.bytes(),
        })?;
    Ok(Limits::new(
        bytes,
        limits.depth(),
        limits.container_entries(),
        limits.nodes(),
    )?)
}
