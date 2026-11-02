use super::codec::*;
use super::formula::MAX_TREE_DEPTH;
use super::identity::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_witness(value: &Value) -> Result<Witness> {
    decode_witness(value, MAX_TREE_DEPTH)
}
fn decode_witness(value: &Value, remaining: usize) -> Result<Witness> {
    if remaining == 0 {
        return Err(InvalidRequest);
    }
    let v = object(value)?;
    match text(field(v, "tag")?)?.as_str() {
        "attest" => {
            obj(value, "tag attestation")?;
            Ok(Witness::Attest {
                attestation: identity(field(v, "attestation")?)?,
            })
        }
        "truth" => {
            obj(value, "tag scope")?;
            Ok(Witness::Truth {
                scope: dec_scope(field(v, "scope")?)?,
            })
        }
        "and" => {
            obj(value, "tag left right")?;
            Ok(Witness::And {
                left: Box::new(decode_witness(field(v, "left")?, remaining - 1)?),
                right: Box::new(decode_witness(field(v, "right")?, remaining - 1)?),
            })
        }
        "specialize" => {
            obj(value, "tag scope premise")?;
            Ok(Witness::Specialize {
                scope: dec_scope(field(v, "scope")?)?,
                premise: Box::new(decode_witness(field(v, "premise")?, remaining - 1)?),
            })
        }
        "all" => {
            obj(value, "tag enumeration predicate premise")?;
            Ok(Witness::All {
                enumeration: identity(field(v, "enumeration")?)?,
                predicate: nonempty(field(v, "predicate")?)?,
                premise: Box::new(decode_witness(field(v, "premise")?, remaining - 1)?),
            })
        }
        "absent" => {
            obj(value, "tag enumeration premise")?;
            Ok(Witness::Absent {
                enumeration: identity(field(v, "enumeration")?)?,
                premise: Box::new(decode_witness(field(v, "premise")?, remaining - 1)?),
            })
        }
        _ => Err(InvalidRequest),
    }
}
pub fn enc_witness(value: &Witness) -> Value {
    match value {
        Witness::Attest { attestation } => {
            json!({"tag": "attest", "attestation": attestation.as_str()})
        }
        Witness::Truth { scope } => json!({"tag": "truth", "scope": enc_scope(scope)}),
        Witness::And { left, right } => {
            json!({"tag": "and", "left": enc_witness(left), "right": enc_witness(right)})
        }
        Witness::Specialize { scope, premise } => {
            json!({"tag": "specialize", "scope": enc_scope(scope), "premise": enc_witness(premise)})
        }
        Witness::All {
            enumeration,
            predicate,
            premise,
        } => {
            json!({"tag": "all", "enumeration": enumeration.as_str(), "predicate": predicate, "premise": enc_witness(premise)})
        }
        Witness::Absent {
            enumeration,
            premise,
        } => {
            json!({"tag": "absent", "enumeration": enumeration.as_str(), "premise": enc_witness(premise)})
        }
    }
}
pub fn dec_derivation(value: &Value) -> Result<Derivation> {
    let v = obj(value, "id claim witness")?;
    Ok(Derivation::new(
        identity(field(v, "id")?)?,
        dec_ref(field(v, "claim")?)?,
        dec_witness(field(v, "witness")?)?,
    ))
}
pub fn enc_derivation(value: &Derivation) -> Value {
    json!({"id": value.id().as_str(), "claim": enc_ref(value.claim()), "witness": enc_witness(value.witness())})
}
