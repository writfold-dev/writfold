use super::*;
use writfold_wire::operation::BoundaryError;

fn approval_value(operation: &StorageOperation, signature: Value) -> Value {
    json!({"namespace":"writfold-storage-approval", "schema":1,
        "plan":operation.id().as_str(), "key":"reviewer-key", "signature":signature})
}

#[test]
fn old_domains_and_detached_workflow_approvals_never_authorize_storage() -> Result {
    let operation = wrapped("bootstrap")?;
    let policy = policy()?;
    for name in ["v2-admission", "v3-admission", "v3-admission.wrong-domain"] {
        let path = if name.ends_with("wrong-domain") {
            format!("{name}.json")
        } else {
            format!("{name}.approval.json")
        };
        let old = fixture("signatures", &path)?;
        assert!(StorageApproval::decode(&old, limits()?).is_err());
        let old: Value = serde_json::from_slice(&old)?;
        let transplanted = StorageApproval::decode(
            &bytes(&approval_value(&operation, old["signature"].clone()))?,
            limits()?,
        )?;
        assert!(matches!(
            verify_storage_current(
                &operation,
                &transplanted,
                &policy,
                operation.operation().header().toolchain()
            ),
            Err(BoundaryError::Signature)
        ));
    }
    Ok(())
}

#[test]
fn altered_target_pin_mapping_release_and_operation_cannot_reuse_signature() -> Result {
    let original = wrapped("selection")?;
    let approval: Value =
        serde_json::from_slice(&fixture("storage-operation", "selection.approval.json")?)?;
    let policy = policy()?;
    for mutation in 0..7 {
        let mut value: Value = serde_json::from_slice(original.canonical_bytes())?;
        match mutation {
            0 => value["target"]["producer_generation"] = json!("e".repeat(64)),
            1 => {
                value["target"]["graph"] = json!("Graph-02");
                value["mapping"] = json!("d".repeat(64));
            }
            2 => value["base"]["pin"]["commit"] = json!("7ZZZZZZZZZZZZZZZZZZZZZZZZZ"),
            3 => value["mapping"] = json!("d".repeat(64)),
            4 => value["operation"]["plan"]["contexts"][0]["context"]["now"] = json!(6),
            5 => {
                value["base"]["release"] = json!("e".repeat(40));
                value["operation"]["base"] = json!("e".repeat(40));
            }
            _ => value["operation"]["retained"] = json!("e".repeat(40)),
        }
        assert!(StorageOperation::decode(&bytes(&value)?, limits()?).is_err());
        if mutation >= 4 {
            reidentify_inner(&mut value["operation"])?;
        }
        let changed = StorageOperation::decode(&reidentify(&mut value)?, limits()?)?;
        let replay = StorageApproval::decode(
            &bytes(&approval_value(&changed, approval["signature"].clone()))?,
            limits()?,
        )?;
        assert!(matches!(
            verify_storage_current(
                &changed,
                &replay,
                &policy,
                changed.operation().header().toolchain()
            ),
            Err(BoundaryError::Signature)
        ));
    }
    Ok(())
}

#[test]
fn wrong_policy_toolchain_kb_actor_and_private_key_refuse() -> Result {
    let operation = wrapped("bootstrap")?;
    let policy = policy()?;
    let key = signer()?;
    let toolchain = operation.operation().header().toolchain();
    let approval = key.sign_storage(&operation, &policy, "reviewer-key".parse()?, toolchain)?;
    let stale = "f".repeat(64).parse()?;
    assert!(verify_storage_current(&operation, &approval, &policy, &stale).is_err());
    assert!(
        key.sign_storage(&operation, &policy, "reviewer-key".parse()?, &stale)
            .is_err()
    );
    assert!(
        key.sign_storage(&operation, &policy, "other-key".parse()?, toolchain)
            .is_err()
    );
    let mut approval_json: Value = serde_json::from_slice(&approval.canonical_bytes(4096)?)?;
    for (field, value) in [("key", json!("other-key")), ("plan", json!("f".repeat(64)))] {
        let mut altered = approval_json.clone();
        altered[field] = value;
        let altered = StorageApproval::decode(&bytes(&altered)?, limits()?)?;
        assert!(verify_storage_current(&operation, &altered, &policy, toolchain).is_err());
    }
    let policy_json: Value = serde_json::from_slice(policy.original_bytes())?;
    let stale_policy =
        AuthorityPolicy::historical(&serde_json::to_vec_pretty(&policy_json)?, limits()?)?;
    assert!(verify_storage_current(&operation, &approval, &stale_policy, toolchain).is_err());
    let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
    changed["target"]["kb"] = json!("other-kb");
    let changed = StorageOperation::decode(&reidentify(&mut changed)?, limits()?)?;
    approval_json["plan"] = json!(changed.id().as_str());
    let replay = StorageApproval::decode(&bytes(&approval_json)?, limits()?)?;
    assert!(
        key.sign_storage(&changed, &policy, "reviewer-key".parse()?, toolchain)
            .is_err()
    );
    assert!(verify_storage_current(&changed, &replay, &policy, toolchain).is_err());
    let pem =
        String::from_utf8(fixture("signatures", "PUBLIC-TEST-ONLY.pem")?)?.replace("HR4f", "HR4e");
    let wrong_key = ApprovalSigningKey::from_pkcs8_pem(&pem)?;
    assert!(matches!(
        wrong_key.sign_storage(&operation, &policy, "reviewer-key".parse()?, toolchain),
        Err(BoundaryError::Binding(_))
    ));
    Ok(())
}

