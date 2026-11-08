//! Operator grants and revocations: exact round trip, strict verification,
//! distinct signing domains and target uses, and refusal of transplanted,
//! tampered, noncanonical and mis-bound records.
use ed25519_dalek::{Signer as _, SigningKey, pkcs8::DecodePrivateKey};
use serde_json::json;
use std::path::PathBuf;
use writfold_domain::engineering::approval::{Signer, Validity};
use writfold_domain::engineering::execution::Capability;
use writfold_domain::engineering::grant::{Grant, Revocation};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_domain::worktree::Scope;
use writfold_domain::{EpochSeconds, ExecutionCost};
use writfold_wire::engineering::edit_approval::SIGNING_DOMAIN as EDIT_DOMAIN;
use writfold_wire::engineering::grant::{
    GRANT_DOMAIN, GRANT_ENVELOPE, REVOCATION_DOMAIN, REVOCATION_ENVELOPE, decode_grant,
    encode_envelope, encode_grant,
};
use writfold_wire::operation::BoundaryError;
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::signatures::grant::{GRANT_USE, REVOCATION_USE, Signed, verify};
use writfold_wire::{Limits, RawJson, canonical};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const PUBLIC_KEY: &str = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

fn limits() -> Result<Limits> {
    Ok(Limits::new(1_000_000, 128, 10_000, 100_000)?)
}
fn pem() -> Result<String> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/signatures/PUBLIC-TEST-ONLY.pem");
    Ok(String::from_utf8(std::fs::read(path)?)?)
}
fn signing_key() -> Result<ApprovalSigningKey> {
    Ok(ApprovalSigningKey::from_pkcs8_pem(&pem()?)?)
}
/// The same test key signing `payload` under an arbitrary domain.
fn raw_sign(domain: &[u8], payload: &[u8]) -> Result<[u8; 64]> {
    let raw = SigningKey::from_pkcs8_pem(&pem()?)?;
    Ok(raw.sign(&[domain, payload].concat()).to_bytes())
}
fn make_policy(uses: &[&str]) -> Result<AuthorityPolicy> {
    let value = json!({"kb_id": "test-kb", "policy_version": "grant-policy", "schema_version": 2,
        "trusted_keys": {"operator-key": {"public_key": PUBLIC_KEY, "target_uses": uses}}});
    let bytes = canonical(
        &RawJson::parse(&serde_json::to_vec(&value)?, limits()?)?,
        1_000_000,
    )?;
    Ok(AuthorityPolicy::new_bootstrap(&bytes, limits()?)?)
}
fn both() -> Result<AuthorityPolicy> {
    make_policy(&[GRANT_USE, REVOCATION_USE])
}
fn seconds(n: u64) -> Result<EpochSeconds> {
    Ok(EpochSeconds::new(n)?)
}
fn signer(policy: &AuthorityPolicy) -> Result<Signer> {
    Ok(Signer::new(
        "operator-key".parse()?,
        policy.identity().clone(),
    ))
}
fn grant(policy: &AuthorityPolicy) -> Result<Grant> {
    Ok(Grant::new(
        "grant-1".parse()?,
        Scope::new(
            "realm".parse()?,
            "run-1".parse()?,
            "repo".parse()?,
            "wt-1".parse()?,
            "binding-1".parse()?,
        )?,
        vec![
            "resource-2".parse()?,
            "resource-1".parse()?,
            "resource-2".parse()?,
        ],
        vec![Capability::Write, Capability::Read, Capability::Write],
        ExecutionCost::new(10)?,
        Validity::new(seconds(1_000)?, seconds(2_000)?)?,
        signer(policy)?,
    )?)
}
fn revocation(policy: &AuthorityPolicy) -> Result<Revocation> {
    Ok(Revocation::new(
        "realm".parse()?,
        "grant-1".parse()?,
        BlobDigest::from_digest([7; 32]),
        seconds(1_500)?,
        signer(policy)?,
    ))
}
fn binding(error: Option<BoundaryError>, expected: &str) -> Result {
    match error {
        Some(BoundaryError::Binding(reason)) if reason == expected => Ok(()),
        other => Err(format!("expected Binding({expected}), got {other:?}").into()),
    }
}
fn with_signature<T>(namespace: &str, payload: &[u8], signature: [u8; 64]) -> Result<Signed<T>>
where
    T: writfold_wire::signatures::grant::Record,
{
    Ok(Signed::decode(
        &encode_envelope(namespace, payload, &signature, limits()?)?,
        limits()?,
    )?)
}

