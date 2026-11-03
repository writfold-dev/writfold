//! Bounded, canonical storage-pin records, independent of signed operation schemas.
//! Decoding checks representation and nominal identities, not backend admission,
//! authentication, graph existence, or the availability of any reader/writer.
use crate::{Limits, RawJson, canonical, semantic::codec};
use serde_json::{Map, Value};
use thiserror::Error;
use writfold_domain::{LegacyDoltPin, NativeFormatVersion, OmnigraphPin, SchemaEpoch, StoragePin};

pub const STORAGE_PIN_NAMESPACE: &str = "writfold-storage-pin";
pub const STORAGE_PIN_SCHEMA: u64 = 1;

const OMNIGRAPH_FIELDS: &str = "namespace schema backend kb graph commit epoch schema_digest query_digest storage_profile native_format producer_generation";
const LEGACY_FIELDS: &str = "namespace schema backend kb dolt_commit";

#[derive(Debug, Error, Eq, PartialEq)]
pub enum StorageWireError {
    #[error(transparent)]
    Wire(#[from] crate::Error),
    #[error("storage pin must be an object with exactly the backend's declared fields")]
    InvalidShape,
    #[error("invalid or missing storage pin field {field}")]
    InvalidField { field: &'static str },
    #[error("invalid storage pin identity in {field}")]
    InvalidIdentity { field: &'static str },
    #[error("unsupported storage pin namespace")]
    UnsupportedNamespace,
    #[error("unsupported storage pin schema")]
    UnsupportedVersion,
    #[error("unsupported storage pin backend")]
    UnsupportedBackend,
}

type Result<T, E = StorageWireError> = std::result::Result<T, E>;

/// Applies raw budgets/duplicate detection before semantic checks. Semantic error
/// precedence is root shape, namespace, schema, backend, exact field set, then
/// backend fields in constructor order. No omitted field has a default.
pub fn decode_pin(bytes: &[u8], limits: Limits) -> Result<StoragePin> {
    let raw = RawJson::parse(bytes, limits)?;
    let map = codec::object(raw.value()).map_err(|_| StorageWireError::InvalidShape)?;
    if text(map, "namespace")? != STORAGE_PIN_NAMESPACE {
        return Err(StorageWireError::UnsupportedNamespace);
    }
    let schema = field(map, "schema")?
        .as_number()
        .filter(|number| !number.as_str().contains(['.', 'e', 'E']))
        .ok_or(StorageWireError::InvalidField { field: "schema" })?;
    if schema.as_u64() != Some(STORAGE_PIN_SCHEMA) {
        return Err(StorageWireError::UnsupportedVersion);
    }
    match text(map, "backend")? {
        "omnigraph" => {
            codec::obj(raw.value(), OMNIGRAPH_FIELDS)
                .map_err(|_| StorageWireError::InvalidShape)?;
            decode_omnigraph(map).map(StoragePin::Omnigraph)
        }
        "legacy-dolt" => {
            codec::obj(raw.value(), LEGACY_FIELDS).map_err(|_| StorageWireError::InvalidShape)?;
            Ok(StoragePin::LegacyDolt(LegacyDoltPin::new(
                identity(map, "kb")?,
                identity(map, "dolt_commit")?,
            )))
        }
        _ => Err(StorageWireError::UnsupportedBackend),
    }
}

fn decode_omnigraph(map: &Map<String, Value>) -> Result<OmnigraphPin> {
    let kb = identity(map, "kb")?;
    let graph = identity(map, "graph")?;
    let commit = identity(map, "commit")?;
    let epoch = SchemaEpoch::new(
        identity(map, "epoch")?,
        identity(map, "schema_digest")?,
        identity(map, "query_digest")?,
    );
    let profile = identity(map, "storage_profile")?;
    let native_format = field(map, "native_format")?
        .as_u64()
        .and_then(|value| NativeFormatVersion::new(value).ok())
        .ok_or(StorageWireError::InvalidField {
            field: "native_format",
        })?;
    Ok(OmnigraphPin::new(
        kb,
        graph,
        commit,
        epoch,
        profile,
        native_format,
        identity(map, "producer_generation")?,
    ))
}

fn field<'a>(map: &'a Map<String, Value>, field: &'static str) -> Result<&'a Value> {
    codec::field(map, field).map_err(|_| StorageWireError::InvalidField { field })
}

fn text<'a>(map: &'a Map<String, Value>, key: &'static str) -> Result<&'a str> {
    field(map, key)?
        .as_str()
        .ok_or(StorageWireError::InvalidField { field: key })
}

fn identity<T: TryFrom<String>>(map: &Map<String, Value>, key: &'static str) -> Result<T> {
    let value = field(map, key)?;
    if !value.is_string() {
        return Err(StorageWireError::InvalidField { field: key });
    }
    codec::identity(value).map_err(|_| StorageWireError::InvalidIdentity { field: key })
}

/// Emits canonical UTF-8 JSON without a newline, bounded independently of decode
/// limits. Only checked domain fields are materialized; no retained input object
/// or metadata is cloned. This is not a signed-envelope encoding.
pub fn encode_pin(pin: &StoragePin, max_bytes: usize) -> Result<Vec<u8>> {
    let mut map = Map::new();
    map.insert("namespace".into(), Value::from(STORAGE_PIN_NAMESPACE));
    map.insert("schema".into(), Value::from(STORAGE_PIN_SCHEMA));
    match pin {
        StoragePin::Omnigraph(pin) => encode_omnigraph(pin, &mut map),
        StoragePin::LegacyDolt(pin) => {
            map.insert("backend".into(), Value::from("legacy-dolt"));
            map.insert("kb".into(), codec::enc_identity(pin.kb()));
            map.insert("dolt_commit".into(), codec::enc_identity(pin.commit()));
        }
    }
    canonical(
        &RawJson {
            value: Value::Object(map),
        },
        max_bytes,
    )
    .map_err(StorageWireError::Wire)
}

fn encode_omnigraph(pin: &OmnigraphPin, map: &mut Map<String, Value>) {
    map.insert("backend".into(), Value::from("omnigraph"));
    map.insert("kb".into(), codec::enc_identity(pin.kb()));
    map.insert("graph".into(), codec::enc_identity(pin.graph()));
    map.insert("commit".into(), codec::enc_identity(pin.commit()));
    map.insert("epoch".into(), codec::enc_identity(pin.epoch().id()));
    map.insert(
        "schema_digest".into(),
        codec::enc_identity(pin.epoch().schema()),
    );
    map.insert(
        "query_digest".into(),
        codec::enc_identity(pin.epoch().query()),
    );
    map.insert("storage_profile".into(), codec::enc_identity(pin.profile()));
    map.insert(
        "native_format".into(),
        Value::from(pin.native_format().get()),
    );
    map.insert(
        "producer_generation".into(),
        codec::enc_identity(pin.producer()),
    );
}
