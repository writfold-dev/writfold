use super::codec::*;
use super::identity::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub const MAX_TREE_DEPTH: usize = 256;

pub fn dec_formula(value: &Value) -> Result<Formula> {
    decode_formula(value, MAX_TREE_DEPTH)
}
fn decode_formula(value: &Value, remaining: usize) -> Result<Formula> {
    if remaining == 0 {
        return Err(InvalidRequest);
    }
    let v = object(value)?;
    match text(field(v, "tag")?)?.as_str() {
        "atom" => {
            obj(value, "tag atom")?;
            let a = obj(field(v, "atom")?, "subject predicate args")?;
            Ok(Formula::Atomic(Atom::new(
                nonempty(field(a, "subject")?)?,
                nonempty(field(a, "predicate")?)?,
                items(field(a, "args")?, text)?,
            )))
        }
        "truth" => {
            obj(value, "tag")?;
            Ok(Formula::Truth)
        }
        "and" => {
            obj(value, "tag left right")?;
            Ok(Formula::Conjunction {
                left: Box::new(decode_formula(field(v, "left")?, remaining - 1)?),
                right: Box::new(decode_formula(field(v, "right")?, remaining - 1)?),
            })
        }
        "all" => {
            obj(value, "tag enumeration predicate")?;
            Ok(Formula::All {
                enumeration: identity(field(v, "enumeration")?)?,
                predicate: nonempty(field(v, "predicate")?)?,
            })
        }
        "inventory" => {
            obj(value, "tag enumeration")?;
            Ok(Formula::Inventory {
                enumeration: identity(field(v, "enumeration")?)?,
            })
        }
        "absent" => {
            obj(value, "tag enumeration")?;
            Ok(Formula::Absent {
                enumeration: identity(field(v, "enumeration")?)?,
            })
        }
        _ => Err(InvalidRequest),
    }
}
pub fn enc_formula(value: &Formula) -> Value {
    match value {
        Formula::Atomic(atom) => {
            json!({"tag": "atom", "atom": {"subject": atom.subject(), "predicate": atom.predicate(), "args": atom.args()}})
        }
        Formula::Truth => json!({"tag": "truth"}),
        Formula::Conjunction { left, right } => {
            json!({"tag": "and", "left": enc_formula(left), "right": enc_formula(right)})
        }
        Formula::All {
            enumeration,
            predicate,
        } => json!({"tag": "all", "enumeration": enumeration.as_str(), "predicate": predicate}),
        Formula::Inventory { enumeration } => {
            json!({"tag": "inventory", "enumeration": enumeration.as_str()})
        }
        Formula::Absent { enumeration } => {
            json!({"tag": "absent", "enumeration": enumeration.as_str()})
        }
    }
}
pub fn dec_statement(value: &Value) -> Result<Statement> {
    let v = obj(value, "formula scope polarity")?;
    let formula = dec_formula(field(v, "formula")?)?;
    let polarity = match text(field(v, "polarity")?)?.as_str() {
        "positive" => Polarity::Positive,
        "negative" => Polarity::Negative,
        _ => return Err(InvalidRequest),
    };
    if polarity == Polarity::Negative && !matches!(formula, Formula::Atomic(_)) {
        return Err(InvalidRequest);
    }
    Ok(Statement::new(
        formula,
        dec_scope(field(v, "scope")?)?,
        polarity,
    ))
}
pub fn enc_statement(value: &Statement) -> Value {
    json!({"formula": enc_formula(value.formula()), "scope": enc_scope(value.scope()), "polarity": match value.polarity() { Polarity::Positive => "positive", Polarity::Negative => "negative" }})
}
pub fn dec_claim(value: &Value) -> Result<Claim> {
    let v = obj(value, "id revision statement")?;
    Ok(Claim::new(
        identity(field(v, "id")?)?,
        identity(field(v, "revision")?)?,
        dec_statement(field(v, "statement")?)?,
    ))
}
pub fn enc_claim(value: &Claim) -> Value {
    json!({"id": value.id().as_str(), "revision": value.revision().as_str(), "statement": enc_statement(value.statement())})
}