#[test]
fn grant_and_revocation_round_trip_and_verify() -> Result {
    let policy = both()?;
    let key = signing_key()?;
    let signed = key.sign_record(grant(&policy)?, &policy, limits()?)?;
    let decoded: Signed<Grant> = Signed::decode(&signed.encode(limits()?)?, limits()?)?;
    assert_eq!(decoded, signed);
    let resources: Vec<_> = decoded
        .record()
        .resources()
        .iter()
        .map(|r| r.as_str())
        .collect();
    assert_eq!(resources, ["resource-1", "resource-2"]);
    assert_eq!(
        decoded.record().capabilities(),
        [Capability::Read, Capability::Write]
    );
    verify(&decoded, &policy)?;
    let revoked = key.sign_record(revocation(&policy)?, &policy, limits()?)?;
    let decoded: Signed<Revocation> = Signed::decode(&revoked.encode(limits()?)?, limits()?)?;
    assert_eq!(decoded.record(), &revocation(&policy)?);
    verify(&decoded, &policy)?;
    Ok(())
}

#[test]
fn each_record_has_its_own_domain_and_envelope() -> Result {
    let policy = both()?;
    let signed = signing_key()?.sign_record(grant(&policy)?, &policy, limits()?)?;
    for domain in [REVOCATION_DOMAIN, EDIT_DOMAIN] {
        let foreign: Signed<Grant> = with_signature(
            GRANT_ENVELOPE,
            signed.payload(),
            raw_sign(domain, signed.payload())?,
        )?;
        assert!(matches!(
            verify(&foreign, &policy),
            Err(BoundaryError::Signature)
        ));
    }
    let revoked = signing_key()?.sign_record(revocation(&policy)?, &policy, limits()?)?;
    let foreign: Signed<Revocation> = with_signature(
        REVOCATION_ENVELOPE,
        revoked.payload(),
        raw_sign(GRANT_DOMAIN, revoked.payload())?,
    )?;
    assert!(matches!(
        verify(&foreign, &policy),
        Err(BoundaryError::Signature)
    ));
    // A grant envelope is not a revocation envelope, and vice versa.
    let bytes = signed.encode(limits()?)?;
    assert!(Signed::<Revocation>::decode(&bytes, limits()?).is_err());
    assert!(Signed::<Grant>::decode(&revoked.encode(limits()?)?, limits()?).is_err());
    Ok(())
}

#[test]
fn transplanted_and_tampered_signatures_refuse() -> Result {
    let policy = both()?;
    let signed = signing_key()?.sign_record(grant(&policy)?, &policy, limits()?)?;
    let wider = Grant::new(
        "grant-1".parse()?,
        signed.record().scope().clone(),
        vec![
            "resource-1".parse()?,
            "resource-2".parse()?,
            "resource-3".parse()?,
        ],
        signed.record().capabilities().to_vec(),
        signed.record().budget(),
        signed.record().validity(),
        signed.record().signer().clone(),
    )?;
    let transplanted: Signed<Grant> = with_signature(
        GRANT_ENVELOPE,
        &encode_grant(&wider, limits()?)?,
        *signed.signature(),
    )?;
    assert!(matches!(
        verify(&transplanted, &policy),
        Err(BoundaryError::Signature)
    ));
    let mut flipped = *signed.signature();
    flipped[0] ^= 1;
    let tampered: Signed<Grant> = with_signature(GRANT_ENVELOPE, signed.payload(), flipped)?;
    assert!(matches!(
        verify(&tampered, &policy),
        Err(BoundaryError::Signature)
    ));
    Ok(())
}

