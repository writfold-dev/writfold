use super::*;

#[test]
fn bootstrap_and_exact_embedded_base_are_required() -> Result {
    let bootstrap = inner("v3-admission")?;
    assert!(
        DoltOperation::prepare(
            bootstrap.clone(),
            Some(native_base()?),
            target()?,
            None,
            limits()?
        )
        .is_err()
    );
    assert!(
        DoltOperation::prepare(
            bootstrap,
            None,
            target()?,
            Some("d".repeat(64).parse()?),
            limits()?
        )
        .is_err()
    );
    let selection = wrapped("selection")?;
    assert!(
        DoltOperation::prepare(
            selection.operation().clone(),
            None,
            target()?,
            None,
            limits()?
        )
        .is_err()
    );
    let base = native_base()?;
    let wrong = DoltBase::new("e".repeat(40).parse()?, base.pin().clone());
    assert!(
        DoltOperation::prepare(
            selection.operation().clone(),
            Some(wrong),
            target()?,
            None,
            limits()?
        )
        .is_err()
    );
    for name in ["v2-admission", "v2-selection"] {
        assert!(DoltOperation::prepare(inner(name)?, None, target()?, None, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn mapping_is_required_for_database_epoch_schema_query_and_profile_only() -> Result {
    let original = wrapped("selection")?;
    for (field, replacement) in [
        ("database", json!("next_database")),
        ("epoch", json!("epoch-2")),
        ("schema_digest", json!("e".repeat(64))),
        ("query_digest", json!("e".repeat(64))),
        ("storage_profile", json!("other-profile")),
        ("producer", json!("e".repeat(64))),
    ] {
        let mut value: Value = serde_json::from_slice(original.canonical_bytes())?;
        value["target"][field] = replacement;
        let without_mapping = DoltOperation::decode(&reidentify(&mut value)?, limits()?);
        assert_eq!(without_mapping.is_ok(), field == "producer", "{field}");
        value["mapping"] = json!("d".repeat(64));
        let decoded = DoltOperation::decode(&reidentify(&mut value)?, limits()?)?;
        assert_ne!(decoded.id(), original.id());
        assert_eq!(
            DoltOperation::prepare(
                decoded.operation().clone(),
                decoded.base().clone(),
                decoded.target().clone(),
                decoded.mapping().clone(),
                limits()?
            )?,
            decoded
        );
    }
    for field in ["realm", "kb"] {
        for mapping in [Value::Null, json!("d".repeat(64))] {
            let mut value: Value = serde_json::from_slice(original.canonical_bytes())?;
            value["target"][field] = json!("other-identity");
            value["mapping"] = mapping;
            assert!(DoltOperation::decode(&reidentify(&mut value)?, limits()?).is_err());
        }
    }
    Ok(())
}

#[test]
fn expected_realm_is_required_for_signing_and_verification() -> Result {
    let operation = wrapped("bootstrap")?;
    let policy = policy()?;
    let key = signer()?;
    let toolchain = operation.operation().header().toolchain();
    let realm = operation.target().realm();
    let approval = key.sign_dolt(
        &operation,
        &policy,
        "reviewer-key".parse()?,
        toolchain,
        realm,
    )?;
    let other = "other-realm".parse()?;
    assert!(
        key.sign_dolt(
            &operation,
            &policy,
            "reviewer-key".parse()?,
            toolchain,
            &other
        )
        .is_err()
    );
    assert!(verify_dolt_current(&operation, &approval, &policy, toolchain, &other).is_err());
    let supplied = verify_dolt_current(&operation, &approval, &policy, toolchain, realm)?;
    assert!(std::ptr::eq(supplied.realm(), realm));
    Ok(())
}

#[test]
fn old_storage_domains_and_pins_are_not_reinterpreted() -> Result {
    assert!(
        DoltOperation::decode(
            &fixture("storage-operation", "selection.operation.json")?,
            limits()?
        )
        .is_err()
    );
    assert!(
        DoltApproval::decode(
            &fixture("storage-operation", "selection.approval.json")?,
            limits()?
        )
        .is_err()
    );
    let original = wrapped("selection")?;
    for name in ["selection", "legacy-selection"] {
        let old: Value = serde_json::from_slice(&fixture(
            "storage-operation",
            &format!("{name}.operation.json"),
        )?)?;
        let mut value: Value = serde_json::from_slice(original.canonical_bytes())?;
        value["base"]["pin"] = old["base"]["pin"].clone();
        assert!(DoltOperation::decode(&reidentify(&mut value)?, limits()?).is_err());
    }
    let old: Value =
        serde_json::from_slice(&fixture("storage-operation", "selection.approval.json")?)?;
    let mut approval: Value =
        serde_json::from_slice(&fixture("dolt-operation", "selection.approval.json")?)?;
    approval["signature"] = old["signature"].clone();
    let approval = DoltApproval::decode(&bytes(&approval)?, limits()?)?;
    assert!(
        verify_dolt_current(
            &original,
            &approval,
            &policy()?,
            original.operation().header().toolchain(),
            original.target().realm()
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn deep_unknown_fields_remain_iteratively_owned_on_error() -> Result {
    let operation = wrapped("selection")?;
    let approval = String::from_utf8(fixture("dolt-operation", "selection.approval.json")?)?;
    let deep = format!("{}0{}", "[".repeat(512), "]".repeat(512));
    let original = String::from_utf8(operation.canonical_bytes().to_vec())?;
    let mut malicious = Vec::new();
    for needle in ["{\"base\":", "{\"pin\":", "{\"backend\":", "{\"commit\":"] {
        malicious.push(original.replacen(
            needle,
            &format!("{{\"unknown\":{deep},{}", &needle[1..]),
            1,
        ));
    }
    let bad_approval = approval.replacen("{", &format!("{{\"unknown\":{deep},"), 1);
    let generous = Limits::new(1_000_000, 1024, 10_000, 100_000)?;
    let refused = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            malicious
                .iter()
                .all(|input| DoltOperation::decode(input.as_bytes(), generous).is_err())
                && DoltApproval::decode(bad_approval.as_bytes(), generous).is_err()
        })?
        .join()
        .map_err(|_| "deep unknown field exhausted native stack")?;
    assert!(refused);
    Ok(())
}
