//! Local realm identity representation for the explicit experimental-alpha
//! profile. These bytes do not establish complete initialization or authority.
use crate::{Limits, RawJson, canonical::Writer};
use serde_json::Value;
use writfold_domain::engineering::objects::RealmIdentity;

const NAMESPACE: &str = "writfold-realm";
const PROFILE: &str = "writfold-experimental-alpha/1";
const FIELDS: &[&str] = &["instance", "kb", "namespace", "profile", "realm", "schema"];
const STRINGS: &[&str] = &["instance", "kb", "namespace", "profile", "realm"];

#[derive(Debug, thiserror::Error)]
pub enum ObjectError {
    #[error(transparent)]
    Wire(#[from] crate::Error),
    #[error("invalid realm identity fields or types")]
    Shape,
    #[error("unsupported realm identity namespace")]
    Namespace,
    #[error("unsupported realm identity schema")]
    Schema,
    #[error("unsupported realm initialization profile")]
    Profile,
    #[error("invalid nominal realm identity field: {0}")]
    Identity(&'static str),
}

type Result<T, E = ObjectError> = std::result::Result<T, E>;

pub fn decode_identity(bytes: &[u8], limits: Limits) -> Result<RealmIdentity> {
    let mut raw = RawJson::parse(bytes, limits)?;
    let map = raw.value().as_object().ok_or(ObjectError::Shape)?;
    if map.len() != FIELDS.len()
        || FIELDS.iter().any(|field| !map.contains_key(*field))
        || STRINGS.iter().any(|field| !map[*field].is_string())
    {
        return Err(ObjectError::Shape);
    }
    if map["namespace"].as_str() != Some(NAMESPACE) {
        return Err(ObjectError::Namespace);
    }
    if map["schema"].as_u64() != Some(1) {
        return Err(ObjectError::Schema);
    }
    if map["profile"].as_str() != Some(PROFILE) {
        return Err(ObjectError::Profile);
    }
    Ok(RealmIdentity::new(
        identity(&mut raw.value, "realm")?,
        identity(&mut raw.value, "kb")?,
        identity(&mut raw.value, "instance")?,
    ))
}

/// Borrow fields into the existing bounded canonical JSON writer. Schema and
/// profile are closed constants, not caller-selectable compatibility fallbacks.
pub fn encode_identity(identity: &RealmIdentity, max_bytes: usize) -> Result<Vec<u8>> {
    let mut writer = Writer {
        bytes: Vec::new(),
        max_bytes,
    };
    writer.append(b"{")?;
    for (index, (key, value)) in [
        ("instance", identity.instance().as_str()),
        ("kb", identity.kb().as_str()),
        ("namespace", NAMESPACE),
        ("profile", PROFILE),
        ("realm", identity.realm().as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        if index != 0 {
            writer.append(b",")?;
        }
        writer.string(key)?;
        writer.append(b":")?;
        writer.string(value)?;
    }
    writer.append(b",\"schema\":1}")?;
    Ok(writer.bytes)
}

fn identity<T: TryFrom<String>>(root: &mut Value, field: &'static str) -> Result<T> {
    let value = root.get_mut(field).ok_or(ObjectError::Shape)?;
    if !value.is_string() {
        return Err(ObjectError::Shape);
    }
    let Value::String(text) = std::mem::take(value) else {
        return Err(ObjectError::Shape);
    };
    T::try_from(text).map_err(|_| ObjectError::Identity(field))
}
