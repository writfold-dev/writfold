use super::*;
use writfold_domain::LegacyDoltPin;
use writfold_wire::{
    storage::{decode_pin, encode_pin},
    storage_operation::StorageOperationError,
};

#[test]
fn exact_base_release_presence_and_bootstrap_mapping_are_required() -> Result {
    let admission = inner("v3-admission")?;
    let selection = inner("v3-selection")?;
    let base = native_base()?;
    let mapping = Some("d".repeat(64).parse()?);
    assert!(
        StorageOperation::prepare(
            admission.clone(),
            Some(base.clone()),
            target()?,
            None,
            limits()?
        )
        .is_err()
    );
    assert!(
        StorageOperation::prepare(admission.clone(), None, target()?, mapping, limits()?).is_err()
    );
    assert!(
        StorageOperation::prepare(selection.clone(), None, target()?, None, limits()?).is_err()
    );
    let wrong = StorageBase::new("b".repeat(40).parse()?, base.pin().clone());
    assert!(
        StorageOperation::prepare(selection.clone(), Some(wrong), target()?, None, limits()?)
            .is_err()
    );
    let OperationBody::PortfolioAdmission(plan) = admission.body() else {
        return Err("admission required".into());
    };
    let admission = Operation::prepare_admission(
        Some(base.release().clone()),
        plan.header().policy().clone(),
        plan.captures().clone(),
        plan.target_use().clone(),
        plan.semantic().clone(),
        plan.header().toolchain().clone(),
        limits()?,
    )?;
    assert!(StorageOperation::prepare(admission, Some(base), target()?, None, limits()?).is_ok());
    Ok(())
}

#[test]
fn every_storage_transition_dimension_requires_mapping_except_producer_and_commit() -> Result {
    let operation = inner("v3-selection")?;
    let base = native_base()?;
    for (field, value) in [
        ("graph", json!("Graph-02")),
        ("epoch", json!("epoch-2")),
        ("schema_digest", json!("e".repeat(64))),
        ("query_digest", json!("e".repeat(64))),
        ("storage_profile", json!("reader-2")),
        ("native_format", json!(2)),
    ] {
        let mut pin: Value = serde_json::from_slice(&encode_pin(base.pin(), 4096)?)?;
        pin[field] = value;
        let changed = StorageBase::new(
            base.release().clone(),
            decode_pin(&bytes(&pin)?, limits()?)?,
        );
        assert!(matches!(
            StorageOperation::prepare(
                operation.clone(),
                Some(changed.clone()),
                target()?,
                None,
                limits()?
            ),
            Err(StorageOperationError::Binding(
                "storage transition requires a mapping"
            ))
        ));
        let mapped = StorageOperation::prepare(
            operation.clone(),
            Some(changed),
            target()?,
            Some("d".repeat(64).parse()?),
            limits()?,
        )?;
        assert_eq!(
            StorageOperation::decode(mapped.canonical_bytes(), limits()?)?,
            mapped
        );
    }
    let original = wrapped("selection")?;
    for (field, value) in [
        ("producer_generation", json!("e".repeat(64))),
        ("commit", json!("7ZZZZZZZZZZZZZZZZZZZZZZZZZ")),
    ] {
        let mut pin: Value = serde_json::from_slice(&encode_pin(base.pin(), 4096)?)?;
        pin[field] = value;
        let changed = StorageBase::new(
            base.release().clone(),
            decode_pin(&bytes(&pin)?, limits()?)?,
        );
        let prepared = StorageOperation::prepare(
            operation.clone(),
            Some(changed),
            target()?,
            None,
            limits()?,
        )?;
        assert_ne!(original.id(), prepared.id());
    }
    assert!(
        StorageOperation::prepare(
            operation,
            Some(base),
            target()?,
            Some("d".repeat(64).parse()?),
            limits()?
        )
        .is_ok()
    );
    Ok(())
}

#[test]
fn legacy_base_requires_mapping_and_no_mapping_can_cross_kb() -> Result {
    let operation = inner("v3-selection")?;
    for kb in ["test-kb", "other-kb"] {
        let legacy = StorageBase::new(
            "a".repeat(40).parse()?,
            StoragePin::LegacyDolt(LegacyDoltPin::new(
                kb.parse()?,
                "0123456789abcdefghijklmnopqrstuv".parse()?,
            )),
        );
        assert!(
            StorageOperation::prepare(
                operation.clone(),
                Some(legacy.clone()),
                target()?,
                None,
                limits()?
            )
            .is_err()
        );
        let mapped = StorageOperation::prepare(
            operation.clone(),
            Some(legacy),
            target()?,
            Some("d".repeat(64).parse()?),
            limits()?,
        );
        assert_eq!(mapped.is_ok(), kb == "test-kb");
    }
    let mut value: Value = serde_json::from_slice(wrapped("selection")?.canonical_bytes())?;
    value["base"]["pin"]["kb"] = json!("other-kb");
    value["mapping"] = json!("d".repeat(64));
    assert!(StorageOperation::decode(&reidentify(&mut value)?, limits()?).is_err());
    Ok(())
}

#[test]
fn legacy_semantic_operations_cannot_be_wrapped_even_with_fresh_identity() -> Result {
    for name in ["v2-admission", "v2-selection"] {
        let operation = inner(name)?;
        let base = if name.ends_with("selection") {
            Some(native_base()?)
        } else {
            None
        };
        assert!(matches!(
            StorageOperation::prepare(operation.clone(), base, target()?, None, limits()?),
            Err(StorageOperationError::UnsupportedOperation)
        ));
        let mut value: Value = serde_json::from_slice(wrapped("bootstrap")?.canonical_bytes())?;
        value["operation"] = serde_json::from_slice(operation.canonical_bytes())?;
        assert!(matches!(
            StorageOperation::decode(&reidentify(&mut value)?, limits()?),
            Err(StorageOperationError::UnsupportedOperation)
        ));
    }
    Ok(())
}
