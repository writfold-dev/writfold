//! Strict DTO primitives; JSON values do not cross into kernel decisions.
use serde_json::{Map, Number, Value};
use writfold_domain::Integer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidRequest;
impl std::fmt::Display for InvalidRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid_request")
    }
}
impl std::error::Error for InvalidRequest {}
pub type Result<T, E = InvalidRequest> = std::result::Result<T, E>;

pub fn object(value: &Value) -> Result<&Map<String, Value>> {
    value.as_object().ok_or(InvalidRequest)
}
pub fn obj<'a>(value: &'a Value, fields: &str) -> Result<&'a Map<String, Value>> {
    let map = object(value)?;
    if map.len() != fields.split_whitespace().count()
        || fields.split_whitespace().any(|key| !map.contains_key(key))
    {
        return Err(InvalidRequest);
    }
    Ok(map)
}
pub fn field<'a>(value: &'a Map<String, Value>, key: &str) -> Result<&'a Value> {
    value.get(key).ok_or(InvalidRequest)
}
pub fn text(value: &Value) -> Result<String> {
    value.as_str().map(str::to_owned).ok_or(InvalidRequest)
}
pub fn nonempty(value: &Value) -> Result<String> {
    let value = text(value)?;
    if value.is_empty() {
        Err(InvalidRequest)
    } else {
        Ok(value)
    }
}
pub fn integer(value: &Value) -> Result<Integer> {
    let number = value.as_number().ok_or(InvalidRequest)?.to_string();
    if number.contains(['.', 'e', 'E']) {
        return Err(InvalidRequest);
    }
    number.parse().map_err(|_| InvalidRequest)
}
pub fn nat(value: &Value) -> Result<Integer> {
    let value = integer(value)?;
    if value.is_negative() {
        Err(InvalidRequest)
    } else {
        Ok(value)
    }
}
pub fn boolean(value: &Value) -> Result<bool> {
    value.as_bool().ok_or(InvalidRequest)
}
pub fn items<T>(value: &Value, decode: impl Fn(&Value) -> Result<T>) -> Result<Vec<T>> {
    value
        .as_array()
        .ok_or(InvalidRequest)?
        .iter()
        .map(decode)
        .collect()
}
pub fn identity<T: TryFrom<String>>(value: &Value) -> Result<T> {
    T::try_from(text(value)?).map_err(|_| InvalidRequest)
}
pub fn optional_identity<T: TryFrom<String>>(value: &Value) -> Result<Option<T>> {
    let value = text(value)?;
    if value.is_empty() {
        Ok(None)
    } else {
        T::try_from(value).map(Some).map_err(|_| InvalidRequest)
    }
}
pub fn enc_identity(value: &impl std::fmt::Display) -> Value {
    Value::String(value.to_string())
}
pub fn enc_optional_identity<T: std::fmt::Display>(value: &Option<T>) -> Value {
    Value::String(value.as_ref().map(ToString::to_string).unwrap_or_default())
}
pub fn enc_integer(value: &Integer) -> Value {
    // Integer's Display is a valid arbitrary-precision JSON integer by construction.
    Value::Number(Number::from_string_unchecked(value.to_string()))
}
pub fn unique<T: PartialEq>(values: &[T]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(i, value)| !values[..i].contains(value))
}
