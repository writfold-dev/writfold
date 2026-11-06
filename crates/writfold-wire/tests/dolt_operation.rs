use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt::Write, path::PathBuf};
use writfold_domain::{SchemaEpoch, dolt::DoltReadPin};
use writfold_wire::{
    Limits, RawJson, canonical,
    dolt_operation::{DoltBase, DoltOperation, DoltTarget},
    operation::Operation,
    policy::AuthorityPolicy,
    signatures::{
        ApprovalSigningKey,
        dolt::{DoltApproval, verify_dolt_current},
    },
};

#[path = "dolt_operation/binding.rs"]
mod binding;
#[path = "dolt_operation/boundaries.rs"]
mod boundaries;
#[path = "dolt_operation/signatures.rs"]
mod signatures;

type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;
fn limits() -> Result<Limits> {
    Ok(Limits::new(1_000_000, 128, 10_000, 100_000)?)
}
fn fixture(directory: &str, name: &str) -> Result<Vec<u8>> {
    Ok(std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(directory)
            .join(name),
    )?)
}
fn inner(name: &str) -> Result<Operation> {
    Ok(Operation::decode(
        &fixture("signatures", &format!("{name}.operation.json"))?,
        limits()?,
    )?)
}
fn wrapped(name: &str) -> Result<DoltOperation> {
    Ok(DoltOperation::decode(
        &fixture("dolt-operation", &format!("{name}.operation.json"))?,
        limits()?,
    )?)
}
fn policy() -> Result<AuthorityPolicy> {
    Ok(AuthorityPolicy::historical(
        &fixture("signatures", "v3-admission.policy.json")?,
        limits()?,
    )?)
}
fn signer() -> Result<ApprovalSigningKey> {
    Ok(ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(
        &fixture("signatures", "PUBLIC-TEST-ONLY.pem")?,
    )?)?)
}
fn target() -> Result<DoltTarget> {
    Ok(DoltTarget::new(
        "test-realm".parse()?,
        "test-kb".parse()?,
        "test_database".parse()?,
        SchemaEpoch::new(
            "epoch-1".parse()?,
            "a".repeat(64).parse()?,
            "b".repeat(64).parse()?,
        ),
        "dolt-corpus-read-v1".parse()?,
        "c".repeat(64).parse()?,
    ))
}
fn native_base() -> Result<DoltBase> {
    let t = target()?;
    Ok(DoltBase::new(
        "a".repeat(40).parse()?,
        DoltReadPin::new(
            t.realm().clone(),
            t.kb().clone(),
            t.database().clone(),
            "0123456789abcdefghijklmnopqrstuv".parse()?,
            t.epoch().clone(),
            t.profile().clone(),
            t.producer().clone(),
        ),
    ))
}
fn bytes(value: &Value) -> Result<Vec<u8>> {
    Ok(canonical(
        &RawJson::parse(&serde_json::to_vec(value)?, limits()?)?,
        1_000_000,
    )?)
}
fn digest(value: &Value, prefix: &[u8]) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(prefix);
    hash.update(bytes(value)?);
    let mut text = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(&mut text, "{byte:02x}")?;
    }
    Ok(text)
}
fn reidentify(value: &mut Value) -> Result<Vec<u8>> {
    value.as_object_mut().ok_or("object required")?.remove("id");
    value["id"] = json!(digest(value, b"writfold-dolt-plan-v1\0")?);
    bytes(value)
}
fn reidentify_inner(value: &mut Value) -> Result<Vec<u8>> {
    value.as_object_mut().ok_or("object required")?.remove("id");
    value["plan"]
        .as_object_mut()
        .ok_or("plan required")?
        .remove("id");
    let id = digest(value, b"")?;
    value["id"] = json!(id);
    value["plan"]["id"] = value["id"].clone();
    bytes(value)
}

// Frozen vectors generated independently with Python json.dumps(sort_keys=True,
// separators=(",", ":"), ensure_ascii=False), hashlib.sha256 and cryptography's
// Ed25519PrivateKey.from_private_bytes(bytes(range(32))). Generation is not qualification.
#[test]
fn exact_canonical_domains_and_real_ed25519_goldens() -> Result {
    let key = signer()?;
    let policy = policy()?;
    for name in ["bootstrap", "selection", "migration"] {
        let decoded = wrapped(name)?;
        let prepared = DoltOperation::prepare(
            decoded.operation().clone(),
            decoded.base().clone(),
            decoded.target().clone(),
            decoded.mapping().clone(),
            limits()?,
        )?;
        assert_eq!(prepared, decoded);
        assert_eq!(
            prepared.canonical_bytes(),
            fixture("dolt-operation", &format!("{name}.operation.json"))?
        );
        assert_eq!(
            prepared.approval_message(),
            fixture("dolt-operation", &format!("{name}.message.bin"))?
        );
        let value: Value = serde_json::from_slice(prepared.canonical_bytes())?;
        assert_eq!(
            DoltOperation::decode(&serde_json::to_vec_pretty(&value)?, limits()?)?,
            prepared
        );
        let approval = key.sign_dolt(
            &prepared,
            &policy,
            prepared.operation().actor()?,
            prepared.operation().header().toolchain(),
            prepared.target().realm(),
        )?;
        let golden = fixture("dolt-operation", &format!("{name}.approval.json"))?;
        assert_eq!(approval.canonical_bytes(golden.len())?, golden);
        assert_eq!(DoltApproval::decode(&golden, limits()?)?, approval);
        assert!(approval.canonical_bytes(golden.len() - 1).is_err());
        let toolchain = prepared.operation().header().toolchain();
        verify_dolt_current(
            &prepared,
            &approval,
            &policy,
            toolchain,
            prepared.target().realm(),
        )?;
        assert_ne!(
            prepared.id().as_str(),
            prepared.operation().header().id().as_str()
        );
    }
    Ok(())
}
