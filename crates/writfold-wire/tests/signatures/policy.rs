use super::*;
#[test]
fn uppercase_policy_hex_preserves_bytes_and_decodes_exact_key() -> Result {
    let original: Value = serde_json::from_slice(&fixture("v3-admission.policy.json")?)?;
    let lower = AuthorityPolicy::new_bootstrap(&bytes(&original)?, limits()?)?;
    let mut upper = original.clone();
    let key = original["trusted_keys"]["reviewer-key"]["public_key"]
        .as_str()
        .ok_or("public key required")?;
    upper["trusted_keys"]["reviewer-key"]["public_key"] = json!(key.to_uppercase());
    let encoded = bytes(&upper)?;
    let upper = AuthorityPolicy::new_bootstrap(&encoded, limits()?)?;
    assert_eq!(lower.trusted_keys(), upper.trusted_keys());
    assert_ne!(lower.identity(), upper.identity());
    assert_eq!(upper.canonical_bytes(), encoded);
    for blank in ["\u{1c}", "\u{1d}", "\u{1e}", "\u{1f}", "\u{85}"] {
        let mut changed = original.clone();
        changed["policy_version"] = json!(blank);
        assert!(AuthorityPolicy::historical(&bytes(&changed)?, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn approval_schema_fields_duplicate_keys_and_actual_private_key_binding() -> Result {
    let (operation, approval, _) = data("v3-admission")?;
    for index in 0..4 {
        let mut value: Value = serde_json::from_slice(&approval.canonical_bytes(4096)?)?;
        match index {
            0 => value["schema_version"] = json!(3.0),
            1 => value["accepted"] = json!(true),
            2 => value["signature"] = json!("ff".repeat(63)),
            _ => value["signature"] = json!("zz".repeat(64)),
        }
        assert!(Approval::decode(&bytes(&value)?, limits()?).is_err());
    }
    let original = String::from_utf8(approval.canonical_bytes(4096)?)?;
    let duplicate = original.replacen('{', "{\"schema_version\":3,", 1);
    assert!(Approval::decode(duplicate.as_bytes(), limits()?).is_err());
    let mut policy: Value = serde_json::from_slice(&fixture("v3-admission.policy.json")?)?;
    policy["trusted_keys"]["reviewer-key"]["public_key"] =
        json!("01".to_owned() + &"00".repeat(31));
    let policy = AuthorityPolicy::historical(&bytes(&policy)?, limits()?)?;
    let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
    changed["policy"] = json!(policy.identity().as_str());
    let changed = Operation::decode(&reidentify(changed)?, limits()?)?;
    let key = ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(&fixture(
        "PUBLIC-TEST-ONLY.pem",
    )?)?)?;
    assert!(
        key.sign(
            &changed,
            &policy,
            changed.actor()?,
            changed.header().toolchain()
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn deeply_nested_untrusted_operation_is_refused_on_a_small_stack() -> Result {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| -> std::result::Result<(), String> {
            let id = "a".repeat(64);
            let nested = format!("{}0{}", "[".repeat(1000), "]".repeat(1000));
            let input = format!(
                r#"{{"schema_version":3,"kind":"admission","id":"{id}","base":null,"policy":"{id}","captures":[],"target_use":"review","plan":{{"id":"{id}","nested":{nested}}},"toolchain":"{id}"}}"#
            );
            let limits = Limits::new(32_768, 1024, 4096, 4096)
                .map_err(|error| error.to_string())?;
            match Operation::decode(input.as_bytes(), limits) {
                Err(writfold_wire::operation::BoundaryError::Binding(_)) => Ok(()),
                other => Err(format!("expected operation identity refusal, got {other:?}")),
            }
        })?
        .join()
        .map_err(|_| "operation decoder thread panicked")?
        .map_err(Into::into)
}
