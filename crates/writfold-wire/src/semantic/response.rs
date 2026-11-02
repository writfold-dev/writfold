use super::formula::enc_statement;
use super::kb::enc_dependency;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn enc_support(value: &Support) -> Value {
    Value::String(
        match value {
            Support::None => "none",
            Support::Positive => "positive",
            Support::Negative => "negative",
            Support::Both => "both",
        }
        .to_owned(),
    )
}
pub fn enc_freshness(value: &Freshness) -> Value {
    Value::String(
        match value {
            Freshness::Fresh => "fresh",
            Freshness::Stale => "stale",
            Freshness::Unknown => "unknown",
        }
        .to_owned(),
    )
}
pub fn enc_assessment(value: &Assessment) -> Value {
    json!({"support": enc_support(value.support()), "applicable": value.applicable(), "freshness": enc_freshness(value.freshness()), "validated": value.validated(), "contested": value.contested(), "available": value.available(), "usable": value.usable()})
}
pub fn enc_checked(value: Option<&Checked>) -> Value {
    match value {
        None => Value::Null,
        Some(value) => {
            json!({"statement": enc_statement(value.statement()), "dependencies": value.dependencies().iter().map(enc_dependency).collect::<Vec<_>>()})
        }
    }
}
