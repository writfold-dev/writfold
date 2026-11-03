use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt::Write, path::PathBuf};
use writfold_domain::{NativeFormatVersion, OmnigraphPin, SchemaEpoch, StoragePin};
use writfold_wire::{
    Limits, RawJson, canonical,
    operation::{Operation, OperationBody},
    policy::AuthorityPolicy,
    signatures::{
        ApprovalSigningKey,
        storage::{StorageApproval, verify_storage_current},
    },
    storage_operation::{StorageBase, StorageOperation, StorageTarget},
};

#[path = "storage_operation/binding.rs"]
mod binding;
#[path = "storage_operation/boundaries.rs"]
mod boundaries;
#[path = "storage_operation/signatures.rs"]
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
fn wrapped(name: &str) -> Result<StorageOperation> {
    Ok(StorageOperation::decode(
        &fixture("storage-operation", &format!("{name}.operation.json"))?,
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
fn target() -> Result<StorageTarget> {
    Ok(StorageTarget::new(
        "test-kb".parse()?,
        "Graph-01".parse()?,
        SchemaEpoch::new(
            "epoch-1".parse()?,
            "a".repeat(64).parse()?,
            "b".repeat(64).parse()?,
        ),
        "reader-1".parse()?,
        NativeFormatVersion::new(1)?,
        "c".repeat(64).parse()?,
    ))
}
fn native_base() -> Result<StorageBase> {
    let t = target()?;
    Ok(StorageBase::new(
        "a".repeat(40).parse()?,
        StoragePin::Omnigraph(OmnigraphPin::new(
            t.kb().clone(),
            t.graph().clone(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse()?,
            t.epoch().clone(),
            t.profile().clone(),
            *t.native_format(),
            t.producer().clone(),
        )),
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
    value["id"] = json!(digest(value, b"writfold-storage-plan-v1\0")?);
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
    for name in ["bootstrap", "selection", "legacy-selection"] {
        let decoded = wrapped(name)?;
        let prepared = StorageOperation::prepare(
            decoded.operation().clone(),
            decoded.base().clone(),
            target()?,
            decoded.mapping().clone(),
            limits()?,
        )?;
        assert_eq!(prepared, decoded);
        assert_eq!(
            prepared.canonical_bytes(),
            fixture("storage-operation", &format!("{name}.operation.json"))?
        );
        assert_eq!(
            prepared.approval_message(),
            fixture("storage-operation", &format!("{name}.message.bin"))?
        );
        let value: Value = serde_json::from_slice(prepared.canonical_bytes())?;
        assert_eq!(
            StorageOperation::decode(&serde_json::to_vec_pretty(&value)?, limits()?)?,
            prepared
        );
        let approval = key.sign_storage(
            &prepared,
            &policy,
            prepared.operation().actor()?,
            prepared.operation().header().toolchain(),
        )?;
        let golden = fixture("storage-operation", &format!("{name}.approval.json"))?;
        assert_eq!(approval.canonical_bytes(golden.len())?, golden);
        assert_eq!(StorageApproval::decode(&golden, limits()?)?, approval);
        assert!(approval.canonical_bytes(golden.len() - 1).is_err());
        let toolchain = prepared.operation().header().toolchain();
        let authorization = verify_storage_current(&prepared, &approval, &policy, toolchain)?;
        assert!(std::ptr::eq(authorization.operation(), &prepared));
        assert!(std::ptr::eq(authorization.approval(), &approval));
        assert!(std::ptr::eq(authorization.policy(), &policy));
        assert!(std::ptr::eq(authorization.toolchain(), toolchain));
        assert_ne!(
            prepared.id().as_str(),
            prepared.operation().header().id().as_str()
        );
    }
    Ok(())
}