#[test]
fn policy_identity_target_use_and_private_key_bind() -> Result {
    let policy = both()?;
    let key = signing_key()?;
    let signed = key.sign_record(grant(&policy)?, &policy, limits()?)?;
    binding(
        verify(&signed, &make_policy(&[GRANT_USE, "review"])?).err(),
        "record names a different policy",
    )?;
    let grants_only = make_policy(&[GRANT_USE])?;
    let revoke_only = make_policy(&[REVOCATION_USE])?;
    binding(
        key.sign_record(revocation(&grants_only)?, &grants_only, limits()?)
            .err(),
        "record key lacks the target use",
    )?;
    binding(
        key.sign_record(grant(&revoke_only)?, &revoke_only, limits()?)
            .err(),
        "record key lacks the target use",
    )?;
    // A policy trusting a different, strong key: the test key cannot sign under it.
    let other = SigningKey::from_bytes(&[0x41; 32]).verifying_key();
    let hex: String = other
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let value = json!({"kb_id": "test-kb", "policy_version": "grant-policy", "schema_version": 2,
        "trusted_keys": {"operator-key": {"public_key": hex, "target_uses": [GRANT_USE]}}});
    let foreign = AuthorityPolicy::new_bootstrap(
        &canonical(
            &RawJson::parse(&serde_json::to_vec(&value)?, limits()?)?,
            1_000_000,
        )?,
        limits()?,
    )?;
    binding(
        key.sign_record(grant(&foreign)?, &foreign, limits()?).err(),
        "private key differs from policy key",
    )?;
    Ok(())
}

#[test]
fn noncanonical_and_malformed_records_refuse() -> Result {
    let policy = both()?;
    let signed = signing_key()?.sign_record(grant(&policy)?, &policy, limits()?)?;
    let swap = |from: &[u8], to: &[u8]| -> Vec<u8> {
        let mut payload = signed.payload().to_vec();
        let at = payload.windows(from.len()).position(|w| w == from);
        if let Some(at) = at {
            payload[at..at + to.len()].copy_from_slice(to);
        }
        payload
    };
    // Unsorted resources: resource-2 before resource-1.
    let unsorted = swap(b"resource-1", b"resource-3");
    let unsorted = {
        let mut p = unsorted;
        let at = p.windows(10).position(|w| w == b"resource-2").ok_or("r2")?;
        p[at..at + 10].copy_from_slice(b"resource-1");
        let at = p.windows(10).position(|w| w == b"resource-3").ok_or("r3")?;
        p[at..at + 10].copy_from_slice(b"resource-2");
        p
    };
    // Duplicated resources.
    let duplicated = swap(b"resource-2", b"resource-1");
    for payload in [unsorted, duplicated] {
        assert!(decode_grant(&payload, limits()?).is_ok());
        let envelope = encode_envelope(GRANT_ENVELOPE, &payload, signed.signature(), limits()?)?;
        binding(
            Signed::<Grant>::decode(&envelope, limits()?).err(),
            "noncanonical operator authority record",
        )?;
    }
    let mut trailing = signed.encode(limits()?)?;
    trailing.push(0);
    assert!(Signed::<Grant>::decode(&trailing, limits()?).is_err());
    let encoded = signed.encode(limits()?)?;
    assert!(Signed::<Grant>::decode(&encoded[..encoded.len() - 1], limits()?).is_err());
    Ok(())
}

#[test]
fn signatures_cover_exactly_domain_and_payload() -> Result {
    let policy = both()?;
    let raw = SigningKey::from_pkcs8_pem(&pem()?)?.verifying_key();
    let signed = signing_key()?.sign_record(grant(&policy)?, &policy, limits()?)?;
    let signature = ed25519_dalek::Signature::from_bytes(signed.signature());
    raw.verify_strict(&[GRANT_DOMAIN, signed.payload()].concat(), &signature)?;
    assert!(raw.verify_strict(signed.payload(), &signature).is_err());
    let revoked = signing_key()?.sign_record(revocation(&policy)?, &policy, limits()?)?;
    let signature = ed25519_dalek::Signature::from_bytes(revoked.signature());
    raw.verify_strict(&[REVOCATION_DOMAIN, revoked.payload()].concat(), &signature)?;
    // A bare, undomained signature over the exact payload is refused.
    let bare: Signed<Grant> = with_signature(
        GRANT_ENVELOPE,
        signed.payload(),
        raw_sign(b"", signed.payload())?,
    )?;
    assert!(matches!(
        verify(&bare, &policy),
        Err(BoundaryError::Signature)
    ));
    Ok(())
}
