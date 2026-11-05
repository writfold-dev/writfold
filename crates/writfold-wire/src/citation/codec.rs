use super::{
    CitationDescriptor, CitationWireError, GIT_BLOB_PROFILE, GitSource, NAMESPACE, Result, SCHEMA,
};
use crate::{Limits, RawJson, semantic::codec};
use serde_json::{Map, Value};
use writfold_domain::{
    ByteOffset, GitObjectFormat, GitObjectId,
    citation::{CoordinateUnit, Selector},
};

const FIELDS: &str =
    "namespace schema kind id release context role representation selector selection_digest";
const SOURCE_FIELDS: &str = "realm repository revision path blob capture_digest profile";

/// Budget and duplicate-key checks precede closed schema/profile checks. No
/// omitted identity, selector, role, namespace, or algorithm receives a default.
pub fn decode_descriptor(bytes: &[u8], limits: Limits) -> Result<CitationDescriptor> {
    let mut raw = RawJson::parse(bytes, limits)?;
    shape(raw.value())?;
    // Shape admission proves that every remaining value is scalar or one of the
    // fixed shallow records. Move strings rather than clone the parsed tree;
    // malformed/deep input stays owned by RawJson's iterative destructor.
    let mut fields = object(std::mem::take(&mut raw.value))?;
    let source = source(take(&mut fields, "representation")?)?;
    let mut selected = object(take(&mut fields, "selector")?)?;
    let first = ByteOffset::new(number(&take(&mut selected, "first")?, "first")?)
        .map_err(|_| CitationWireError::Field("first"))?;
    let last = ByteOffset::new(number(&take(&mut selected, "last")?, "last")?)
        .map_err(|_| CitationWireError::Field("last"))?;
    let selector = Selector::new(first, last, CoordinateUnit::Byte)
        .map_err(|_| CitationWireError::Field("selector"))?;
    CitationDescriptor::new(
        identity(&mut fields, "id")?,
        identity(&mut fields, "release")?,
        identity(&mut fields, "context")?,
        identity(&mut fields, "role")?,
        source,
        selector,
        identity(&mut fields, "selection_digest")?,
    )
}

/// Canonical UTF-8 JSON with a separate output budget. This is not a signed
/// operation, a read grant or proof that the named retained source exists.
pub fn encode_descriptor(descriptor: &CitationDescriptor, maximum: usize) -> Result<Vec<u8>> {
    super::json::encode(descriptor, maximum)
}

fn shape(value: &Value) -> Result<()> {
    let root = codec::object(value).map_err(|_| CitationWireError::Shape)?;
    if text(root, "namespace")? != NAMESPACE {
        return Err(CitationWireError::UnsupportedNamespace);
    }
    if number(field(root, "schema")?, "schema")? != SCHEMA {
        return Err(CitationWireError::UnsupportedSchema);
    }
    if text(root, "kind")? != "binding" {
        return Err(CitationWireError::UnsupportedKind);
    }
    codec::obj(value, FIELDS).map_err(|_| CitationWireError::Shape)?;
    for name in ["id", "release", "context", "role", "selection_digest"] {
        text(root, name)?;
    }
    let source = codec::obj(field(root, "representation")?, SOURCE_FIELDS)
        .map_err(|_| CitationWireError::Shape)?;
    for name in ["realm", "repository", "path", "capture_digest"] {
        text(source, name)?;
    }
    if text(source, "profile")? != GIT_BLOB_PROFILE {
        return Err(CitationWireError::UnsupportedProfile);
    }
    for name in ["revision", "blob"] {
        let native = codec::obj(field(source, name)?, "algorithm value")
            .map_err(|_| CitationWireError::Shape)?;
        text(native, "algorithm")?;
        text(native, "value")?;
    }
    let selector = codec::obj(field(root, "selector")?, "first last unit")
        .map_err(|_| CitationWireError::Shape)?;
    if text(selector, "unit")? != "byte" {
        return Err(CitationWireError::UnsupportedProfile);
    }
    number(field(selector, "first")?, "first")?;
    number(field(selector, "last")?, "last")?;
    Ok(())
}

fn source(value: Value) -> Result<GitSource> {
    let mut fields = object(value)?;
    let revision = native(take(&mut fields, "revision")?, "revision")?;
    let blob = native(take(&mut fields, "blob")?, "blob")?;
    GitSource::new(
        identity(&mut fields, "realm")?,
        identity(&mut fields, "repository")?,
        revision,
        string(take(&mut fields, "path")?, "path")?,
        blob,
        identity(&mut fields, "capture_digest")?,
    )
}

fn native(value: Value, name: &'static str) -> Result<GitObjectId> {
    let mut fields = object(value)?;
    let algorithm = string(take(&mut fields, "algorithm")?, "algorithm")?;
    let format = match algorithm.as_str() {
        "sha1" => GitObjectFormat::Sha1,
        "sha256" => GitObjectFormat::Sha256,
        _ => return Err(CitationWireError::UnsupportedProfile),
    };
    GitObjectId::new(format, string(take(&mut fields, "value")?, name)?)
        .map_err(|_| CitationWireError::Identity(name))
}

fn object(value: Value) -> Result<Map<String, Value>> {
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(CitationWireError::Shape),
    }
}
fn take(fields: &mut Map<String, Value>, name: &'static str) -> Result<Value> {
    fields.remove(name).ok_or(CitationWireError::Field(name))
}
fn string(value: Value, name: &'static str) -> Result<String> {
    match value {
        Value::String(text) => Ok(text),
        _ => Err(CitationWireError::Field(name)),
    }
}
fn identity<T: TryFrom<String>>(fields: &mut Map<String, Value>, name: &'static str) -> Result<T> {
    T::try_from(string(take(fields, name)?, name)?).map_err(|_| CitationWireError::Identity(name))
}
fn field<'a>(fields: &'a Map<String, Value>, name: &'static str) -> Result<&'a Value> {
    fields.get(name).ok_or(CitationWireError::Field(name))
}
fn text<'a>(fields: &'a Map<String, Value>, name: &'static str) -> Result<&'a str> {
    field(fields, name)?
        .as_str()
        .ok_or(CitationWireError::Field(name))
}
fn number(value: &Value, name: &'static str) -> Result<u64> {
    value
        .as_number()
        .filter(|number| !number.as_str().contains(['.', 'e', 'E']))
        .and_then(serde_json::Number::as_u64)
        .ok_or(CitationWireError::Field(name))
}
