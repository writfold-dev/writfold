use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt::Write, path::PathBuf};
use writfold_domain::{AuthorityKeyId, ContentDigest};
use writfold_wire::{
    Limits, RawJson, canonical,
    operation::{Operation, OperationBody},
    policy::{AuthorityPolicy, PolicyIdentityProfile},
    signatures::{
        Approval, ApprovalSigningKey, HistoricalEquation, inspect_historical, verify_current,
    },
};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

#[path = "signatures/policy.rs"]
mod policy;
fn limits() -> Result<Limits> {
    Ok(Limits::new(1_000_000, 128, 10_000, 100_000)?)
}
fn fixture(name: &str) -> Result<Vec<u8>> {
    Ok(std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/signatures")
            .join(name),
    )?)
}
fn data(prefix: &str) -> Result<(Operation, Approval, AuthorityPolicy)> {
    Ok((
        Operation::decode(&fixture(&format!("{prefix}.operation.json"))?, limits()?)?,
        Approval::decode(&fixture(&format!("{prefix}.approval.json"))?, limits()?)?,
        AuthorityPolicy::historical(&fixture(&format!("{prefix}.policy.json"))?, limits()?)?,
    ))
}
fn bytes(value: &Value) -> Result<Vec<u8>> {
    Ok(canonical(
        &RawJson::parse(&serde_json::to_vec(value)?, limits()?)?,
        1_000_000,
    )?)
}
fn reidentify(mut value: Value) -> Result<Vec<u8>> {
    value.as_object_mut().ok_or("object required")?.remove("id");
    value["plan"]
        .as_object_mut()
        .ok_or("plan required")?
        .remove("id");
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(bytes(&value)?) {
        write!(&mut digest, "{byte:02x}")?;
    }
    value["id"] = json!(digest);
    value["plan"]["id"] = value["id"].clone();
    bytes(&value)
}

#[test]
fn python_golden_four_variants_have_exact_ids_messages_and_signatures() -> Result {
    let key = ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(&fixture(
        "PUBLIC-TEST-ONLY.pem",
    )?)?)?;
    for prefix in [
        "v2-admission",
        "v2-selection",
        "v3-admission",
        "v3-selection",
    ] {
        let (operation, approval, policy) = data(prefix)?;
        assert_eq!(
            operation.canonical_bytes(),
            fixture(&format!("{prefix}.operation.json"))?
        );
        assert_eq!(
            operation.approval_message(),
            fixture(&format!("{prefix}.message.bin"))?
        );
        assert_eq!(
            approval.canonical_bytes(4096)?,
            fixture(&format!("{prefix}.approval.json"))?
        );
        assert_eq!(
            inspect_historical(&operation, &approval, &policy)?.equation(),
            HistoricalEquation::StrongStrict
        );
        let signed = key.sign(
            &operation,
            &policy,
            operation.actor()?,
            operation.header().toolchain(),
        );
        if operation.schema_version() == 3 {
            assert_eq!(signed?, approval);
            let bound = verify_current(
                &operation,
                &approval,
                &policy,
                operation.header().toolchain(),
            )?;
            assert!(std::ptr::eq(bound.operation(), &operation));
            assert!(std::ptr::eq(bound.policy(), &policy));
        } else {
            assert!(signed.is_err());
            assert!(
                verify_current(
                    &operation,
                    &approval,
                    &policy,
                    operation.header().toolchain()
                )
                .is_err()
            );
        }
    }
    Ok(())
}

#[test]
fn weak_historical_equation_is_inspectable_but_never_current_authority() -> Result {
    for prefix in ["weak-v2-admission", "weak-v3-admission"] {
        let (operation, approval, policy) = data(prefix)?;
        assert_eq!(
            inspect_historical(&operation, &approval, &policy)?.equation(),
            HistoricalEquation::WeakOrNonStrict
        );
        assert!(
            verify_current(
                &operation,
                &approval,
                &policy,
                operation.header().toolchain()
            )
            .is_err()
        );
        assert!(AuthorityPolicy::new_bootstrap(policy.original_bytes(), limits()?).is_err());
        let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
        changed["toolchain"] = json!("f".repeat(64));
        let changed = Operation::decode(&reidentify(changed)?, limits()?)?;
        let mut envelope: Value = serde_json::from_slice(&approval.canonical_bytes(4096)?)?;
        envelope["plan"] = json!(changed.header().id().as_str());
        let weak = Approval::decode(&bytes(&envelope)?, limits()?)?;
        assert_eq!(
            inspect_historical(&changed, &weak, &policy)?.equation(),
            HistoricalEquation::WeakOrNonStrict
        );
    }
    Ok(())
}

