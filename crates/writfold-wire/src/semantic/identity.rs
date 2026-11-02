use super::codec::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_scope(value: &Value) -> Result<Scope> {
    let v = obj(value, "project selector")?;
    Ok(Scope::new(
        identity(field(v, "project")?)?,
        text(field(v, "selector")?)?,
    ))
}
pub fn enc_scope(value: &Scope) -> Value {
    json!({"project": value.project().as_str(), "selector": value.selector()})
}
pub fn dec_ref(value: &Value) -> Result<ClaimRef> {
    let v = obj(value, "id revision")?;
    Ok(ClaimRef::new(
        identity(field(v, "id")?)?,
        identity(field(v, "revision")?)?,
    ))
}
pub fn enc_ref(value: &ClaimRef) -> Value {
    json!({"id": value.id().as_str(), "revision": value.rev().as_str()})
}
