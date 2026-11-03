use super::{Result, StorageBase, StorageOperationError, StorageTarget};
use crate::operation::{Operation, OperationBody};
use writfold_domain::{PlanDigest, StoragePin};

pub(super) fn check(
    operation: &Operation,
    base: Option<&StorageBase>,
    target: &StorageTarget,
    mapping: Option<&PlanDigest>,
) -> Result<()> {
    let semantic_base = match operation.body() {
        OperationBody::PortfolioAdmission(plan) => plan.base().as_ref(),
        OperationBody::PortfolioSelection(plan) => Some(plan.base()),
        _ => return Err(StorageOperationError::UnsupportedOperation),
    };
    if semantic_base != base.map(StorageBase::release) {
        return Err(StorageOperationError::Binding(
            "embedded base release differs",
        ));
    }
    let Some(base) = base else {
        return if mapping.is_some() {
            Err(StorageOperationError::Binding(
                "bootstrap cannot have a mapping",
            ))
        } else {
            Ok(())
        };
    };
    let (kb, needs_mapping) = match base.pin() {
        StoragePin::LegacyDolt(pin) => (pin.kb(), true),
        StoragePin::Omnigraph(pin) => (
            pin.kb(),
            pin.graph() != target.graph()
                || pin.epoch() != target.epoch()
                || pin.profile() != target.profile()
                || pin.native_format() != target.native_format(),
        ),
    };
    if kb != target.kb() {
        return Err(StorageOperationError::Binding("base and target KB differ"));
    }
    // Producing-tool changes do not themselves imply a data transformation.
    if needs_mapping && mapping.is_none() {
        return Err(StorageOperationError::Binding(
            "storage transition requires a mapping",
        ));
    }
    Ok(())
}
