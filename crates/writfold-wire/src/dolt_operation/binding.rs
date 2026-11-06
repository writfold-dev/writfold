use super::{DoltBase, DoltOperationError, DoltTarget, Result};
use crate::operation::{Operation, OperationBody};
use writfold_domain::PlanDigest;

pub(super) fn check(
    operation: &Operation,
    base: Option<&DoltBase>,
    target: &DoltTarget,
    mapping: Option<&PlanDigest>,
) -> Result<()> {
    let semantic_base = match operation.body() {
        OperationBody::PortfolioAdmission(plan) => plan.base().as_ref(),
        OperationBody::PortfolioSelection(plan) => Some(plan.base()),
        _ => return Err(DoltOperationError::UnsupportedOperation),
    };
    if semantic_base != base.map(DoltBase::release) {
        return Err(DoltOperationError::Binding("embedded base release differs"));
    }
    let Some(base) = base else {
        return if mapping.is_some() {
            Err(DoltOperationError::Binding(
                "bootstrap cannot have a mapping",
            ))
        } else {
            Ok(())
        };
    };
    let pin = base.pin();
    if pin.realm() != target.realm() || pin.kb() != target.kb() {
        return Err(DoltOperationError::Binding(
            "base and target realm or KB differ",
        ));
    }
    // Producer-only changes do not themselves imply a data transformation.
    let needs_mapping = pin.database() != target.database()
        || pin.epoch() != target.epoch()
        || pin.profile() != target.profile();
    if needs_mapping && mapping.is_none() {
        return Err(DoltOperationError::Binding(
            "Dolt transition requires a mapping",
        ));
    }
    Ok(())
}
