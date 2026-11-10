//! Bounded Dolt read projection and writer-probe codecs. No release or writer
//! authority is encoded.
use crate::{Limits, RawJson, semantic::codec};
use serde_json::{Map, Value};
use thiserror::Error;

mod pin;
mod rows;
mod write;
pub use pin::{decode_metadata, decode_pin, decode_root, encode_pin};
pub use rows::{decode_edges, decode_nodes};
pub use write::{DoltHeadProbe, decode_applied, decode_head};

#[derive(Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error(transparent)]
    Wire(#[from] crate::Error),
    #[error("Dolt record has an invalid field set or container shape")]
    InvalidShape,
    #[error("invalid or missing Dolt field {field}")]
    InvalidField { field: &'static str },
    #[error("invalid nominal identity in Dolt field {field}")]
    InvalidIdentity { field: &'static str },
    #[error("unsupported Dolt pin namespace")]
    UnsupportedNamespace,
    #[error("unsupported Dolt pin schema")]
    UnsupportedVersion,
    #[error("unsupported Dolt pin kind")]
    UnsupportedKind,
    #[error("Dolt scalar result must contain exactly one row")]
    RowCount,
    #[error("Dolt projection exceeds row limit")]
    RowLimit,
    #[error("Dolt row IDs are not strictly increasing at row {row}")]
    RowOrder { row: usize },
    #[error("Dolt null home path has a nonempty payload")]
    NullPayload,
}

type Result<T, E = Error> = std::result::Result<T, E>;

fn shape<'a>(value: &'a Value, fields: &str) -> Result<&'a Map<String, Value>> {
    codec::obj(value, fields).map_err(|_| Error::InvalidShape)
}

fn text<'a>(map: &'a Map<String, Value>, field: &'static str) -> Result<&'a str> {
    map.get(field)
        .and_then(Value::as_str)
        .ok_or(Error::InvalidField { field })
}

fn integer(map: &Map<String, Value>, field: &'static str) -> Result<u64> {
    map.get(field)
        .and_then(Value::as_number)
        .filter(|number| !number.as_str().contains(['.', 'e', 'E', '-']))
        .and_then(|number| number.as_u64())
        .ok_or(Error::InvalidField { field })
}

fn boolean(map: &Map<String, Value>, field: &'static str) -> Result<bool> {
    map.get(field)
        .and_then(Value::as_bool)
        .ok_or(Error::InvalidField { field })
}

fn strings(map: &Map<String, Value>, fields: &'static str) -> Result<()> {
    // Callers use static field lists. Return the precise key without allocating.
    for field in fields.split_ascii_whitespace() {
        if !map.get(field).is_some_and(Value::is_string) {
            return Err(Error::InvalidField { field });
        }
    }
    Ok(())
}

fn take_text(map: &mut Map<String, Value>, field: &'static str) -> Result<String> {
    match map.remove(field) {
        Some(Value::String(value)) => Ok(value),
        _ => Err(Error::InvalidField { field }),
    }
}

fn identity<T: TryFrom<String>>(map: &mut Map<String, Value>, field: &'static str) -> Result<T> {
    T::try_from(take_text(map, field)?).map_err(|_| Error::InvalidIdentity { field })
}

fn take_map(value: &mut Value) -> Result<Map<String, Value>> {
    // Only called after the entire extracted tree has been validated as shallow.
    match std::mem::take(value) {
        Value::Object(map) => Ok(map),
        _ => Err(Error::InvalidShape),
    }
}

fn native_rows(raw: &RawJson) -> Result<&Vec<Value>> {
    let root = shape(raw.value(), "rows")?;
    root.get("rows")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidShape)
}

fn scalar(
    bytes: &[u8],
    limits: Limits,
    fields: &str,
    string_fields: &'static str,
) -> Result<Map<String, Value>> {
    let mut raw = RawJson::parse(bytes, limits)?;
    let rows = native_rows(&raw)?;
    if rows.len() != 1 {
        return Err(Error::RowCount);
    }
    let map = shape(rows.first().ok_or(Error::RowCount)?, fields)?;
    strings(map, string_fields)?;
    // Numeric scalar metadata is checked while RawJson still owns every subtree.
    if fields.split_ascii_whitespace().any(|field| field == "id") && integer(map, "id")? != 1 {
        return Err(Error::InvalidField { field: "id" });
    }
    let Some(Value::Array(rows)) = raw.value.get_mut("rows") else {
        return Err(Error::InvalidShape);
    };
    take_map(rows.first_mut().ok_or(Error::RowCount)?)
}