#[test]
fn all_contexts_current_policy_and_key_use_are_still_required() -> Result {
    let key = signer()?;
    let policy = policy()?;
    for mutation in 0..5 {
        let mut value: Value = serde_json::from_slice(inner("v3-admission")?.canonical_bytes())?;
        match mutation {
            0 => value["target_use"] = json!("absent"),
            1 => value["plan"]["contexts"][1]["context"]["intended_use"] = json!("deploy"),
            2 => value["plan"]["contexts"][1]["context"]["policy"]["authorized_actors"] = json!([]),
            3 => {
                value["plan"]["contexts"][1]["context"]["policy"]["authorized_actors"] =
                    json!(["reviewer-key", "forged-key"])
            }
            _ => value["plan"]["contexts"][1]["context"]["policy"]["id"] = json!("other-policy"),
        }
        let operation = Operation::decode(&reidentify_inner(&mut value)?, limits()?)?;
        let wrapped = StorageOperation::prepare(operation, None, target()?, None, limits()?)?;
        assert!(
            key.sign_storage(
                &wrapped,
                &policy,
                "reviewer-key".parse()?,
                wrapped.operation().header().toolchain()
            )
            .is_err()
        );
        let signature = json!("00".repeat(64));
        let approval =
            StorageApproval::decode(&bytes(&approval_value(&wrapped, signature))?, limits()?)?;
        assert!(matches!(
            verify_storage_current(
                &wrapped,
                &approval,
                &policy,
                wrapped.operation().header().toolchain()
            ),
            Err(BoundaryError::Binding(_))
        ));
    }
    Ok(())
}

#[test]
fn weak_historical_authority_never_gains_storage_authorization() -> Result {
    let operation = StorageOperation::prepare(
        inner("weak-v3-admission")?,
        None,
        target()?,
        None,
        limits()?,
    )?;
    let policy = AuthorityPolicy::historical(
        &fixture("signatures", "weak-v3-admission.policy.json")?,
        limits()?,
    )?;
    let approval = StorageApproval::decode(
        &bytes(&approval_value(
            &operation,
            json!(format!("01{}", "00".repeat(63))),
        ))?,
        limits()?,
    )?;
    assert!(matches!(
        verify_storage_current(
            &operation,
            &approval,
            &policy,
            operation.operation().header().toolchain()
        ),
        Err(BoundaryError::WeakAuthority)
    ));
    assert!(matches!(
        signer()?.sign_storage(
            &operation,
            &policy,
            "reviewer-key".parse()?,
            operation.operation().header().toolchain()
        ),
        Err(BoundaryError::WeakAuthority)
    ));
    Ok(())
}

#[test]
fn altered_signature_bytes_and_noncanonical_scalar_are_strictly_rejected() -> Result {
    let operation = wrapped("bootstrap")?;
    let policy = policy()?;
    let original = StorageApproval::decode(
        &fixture("storage-operation", "bootstrap.approval.json")?,
        limits()?,
    )?;
    for scalar in [false, true] {
        let mut signature = *original.signature();
        if scalar {
            signature[63] |= 0x80;
        } else {
            signature[0] ^= 1;
        }
        let mut hex = String::with_capacity(128);
        for byte in signature {
            write!(&mut hex, "{byte:02x}")?;
        }
        let altered =
            StorageApproval::decode(&bytes(&approval_value(&operation, json!(hex)))?, limits()?)?;
        assert!(matches!(
            verify_storage_current(
                &operation,
                &altered,
                &policy,
                operation.operation().header().toolchain()
            ),
            Err(BoundaryError::Signature)
        ));
    }
    Ok(())
}
