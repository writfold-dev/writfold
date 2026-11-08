//! Prepared-edit approvals: exact round trip, strict verification and refusal of
//! wrong-domain, transplanted, tampered, stale, expired and mis-bound approvals.
use serde_json::{Value, json};
use std::path::PathBuf;
use writfold_domain::engineering::approval::{EditApproval, Signer, Validity};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest, ProcedureRevision};
use writfold_domain::worktree::{PreparedEditDigest, Scope, WorktreeDigest};
use writfold_domain::{EpochSeconds, ResourceGeneration, ResourceId};
use writfold_wire::engineering::edit_approval::{
    SIGNING_DOMAIN, decode_payload, encode_envelope, encode_payload,
};
use writfold_wire::operation::BoundaryError;
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::signatures::edit::{ExpectedEdit, SignedEditApproval, verify_edit};
use writfold_wire::{Limits, RawJson, canonical};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const PUBLIC_KEY: &str = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

fn limits() -> Result<Limits> {
    Ok(Limits::new(1_000_000, 128, 10_000, 100_000)?)
}
fn signing_key() -> Result<ApprovalSigningKey> {
    let pem = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/signatures/PUBLIC-TEST-ONLY.pem"),
    )?;
    Ok(ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(
        &pem,
    )?)?)
}
fn make_policy(uses: &[&str]) -> Result<AuthorityPolicy> {
    let value = json!({"kb_id": "test-kb", "policy_version": "edit-policy", "schema_version": 2,
        "trusted_keys": {"operator-key": {"public_key": PUBLIC_KEY, "target_uses": uses}}});
    let bytes = canonical(
        &RawJson::parse(&serde_json::to_vec(&value)?, limits()?)?,
        1_000_000,
    )?;
    Ok(AuthorityPolicy::new_bootstrap(&bytes, limits()?)?)
}
fn scope() -> Result<Scope> {
    Ok(Scope::new(
        "realm".parse()?,
        "run-1".parse()?,
        "repo".parse()?,
        "wt-1".parse()?,
        "binding-1".parse()?,
    )?)
}
fn seconds(n: u64) -> Result<EpochSeconds> {
    Ok(EpochSeconds::new(n)?)
}
fn approval(policy: &AuthorityPolicy) -> Result<EditApproval> {
    Ok(EditApproval::new(
        scope()?,
        PreparedEditDigest::from_digest([1; 32]),
        BlobDigest::from_digest([2; 32]),
        ProcedureRevision::from_digest([3; 32]),
        "worktree".parse()?,
        ResourceGeneration::new(4)?,
        Validity::new(seconds(1_000)?, seconds(2_000)?)?,
        Signer::new("operator-key".parse()?, policy.identity().clone()),
    ))
}
struct Current {
    scope: Scope,
    resource: ResourceId,
}
impl Current {
    fn new() -> Result<Self> {
        Ok(Self {
            scope: scope()?,
            resource: "worktree".parse()?,
        })
    }
    fn expected(&self) -> Result<ExpectedEdit<'_>> {
        Ok(ExpectedEdit {
            scope: &self.scope,
            prepared: PreparedEditDigest::from_digest([1; 32]),
            report: BlobDigest::from_digest([2; 32]),
            procedure: ProcedureRevision::from_digest([3; 32]),
            resource: &self.resource,
            generation: ResourceGeneration::new(4)?,
        })
    }
}
fn binding(error: BoundaryError, expected: &str) -> Result {
    match error {
        BoundaryError::Binding(reason) if reason == expected => Ok(()),
        other => Err(format!("expected Binding({expected}), got {other:?}").into()),
    }
}

