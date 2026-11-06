use super::{Error, Result, identity, integer, scalar, shape, strings, take_map, text};
use crate::{Limits, RawJson, canonical::Writer};
use serde_json::{Map, Value};
use writfold_domain::{DoltIntegrationCommitId, SchemaEpoch, dolt::DoltReadPin};

const NAMESPACE: &str = "writfold-dolt-read";
const PIN_FIELDS: &str = "namespace schema kind realm kb database commit epoch schema_digest query_digest storage_profile producer";
const PIN_STRINGS: &str = "namespace kind realm kb database commit epoch schema_digest query_digest storage_profile producer";
const META_FIELDS: &str = "id native_commit realm kb database_id epoch schema_digest query_digest storage_profile producer";
const META_STRINGS: &str =
    "native_commit realm kb database_id epoch schema_digest query_digest storage_profile producer";

pub fn decode_pin(bytes: &[u8], limits: Limits) -> Result<DoltReadPin> {
    let mut raw = RawJson::parse(bytes, limits)?;
    let map = shape(raw.value(), PIN_FIELDS)?;
    strings(map, PIN_STRINGS)?;
    if text(map, "namespace")? != NAMESPACE {
        return Err(Error::UnsupportedNamespace);
    }
    if integer(map, "schema")? != 1 {
        return Err(Error::UnsupportedVersion);
    }
    if text(map, "kind")? != "pin" {
        return Err(Error::UnsupportedKind);
    }
    build_pin(take_map(&mut raw.value)?, "database", "commit")
}

pub fn decode_metadata(bytes: &[u8], limits: Limits) -> Result<DoltReadPin> {
    build_pin(
        scalar(bytes, limits, META_FIELDS, META_STRINGS)?,
        "database_id",
        "native_commit",
    )
}

pub fn decode_root(bytes: &[u8], limits: Limits) -> Result<DoltIntegrationCommitId> {
    identity(
        &mut scalar(bytes, limits, "native_commit", "native_commit")?,
        "native_commit",
    )
}

fn build_pin(
    mut map: Map<String, Value>,
    database: &'static str,
    commit: &'static str,
) -> Result<DoltReadPin> {
    Ok(DoltReadPin::new(
        identity(&mut map, "realm")?,
        identity(&mut map, "kb")?,
        identity(&mut map, database)?,
        identity(&mut map, commit)?,
        SchemaEpoch::new(
            identity(&mut map, "epoch")?,
            identity(&mut map, "schema_digest")?,
            identity(&mut map, "query_digest")?,
        ),
        identity(&mut map, "storage_profile")?,
        identity(&mut map, "producer")?,
    ))
}

/// Canonical sorted UTF-8 JSON, borrowing all pin fields and bounding every write.
/// A pin is not a signed release envelope or a writer approval.
pub fn encode_pin(pin: &DoltReadPin, max_bytes: usize) -> Result<Vec<u8>> {
    let mut writer = Writer {
        bytes: Vec::new(),
        max_bytes,
    };
    writer.append(b"{")?;
    for (index, (key, value)) in [
        ("commit", pin.commit().as_str()),
        ("database", pin.database().as_str()),
        ("epoch", pin.epoch().id().as_str()),
        ("kb", pin.kb().as_str()),
        ("kind", "pin"),
        ("namespace", NAMESPACE),
        ("producer", pin.producer().as_str()),
        ("query_digest", pin.epoch().query().as_str()),
        ("realm", pin.realm().as_str()),
        ("schema", "1"),
        ("schema_digest", pin.epoch().schema().as_str()),
        ("storage_profile", pin.profile().as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        if index != 0 {
            writer.append(b",")?;
        }
        writer.string(key)?;
        writer.append(b":")?;
        if key == "schema" {
            writer.append(b"1")?;
        } else {
            writer.string(value)?;
        }
    }
    writer.append(b"}")?;
    Ok(writer.bytes)
}