#[test]
fn mismatched_domain_version_actor_plan_and_toolchain_are_refused() -> Result {
    for prefix in [
        "v2-admission",
        "v2-selection",
        "v3-admission",
        "v3-selection",
    ] {
        let (operation, approval, policy) = data(prefix)?;
        let wrong = Approval::decode(&fixture(&format!("{prefix}.wrong-domain.json"))?, limits()?)?;
        assert_eq!(
            inspect_historical(&operation, &wrong, &policy)?.equation(),
            HistoricalEquation::Rejected
        );
        for (name, replacement) in [
            ("schema_version", json!(5 - approval.schema_version())),
            ("key", json!("different-key")),
            ("plan", json!("f".repeat(64))),
        ] {
            let mut envelope: Value = serde_json::from_slice(&approval.canonical_bytes(4096)?)?;
            envelope[name] = replacement;
            let wrong = Approval::decode(&bytes(&envelope)?, limits()?)?;
            assert!(inspect_historical(&operation, &wrong, &policy).is_err());
        }
        let stale = ContentDigest::try_from("f".repeat(64))?;
        assert!(verify_current(&operation, &approval, &policy, &stale).is_err());
    }
    Ok(())
}

#[test]
fn inner_outer_identity_payload_and_schema_mutations_are_refused() -> Result {
    let (operation, approval, policy) = data("v3-admission")?;
    for path in ["id", "plan", "target_use", "captures"] {
        let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
        match path {
            "id" => changed["id"] = json!("f".repeat(64)),
            "plan" => changed["plan"]["id"] = json!("f".repeat(64)),
            "target_use" => changed["target_use"] = json!("audit"),
            _ => changed["captures"]
                .as_array_mut()
                .ok_or("array required")?
                .reverse(),
        }
        assert!(Operation::decode(&bytes(&changed)?, limits()?).is_err());
    }
    let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
    changed["plan"]["contexts"][0]["context"]["now"] = json!(6);
    let changed = Operation::decode(&reidentify(changed)?, limits()?)?;
    let mut envelope: Value = serde_json::from_slice(&approval.canonical_bytes(4096)?)?;
    envelope["plan"] = json!(changed.header().id().as_str());
    let changed_approval = Approval::decode(&bytes(&envelope)?, limits()?)?;
    assert!(
        verify_current(
            &changed,
            &changed_approval,
            &policy,
            changed.header().toolchain()
        )
        .is_err()
    );
    for field in ["extra", "schema_version"] {
        let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
        changed[field] = json!(2.0);
        assert!(Operation::decode(&bytes(&changed)?, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn every_context_use_and_exact_actor_projection_are_checked() -> Result {
    let (operation, _, policy) = data("v3-admission")?;
    let key = ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(&fixture(
        "PUBLIC-TEST-ONLY.pem",
    )?)?)?;
    for index in 0..5 {
        let mut changed: Value = serde_json::from_slice(operation.canonical_bytes())?;
        match index {
            0 => changed["target_use"] = json!("absent"),
            1 => changed["plan"]["contexts"][1]["context"]["intended_use"] = json!("deploy"),
            2 => {
                changed["plan"]["contexts"][1]["context"]["policy"]["authorized_actors"] = json!([])
            }
            3 => {
                changed["plan"]["contexts"][1]["context"]["policy"]["authorized_actors"] =
                    json!(["reviewer-key", "forged-key"])
            }
            _ => changed["plan"]["contexts"][1]["context"]["policy"]["id"] = json!("other-policy"),
        }
        let changed = Operation::decode(&reidentify(changed)?, limits()?)?;
        assert!(
            key.sign(
                &changed,
                &policy,
                changed.actor()?,
                changed.header().toolchain()
            )
            .is_err()
        );
    }
    assert!(
        key.sign(
            &operation,
            &policy,
            "other-key".parse::<AuthorityKeyId>()?,
            operation.header().toolchain()
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn policy_original_and_bootstrap_identities_never_silently_reanchor() -> Result {
    let original = fixture("v3-admission.policy.json")?;
    let value: Value = serde_json::from_slice(&original)?;
    let spaced = serde_json::to_vec_pretty(&value)?;
    let historical = AuthorityPolicy::historical(&spaced, limits()?)?;
    let bootstrap = AuthorityPolicy::new_bootstrap(&spaced, limits()?)?;
    assert_eq!(
        historical.profile(),
        PolicyIdentityProfile::HistoricalOriginalBytes
    );
    assert_ne!(historical.identity(), bootstrap.identity());
    assert_eq!(historical.original_digest(), bootstrap.original_digest());
    assert_eq!(bootstrap.original_bytes(), spaced);
    assert_eq!(bootstrap.canonical_bytes(), original);
    let (operation, approval, _) = data("v3-admission")?;
    assert!(
        verify_current(
            &operation,
            &approval,
            &historical,
            operation.header().toolchain()
        )
        .is_err()
    );
    verify_current(
        &operation,
        &approval,
        &bootstrap,
        operation.header().toolchain(),
    )?;
    for name in ["kb_id", "policy_version"] {
        let mut opaque = value.clone();
        opaque[name] = json!(" historical opaque identity ");
        let encoded = bytes(&opaque)?;
        let history = AuthorityPolicy::historical(&encoded, limits()?)?;
        assert_eq!(history.original_bytes(), encoded);
        assert!(AuthorityPolicy::new_bootstrap(&encoded, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn strict_policy_approval_keys_hex_and_opaque_uses() -> Result {
    let original: Value = serde_json::from_slice(&fixture("v3-admission.policy.json")?)?;
    for index in 0..8 {
        let mut changed = original.clone();
        match index {
            0 => changed["schema_version"] = json!(2.0),
            1 => changed["kb_id"] = json!(" \t"),
            2 => changed["trusted_keys"]["reviewer-key"]["target_uses"] = json!([]),
            3 => {
                changed["trusted_keys"]["reviewer-key"]["target_uses"] = json!(["review", "review"])
            }
            4 => changed["trusted_keys"]["reviewer-key"]["target_uses"] = json!([" "]),
            5 => changed["trusted_keys"]["reviewer-key"]["public_key"] = json!("00".repeat(31)),
            6 => changed["trusted_keys"]["reviewer-key"]["public_key"] = json!("gg".repeat(32)),
            _ => changed["trusted_keys"]["reviewer-key"]["revoked"] = json!(false),
        }
        assert!(AuthorityPolicy::historical(&bytes(&changed)?, limits()?).is_err());
    }
    let mut changed = original.clone();
    changed["trusted_keys"]["reviewer-key"]["target_uses"] = json!([" review "]);
    let policy = AuthorityPolicy::new_bootstrap(&bytes(&changed)?, limits()?)?;
    assert!(policy.authorized_actors("review")?.is_empty());
    assert_eq!(policy.authorized_actors(" review ")?.len(), 1);
    let (_, approval, _) = data("v3-admission")?;
    let mut envelope: Value = serde_json::from_slice(&approval.canonical_bytes(4096)?)?;
    envelope["signature"] = json!(
        envelope["signature"]
            .as_str()
            .ok_or("string required")?
            .to_uppercase()
    );
    assert_eq!(Approval::decode(&bytes(&envelope)?, limits()?)?, approval);
    envelope["signature"] = json!("00 ".repeat(64));
    assert!(Approval::decode(&bytes(&envelope)?, limits()?).is_err());
    Ok(())
}

#[test]
fn preparation_reproduces_canonical_ids_without_mutating_prepared_values() -> Result {
    for prefix in ["v3-admission", "v3-selection"] {
        let (operation, _, _) = data(prefix)?;
        let prepared = match operation.body() {
            OperationBody::PortfolioAdmission(v) => Operation::prepare_admission(
                v.base().clone(),
                v.header().policy().clone(),
                v.captures().clone(),
                v.target_use().clone(),
                v.semantic().clone(),
                v.header().toolchain().clone(),
                limits()?,
            )?,
            OperationBody::PortfolioSelection(v) => Operation::prepare_selection(
                v.base().clone(),
                v.retained().clone(),
                v.header().policy().clone(),
                v.target_use().clone(),
                v.semantic().clone(),
                v.header().toolchain().clone(),
                limits()?,
            )?,
            _ => return Err("expected portfolio fixture".into()),
        };
        assert_eq!(prepared, operation);
        assert_eq!(
            operation.canonical_bytes(),
            fixture(&format!("{prefix}.operation.json"))?
        );
    }
    assert!(ApprovalSigningKey::from_pkcs8_pem("not a key").is_err());
    assert!(
        ApprovalSigningKey::from_pkcs8_pem(
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\nAA==\n-----END ENCRYPTED PRIVATE KEY-----"
        )
        .is_err()
    );
    Ok(())
}
