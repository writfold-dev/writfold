use super::codec::*;
use super::formula::*;
use super::identity::*;
use super::kb::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_ttl(value: &Value) -> Result<Ttl> {
    if value.is_null() {
        return Ok(Ttl::Missing);
    }
    if value.is_object() {
        let v = obj(value, "tag")?;
        if text(field(v, "tag")?)? != "noExpiry" {
            return Err(InvalidRequest);
        }
        return Ok(Ttl::NoExpiry);
    }
    Ok(Ttl::ExpireAfter(nat(value)?))
}
pub fn enc_ttl(value: &Ttl) -> Value {
    match value {
        Ttl::Missing => Value::Null,
        Ttl::NoExpiry => json!({"tag": "noExpiry"}),
        Ttl::ExpireAfter(secs) => enc_integer(secs),
    }
}
pub fn dec_policy(value: &Value) -> Result<Policy> {
    let v = obj(
        value,
        "id require_fresh require_validation require_uncontested ttl authorized_actors",
    )?;
    let actors: Vec<ActorId> = items(field(v, "authorized_actors")?, identity)?;
    if !unique(&actors) {
        return Err(InvalidRequest);
    }
    Ok(Policy::new(
        identity(field(v, "id")?)?,
        boolean(field(v, "require_fresh")?)?,
        boolean(field(v, "require_validation")?)?,
        boolean(field(v, "require_uncontested")?)?,
        dec_ttl(field(v, "ttl")?)?,
        actors,
    ))
}
pub fn enc_policy(value: &Policy) -> Value {
    json!({"id": value.id().as_str(), "require_fresh": value.require_fresh(), "require_validation": value.require_validation(), "require_uncontested": value.require_uncontested(), "ttl": enc_ttl(value.ttl()), "authorized_actors": value.authorized_actors().iter().map(ActorId::as_str).collect::<Vec<_>>()})
}
pub fn dec_source_selection(value: &Value) -> Result<SourceSelection> {
    let v = obj(value, "source revision content")?;
    Ok(SourceSelection::new(
        identity(field(v, "source")?)?,
        identity(field(v, "revision")?)?,
        identity(field(v, "content")?)?,
    ))
}
pub fn enc_source_selection(value: &SourceSelection) -> Value {
    json!({"source": value.source().as_str(), "revision": value.revision().as_str(), "content": value.content().as_str()})
}
pub fn dec_context(value: &Value) -> Result<Context> {
    let v = obj(
        value,
        "snapshot semantic_version ruleset checker policy now target intended_use sources",
    )?;
    let result = Context::new(
        identity(field(v, "snapshot")?)?,
        text(field(v, "semantic_version")?)?,
        identity(field(v, "ruleset")?)?,
        identity(field(v, "checker")?)?,
        dec_policy(field(v, "policy")?)?,
        nat(field(v, "now")?)?,
        dec_scope(field(v, "target")?)?,
        nonempty(field(v, "intended_use")?)?,
        items(field(v, "sources")?, dec_source_selection)?,
    );
    if !writfold_kernel::wellformed::context_wf(&result) {
        return Err(InvalidRequest);
    }
    Ok(result)
}
pub fn enc_context(value: &Context) -> Value {
    json!({"snapshot": value.snapshot().as_str(), "semantic_version": value.semantic_version(), "ruleset": value.ruleset().as_str(), "checker": value.checker().as_str(), "policy": enc_policy(value.policy()), "now": enc_integer(value.now()), "target": enc_scope(value.target()), "intended_use": value.intended_use(), "sources": value.sources().iter().map(enc_source_selection).collect::<Vec<_>>()})
}
pub fn dec_authority(value: &Value) -> Result<Authority> {
    let v = obj(
        value,
        "actor target intended_use policy statement approved approvals context",
    )?;
    Ok(Authority::new(
        identity(field(v, "actor")?)?,
        dec_ref(field(v, "target")?)?,
        nonempty(field(v, "intended_use")?)?,
        identity(field(v, "policy")?)?,
        dec_statement(field(v, "statement")?)?,
        dec_kb(field(v, "approved")?)?,
        items(field(v, "approvals")?, dec_claim)?,
        dec_context(field(v, "context")?)?,
    ))
}
pub fn enc_authority(value: &Authority) -> Value {
    json!({"actor": value.actor().as_str(), "target": enc_ref(value.target()), "intended_use": value.intended_use(), "policy": value.policy().as_str(), "statement": enc_statement(value.statement()), "approved": enc_kb(value.approved()), "approvals": value.approvals().iter().map(enc_claim).collect::<Vec<_>>(), "context": enc_context(value.context())})
}
