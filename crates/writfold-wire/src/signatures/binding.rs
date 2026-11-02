use super::*;
use crate::operation::OperationBody;
use writfold_domain::NamedContext;

pub(super) fn approval(
    operation: &Operation,
    approval: &Approval,
    policy: &AuthorityPolicy,
) -> Result<(), BoundaryError> {
    if operation.schema_version() != approval.schema_version() {
        return Err(BoundaryError::Binding(
            "approval schema differs from operation",
        ));
    }
    if operation.header().id() != approval.plan()
        || operation.actor()? != *approval.key()
        || operation.header().policy() != policy.identity()
    {
        return Err(BoundaryError::Binding("plan, actor or policy differs"));
    }
    match operation.body() {
        OperationBody::LegacyAdmission(v) => {
            if v.semantic().authority().context() != v.context() {
                return Err(BoundaryError::Binding("legacy semantic context differs"));
            }
        }
        OperationBody::LegacySelection(v) => {
            if v.semantic().context() != v.context() {
                return Err(BoundaryError::Binding("legacy semantic context differs"));
            }
        }
        OperationBody::PortfolioAdmission(v) => {
            contexts(operation, v.semantic().contexts(), policy)?
        }
        OperationBody::PortfolioSelection(v) => {
            contexts(operation, v.semantic().contexts(), policy)?
        }
    }
    Ok(())
}

pub(super) fn current(
    operation: &Operation,
    policy: &AuthorityPolicy,
    expected_toolchain: &ContentDigest,
) -> Result<(), BoundaryError> {
    if operation.schema_version() != 3 {
        return Err(BoundaryError::Binding(
            "historical schema2 operations are read-only",
        ));
    }
    if operation.header().policy() != policy.identity()
        || operation.header().toolchain() != expected_toolchain
    {
        return Err(BoundaryError::Binding("policy or toolchain differs"));
    }
    match operation.body() {
        OperationBody::PortfolioAdmission(v) => {
            contexts(operation, v.semantic().contexts(), policy)
        }
        OperationBody::PortfolioSelection(v) => {
            contexts(operation, v.semantic().contexts(), policy)
        }
        _ => Err(BoundaryError::Binding(
            "historical schema2 operations are read-only",
        )),
    }
}

fn contexts(
    operation: &Operation,
    contexts: &[NamedContext],
    policy: &AuthorityPolicy,
) -> Result<(), BoundaryError> {
    if !contexts
        .iter()
        .any(|named| named.context().intended_use() == operation.target_use())
    {
        return Err(BoundaryError::Binding(
            "declared operation use is absent from contexts",
        ));
    }
    let actor = operation.actor()?;
    let key = policy.key(&actor, operation.target_use())?;
    for named in contexts {
        if !key.permits(named.context().intended_use()) {
            return Err(BoundaryError::Binding(
                "actor is not authorized for every context use",
            ));
        }
        policy.validate_context(named.context())?;
    }
    Ok(())
}
