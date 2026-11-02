//! Immutable, identity-checked signed operation inputs; not publication authority.
mod codec;
mod prepare;

use crate::{ApprovalDomain, Limits};
pub use codec::BoundaryError;
pub(crate) use codec::{encode, hash, hex, unhex};
use writfold_domain::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationHeader {
    id: PlanId,
    policy: PolicyDigest,
    toolchain: ContentDigest,
}
impl OperationHeader {
    pub fn id(&self) -> &PlanId {
        &self.id
    }
    pub fn policy(&self) -> &PolicyDigest {
        &self.policy
    }
    pub fn toolchain(&self) -> &ContentDigest {
        &self.toolchain
    }
}

macro_rules! operation_record {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { header: OperationHeader, $($field: $ty,)* }
        impl $name {
            pub fn header(&self) -> &OperationHeader { &self.header }
            $(pub fn $field(&self) -> &$ty { &self.$field })*
        }
    };
}
operation_record!(LegacyAdmissionPlan {
    base: Option<ReleaseId>, captures: Vec<SourceCaptureId>, context: Context, semantic: Plan,
});
operation_record!(LegacyReleaseSelectionPlan {
    base: ReleaseId,
    retained: ReleaseId,
    context: Context,
    semantic: SelectionPlan,
});
operation_record!(PortfolioAdmissionPlan {
    base: Option<ReleaseId>, captures: Vec<SourceCaptureId>, target_use: String,
    semantic: PortfolioPlan,
});
operation_record!(PortfolioReleaseSelectionPlan {
    base: ReleaseId,
    retained: ReleaseId,
    target_use: String,
    semantic: PortfolioSelectionPlan,
});

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationBody {
    LegacyAdmission(Box<LegacyAdmissionPlan>),
    LegacySelection(Box<LegacyReleaseSelectionPlan>),
    PortfolioAdmission(PortfolioAdmissionPlan),
    PortfolioSelection(PortfolioReleaseSelectionPlan),
}

/// Construction requires strict decoding or an explicit schema-3 preparation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    body: OperationBody,
    message: Vec<u8>,
}
impl Operation {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, BoundaryError> {
        codec::decode(bytes, limits)
    }
    pub fn body(&self) -> &OperationBody {
        &self.body
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.message[self.domain().prefix().len()..]
    }
    pub fn header(&self) -> &OperationHeader {
        match &self.body {
            OperationBody::LegacyAdmission(v) => &v.header,
            OperationBody::LegacySelection(v) => &v.header,
            OperationBody::PortfolioAdmission(v) => &v.header,
            OperationBody::PortfolioSelection(v) => &v.header,
        }
    }
    pub fn schema_version(&self) -> u8 {
        match self.body {
            OperationBody::LegacyAdmission(_) | OperationBody::LegacySelection(_) => 2,
            _ => 3,
        }
    }
    pub fn domain(&self) -> ApprovalDomain {
        if self.schema_version() == 2 {
            ApprovalDomain::HistoricalSchema2
        } else {
            ApprovalDomain::WorkflowSchema3
        }
    }
    pub fn actor(&self) -> Result<AuthorityKeyId, BoundaryError> {
        let actor = match &self.body {
            OperationBody::LegacyAdmission(v) => v.semantic.authority().actor(),
            OperationBody::LegacySelection(v) => v
                .semantic
                .actor()
                .as_ref()
                .ok_or(BoundaryError::Binding("selection requires an actor"))?,
            OperationBody::PortfolioAdmission(v) => v.semantic.actor(),
            OperationBody::PortfolioSelection(v) => v.semantic.actor(),
        };
        actor.as_str().parse().map_err(|_| BoundaryError::Identity)
    }
    pub fn target_use(&self) -> &str {
        match &self.body {
            OperationBody::LegacyAdmission(v) => v.context.intended_use(),
            OperationBody::LegacySelection(v) => v.context.intended_use(),
            OperationBody::PortfolioAdmission(v) => &v.target_use,
            OperationBody::PortfolioSelection(v) => &v.target_use,
        }
    }
    pub fn approval_message(&self) -> &[u8] {
        &self.message
    }
}