#[test]
fn signs_round_trips_and_verifies_within_validity() -> Result {
    let policy = make_policy(&["prepared-edit-approval"])?;
    let signed = signing_key()?.sign_edit(approval(&policy)?, &policy, limits()?)?;
    let decoded = SignedEditApproval::decode(&signed.encode(limits()?)?, limits()?)?;
    assert_eq!(decoded, signed);
    assert_eq!(
        decode_payload(decoded.payload(), limits()?)?,
        approval(&policy)?
    );
    let current = Current::new()?;
    for now in [1_000, 1_999] {
        verify_edit(&decoded, &policy, current.expected()?, seconds(now)?)?;
    }
    for now in [999, 2_000] {
        let error = verify_edit(&decoded, &policy, current.expected()?, seconds(now)?)
            .err()
            .ok_or("validity")?;
        binding(error, "approval is outside its validity interval")?;
    }
    Ok(())
}

#[test]
fn wrong_domain_transplanted_and_tampered_signatures_refuse() -> Result {
    let policy = make_policy(&["prepared-edit-approval"])?;
    let key = signing_key()?;
    let signed = key.sign_edit(approval(&policy)?, &policy, limits()?)?;
    let current = Current::new()?;
    // Transplant: the signature of one approval attached to another payload.
    let mut other = approval(&policy)?;
    other = EditApproval::new(
        other.scope().clone(),
        PreparedEditDigest::from_digest([9; 32]),
        other.report(),
        other.procedure(),
        other.resource().clone(),
        other.generation(),
        other.validity(),
        other.signer().clone(),
    );
    let other_payload = encode_payload(&other, limits()?)?;
    let transplanted = SignedEditApproval::decode(
        &encode_envelope(&other_payload, signed.signature(), limits()?)?,
        limits()?,
    )?;
    let mut expected = current.expected()?;
    expected.prepared = PreparedEditDigest::from_digest([9; 32]);
    assert!(matches!(
        verify_edit(&transplanted, &policy, expected, seconds(1_500)?),
        Err(BoundaryError::Signature)
    ));
    // A valid key signing the payload without the edit-approval domain prefix.
    let undomained = sign_under_dolt_domain(signed.payload())?;
    let wrong_domain = SignedEditApproval::decode(
        &encode_envelope(signed.payload(), &undomained, limits()?)?,
        limits()?,
    )?;
    assert!(matches!(
        verify_edit(&wrong_domain, &policy, current.expected()?, seconds(1_500)?),
        Err(BoundaryError::Signature)
    ));
    // Flip one signature bit.
    let mut flipped = *signed.signature();
    flipped[0] ^= 1;
    let tampered = SignedEditApproval::decode(
        &encode_envelope(signed.payload(), &flipped, limits()?)?,
        limits()?,
    )?;
    assert!(matches!(
        verify_edit(&tampered, &policy, current.expected()?, seconds(1_500)?),
        Err(BoundaryError::Signature)
    ));
    Ok(())
}

/// The same test key signing the payload under the Dolt approval domain.
fn sign_under_dolt_domain(payload: &[u8]) -> Result<[u8; 64]> {
    use ed25519_dalek::{Signer as _, SigningKey, pkcs8::DecodePrivateKey};
    let pem = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/signatures/PUBLIC-TEST-ONLY.pem"),
    )?;
    let raw = SigningKey::from_pkcs8_pem(std::str::from_utf8(&pem)?)?;
    let mut message = b"writfold-dolt-approval/1\0".to_vec();
    message.extend_from_slice(payload);
    assert!(!message.starts_with(SIGNING_DOMAIN));
    Ok(raw.sign(&message).to_bytes())
}

