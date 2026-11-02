use super::codec::*;
use super::context::*;
use super::formula::*;
use super::identity::*;
use super::witness::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_observation(value: &Value) -> Result<Observation> {
    let v = obj(value, "id source revision content observed_at outcome")?;
    let outcome = match text(field(v, "outcome")?)?.as_str() {
        "ok" => ObsOutcome::Ok,
        "timeout" => ObsOutcome::Timeout,
        "partial" => ObsOutcome::Partial,
        "truncated" => ObsOutcome::Truncated,
        "denied" => ObsOutcome::Denied,
        "unavailable" => ObsOutcome::Unavailable,
        _ => return Err(InvalidRequest),
    };
    Ok(Observation::new(
        identity(field(v, "id")?)?,
        identity(field(v, "source")?)?,
        identity(field(v, "revision")?)?,
        identity(field(v, "content")?)?,
        nat(field(v, "observed_at")?)?,
        outcome,
    ))
}
pub fn enc_observation(value: &Observation) -> Value {
    let outcome = match value.outcome() {
        ObsOutcome::Ok => "ok",
        ObsOutcome::Timeout => "timeout",
        ObsOutcome::Partial => "partial",
        ObsOutcome::Truncated => "truncated",
        ObsOutcome::Denied => "denied",
        ObsOutcome::Unavailable => "unavailable",
    };
    json!({"id": value.id().as_str(), "source": value.source().as_str(), "revision": value.revision().as_str(), "content": value.content().as_str(), "observed_at": enc_integer(value.observed_at()), "outcome": outcome})
}
pub fn dec_attestation(value: &Value) -> Result<Attestation> {
    let v = obj(value, "id claim observation statement authority")?;
    let result = Attestation::new(
        identity(field(v, "id")?)?,
        dec_ref(field(v, "claim")?)?,
        identity(field(v, "observation")?)?,
        dec_statement(field(v, "statement")?)?,
        identity(field(v, "authority")?)?,
    );
    if !matches!(
        result.statement().formula(),
        Formula::Atomic(_) | Formula::Inventory { .. }
    ) {
        return Err(InvalidRequest);
    }
    Ok(result)
}
pub fn enc_attestation(value: &Attestation) -> Value {
    json!({"id": value.id().as_str(), "claim": enc_ref(value.claim()), "observation": value.observation().as_str(), "statement": enc_statement(value.statement()), "authority": value.authority().as_str()})
}
pub fn dec_enumeration(value: &Value) -> Result<Enumeration> {
    let v = obj(
        value,
        "id source revision query scope observation members universe method",
    )?;
    Ok(Enumeration::new(
        identity(field(v, "id")?)?,
        identity(field(v, "source")?)?,
        identity(field(v, "revision")?)?,
        nonempty(field(v, "query")?)?,
        dec_scope(field(v, "scope")?)?,
        identity(field(v, "observation")?)?,
        items(field(v, "members")?, nonempty)?,
        items(field(v, "universe")?, nonempty)?,
        nonempty(field(v, "method")?)?,
    ))
}
pub fn enc_enumeration(value: &Enumeration) -> Value {
    json!({"id": value.id().as_str(), "source": value.source().as_str(), "revision": value.revision().as_str(), "query": value.query(), "scope": enc_scope(value.scope()), "observation": value.observation().as_str(), "members": value.members(), "universe": value.universe(), "method": value.method()})
}
pub fn dec_correction(value: &Value) -> Result<Correction> {
    let v = obj(value, "id kind target replacement recorded_at")?;
    let kind = match text(field(v, "kind")?)?.as_str() {
        "retract" => CorrectionKind::Retract,
        "dispute" => CorrectionKind::Dispute,
        "supersede" => CorrectionKind::Supersede,
        _ => return Err(InvalidRequest),
    };
    let replacement = field(v, "replacement")?;
    Ok(Correction::new(
        identity(field(v, "id")?)?,
        kind,
        dec_ref(field(v, "target")?)?,
        if replacement.is_null() {
            None
        } else {
            Some(dec_ref(replacement)?)
        },
        nat(field(v, "recorded_at")?)?,
    ))
}
pub fn enc_correction(value: &Correction) -> Value {
    json!({"id": value.id().as_str(), "kind": match value.kind() { CorrectionKind::Retract => "retract", CorrectionKind::Dispute => "dispute", CorrectionKind::Supersede => "supersede" }, "target": enc_ref(value.target()), "replacement": value.replacement().as_ref().map(enc_ref), "recorded_at": enc_integer(value.recorded_at())})
}
pub fn dec_dependency(value: &Value) -> Result<Dependency> {
    let v = object(value)?;
    let tag = text(field(v, "tag")?)?;
    if tag == "corrections" {
        obj(value, "tag target events")?;
        return Ok(Dependency::Corrections {
            target: dec_ref(field(v, "target")?)?,
            events: items(field(v, "events")?, dec_correction)?,
        });
    }
    obj(value, "tag value")?;
    let value = field(v, "value")?;
    match tag.as_str() {
        "observation" => Ok(Dependency::Observation(dec_observation(value)?)),
        "claim" => Ok(Dependency::Claim(dec_claim(value)?)),
        "attestation" => Ok(Dependency::Attestation(dec_attestation(value)?)),
        "enumeration" => Ok(Dependency::Enumeration(dec_enumeration(value)?)),
        "context" => {
            let context = dec_context(value)?;
            if context.now().to_u64() != Some(0) {
                return Err(InvalidRequest);
            }
            Ok(Dependency::Context(context))
        }
        _ => Err(InvalidRequest),
    }
}
pub fn enc_dependency(value: &Dependency) -> Value {
    match value {
        Dependency::Observation(value) => {
            json!({"tag": "observation", "value": enc_observation(value)})
        }
        Dependency::Claim(value) => json!({"tag": "claim", "value": enc_claim(value)}),
        Dependency::Attestation(value) => {
            json!({"tag": "attestation", "value": enc_attestation(value)})
        }
        Dependency::Enumeration(value) => {
            json!({"tag": "enumeration", "value": enc_enumeration(value)})
        }
        Dependency::Corrections { target, events } => {
            json!({"tag": "corrections", "target": enc_ref(target), "events": events.iter().map(enc_correction).collect::<Vec<_>>()})
        }
        Dependency::Context(value) => json!({"tag": "context", "value": enc_context(value)}),
    }
}
pub fn dec_receipt(value: &Value) -> Result<Receipt> {
    let v = obj(
        value,
        "id claim context witness dependencies result expires_at",
    )?;
    let result = match text(field(v, "result")?)?.as_str() {
        "pass" => ReceiptResult::Pass,
        "fail" => ReceiptResult::Fail,
        _ => return Err(InvalidRequest),
    };
    Ok(Receipt::new(
        identity(field(v, "id")?)?,
        dec_ref(field(v, "claim")?)?,
        dec_context(field(v, "context")?)?,
        dec_witness(field(v, "witness")?)?,
        items(field(v, "dependencies")?, dec_dependency)?,
        result,
        nat(field(v, "expires_at")?)?,
    ))
}
pub fn enc_receipt(value: &Receipt) -> Value {
    json!({"id": value.id().as_str(), "claim": enc_ref(value.claim()), "context": enc_context(value.context()), "witness": enc_witness(value.witness()), "dependencies": value.dependencies().iter().map(enc_dependency).collect::<Vec<_>>(), "result": match value.result() { ReceiptResult::Pass => "pass", ReceiptResult::Fail => "fail" }, "expires_at": enc_integer(value.expires_at())})
}
pub fn dec_kb(value: &Value) -> Result<KB> {
    let v = obj(
        value,
        "snapshot observations claims attestations enumerations derivations corrections receipts",
    )?;
    let result = KB::new(
        identity(field(v, "snapshot")?)?,
        items(field(v, "observations")?, dec_observation)?,
        items(field(v, "claims")?, dec_claim)?,
        items(field(v, "attestations")?, dec_attestation)?,
        items(field(v, "enumerations")?, dec_enumeration)?,
        items(field(v, "derivations")?, dec_derivation)?,
        items(field(v, "corrections")?, dec_correction)?,
        items(field(v, "receipts")?, dec_receipt)?,
    );
    if !writfold_kernel::wellformed::well_formed(&result) {
        return Err(InvalidRequest);
    }
    Ok(result)
}
pub fn enc_kb(value: &KB) -> Value {
    json!({"snapshot": value.snapshot().as_str(), "observations": value.observations().iter().map(enc_observation).collect::<Vec<_>>(), "claims": value.claims().iter().map(enc_claim).collect::<Vec<_>>(), "attestations": value.attestations().iter().map(enc_attestation).collect::<Vec<_>>(), "enumerations": value.enumerations().iter().map(enc_enumeration).collect::<Vec<_>>(), "derivations": value.derivations().iter().map(enc_derivation).collect::<Vec<_>>(), "corrections": value.corrections().iter().map(enc_correction).collect::<Vec<_>>(), "receipts": value.receipts().iter().map(enc_receipt).collect::<Vec<_>>()})
}
