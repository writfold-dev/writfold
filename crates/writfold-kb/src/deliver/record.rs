//! A delivery record that carries its own digest (a withdrawal, a post
//! record): one JSON object, written create-new, whose `digest` is the
//! SHA-256 of its canonical form without that key. A record whose digest
//! does not match is invalid and counts for nothing.
use crate::gate::receipt::canonical;
use serde_json::Value;
use std::path::Path;

/// The bytes of `value` with its digest added.
pub fn seal(mut value: Value) -> Vec<u8> {
    let digest = super::candidate::sha(canonical(&value).as_bytes());
    if let Value::Object(map) = &mut value {
        map.insert("digest".into(), Value::String(digest));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

/// Read a sealed record of `schema`; why it does not read otherwise.
pub fn open(text: &str, schema: &str) -> Result<Value, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value
        .as_object_mut()
        .ok_or("not a JSON object")?
        .remove("digest");
    if value.get("schema").and_then(Value::as_str) != Some(schema) {
        return Err(format!("schema is not {schema}"));
    }
    let digest = super::candidate::sha(canonical(&value).as_bytes());
    if claimed.as_ref().and_then(Value::as_str) != Some(digest.as_str()) {
        return Err("its digest does not match its content".into());
    }
    Ok(value)
}

/// A one-line string field.
pub fn text(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} is missing or not a one-line string"))
}

/// The `.json` files under `<root>/<dir>` whose names start with `prefix`,
/// by name, with their KB paths and texts.
pub fn files(root: &Path, dir: &str, prefix: &str) -> Vec<(String, String, Option<String>)> {
    let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(prefix) && n.ends_with(".json"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let path = format!("{dir}/{name}");
            let text = std::fs::read_to_string(root.join(&path)).ok();
            (name, path, text)
        })
        .collect()
}