#[test]
fn policy_identity_key_use_and_private_key_bind() -> Result {
    let policy = make_policy(&["prepared-edit-approval"])?;
    let key = signing_key()?;
    let current = Current::new()?;
    let signed = key.sign_edit(approval(&policy)?, &policy, limits()?)?;
    let other_policy = make_policy(&["prepared-edit-approval", "review"])?;
    binding(
        verify_edit(&signed, &other_policy, current.expected()?, seconds(1_500)?)
            .err()
            .ok_or("policy")?,
        "approval names a different policy",
    )?;
    let no_use = make_policy(&["review"])?;
    binding(
        key.sign_edit(approval(&no_use)?, &no_use, limits()?)
            .err()
            .ok_or("use")?,
        "approval key lacks the edit-approval use",
    )?;
    // A strong key from another seed: the test private key must not sign for it.
    let other = ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key();
    let hex: String = other
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let value: Value = json!({"kb_id": "test-kb", "policy_version": "edit-policy", "schema_version": 2,
        "trusted_keys": {"operator-key": {"public_key": hex, "target_uses": ["prepared-edit-approval"]}}});
    let foreign = AuthorityPolicy::new_bootstrap(
        &canonical(
            &RawJson::parse(&serde_json::to_vec(&value)?, limits()?)?,
            1_000_000,
        )?,
        limits()?,
    )?;
    binding(
        key.sign_edit(approval(&foreign)?, &foreign, limits()?)
            .err()
            .ok_or("foreign")?,
        "private key differs from policy key",
    )?;
    Ok(())
}

/// One change to the caller's current expectation.
type Mutation = fn(&mut ExpectedEdit<'_>);

#[test]
fn every_bound_value_must_match_the_current_expectation() -> Result {
    let policy = make_policy(&["prepared-edit-approval"])?;
    let signed = signing_key()?.sign_edit(approval(&policy)?, &policy, limits()?)?;
    let current = Current::new()?;
    let other_scope = Scope::new(
        "realm".parse()?,
        "run-2".parse()?,
        "repo".parse()?,
        "wt-1".parse()?,
        "binding-1".parse()?,
    )?;
    let other_resource: ResourceId = "other".parse()?;
    let cases: [(Mutation, &str); 4] = [
        (
            |e| e.prepared = PreparedEditDigest::from_digest([7; 32]),
            "approval names a different prepared edit",
        ),
        (
            |e| e.report = BlobDigest::from_digest([7; 32]),
            "approval names a different report",
        ),
        (
            |e| e.procedure = ProcedureRevision::from_digest([7; 32]),
            "approval names a different procedure",
        ),
        (
            |e| e.generation = ResourceGeneration::new(5).unwrap_or(e.generation),
            "approval resource generation is stale",
        ),
    ];
    for (mutate, reason) in cases {
        let mut expected = current.expected()?;
        mutate(&mut expected);
        binding(
            verify_edit(&signed, &policy, expected, seconds(1_500)?)
                .err()
                .ok_or(reason)?,
            reason,
        )?;
    }
    let mut expected = current.expected()?;
    expected.scope = &other_scope;
    binding(
        verify_edit(&signed, &policy, expected, seconds(1_500)?)
            .err()
            .ok_or("scope")?,
        "approval scope differs",
    )?;
    let mut expected = current.expected()?;
    expected.resource = &other_resource;
    binding(
        verify_edit(&signed, &policy, expected, seconds(1_500)?)
            .err()
            .ok_or("resource")?,
        "approval names a different resource",
    )?;
    Ok(())
}

#[test]
fn malformed_and_noncanonical_envelopes_refuse() -> Result {
    let policy = make_policy(&["prepared-edit-approval"])?;
    let signed = signing_key()?.sign_edit(approval(&policy)?, &policy, limits()?)?;
    let bytes = signed.encode(limits()?)?;
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(SignedEditApproval::decode(&trailing, limits()?).is_err());
    assert!(SignedEditApproval::decode(&bytes[..bytes.len() - 1], limits()?).is_err());
    let mut payload = signed.payload().to_vec();
    payload.push(0);
    assert!(
        SignedEditApproval::decode(
            &encode_envelope(&payload, signed.signature(), limits()?)?,
            limits()?
        )
        .is_err()
    );
    assert!(Validity::new(seconds(5)?, seconds(5)?).is_err());
    Ok(())
}
