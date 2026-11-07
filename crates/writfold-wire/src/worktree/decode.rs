use super::*;
use crate::RawJson;
use serde_json::{Map, Value};
use std::str::FromStr;

pub(super) fn record(bytes: &[u8], limits: Limits) -> Result<Record> {
    let raw = RawJson::parse(bytes, limits)?;
    let e = object(
        raw.value(),
        &["profile", "algorithm", "kind", "id", "payload"],
    )?;
    if text(get(e, "profile")?)? != PROFILE || text(get(e, "algorithm")?)? != ALGORITHM {
        return Err(Error::Invalid);
    }
    let kind = text(get(e, "kind")?)?;
    let p = get(e, "payload")?;
    let result = match kind {
        "recipe" => Record::Recipe(recipe(p)?),
        "capture" => Record::Capture(capture(p)?),
        "candidate" => Record::Candidate(candidate(p)?),
        "prepared-edit" => Record::PreparedEdit(prepared(p)?),
        "request-key" => Record::RequestKey(request_key(p)?),
        "observation" => Record::Observation(observation(p)?),
        _ => return Err(Error::Invalid),
    };
    if result.identity(limits)? != text(get(e, "id")?)? {
        return Err(Error::Invalid);
    }
    Ok(result)
}
fn object<'a>(v: &'a Value, fields: &[&str]) -> Result<&'a Map<String, Value>> {
    let obj = v.as_object().ok_or(Error::Invalid)?;
    if obj.len() != fields.len() || fields.iter().any(|k| !obj.contains_key(*k)) {
        return Err(Error::Invalid);
    }
    Ok(obj)
}
fn get<'a>(v: &'a Map<String, Value>, key: &str) -> Result<&'a Value> {
    v.get(key).ok_or(Error::Invalid)
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or(Error::Invalid)
}
fn string(v: &Value) -> Result<String> {
    Ok(text(v)?.to_owned())
}
fn uint(v: &Value) -> Result<u64> {
    let number = v.as_number().ok_or(Error::Invalid)?;
    if number.as_str().contains(['.', 'e', 'E', '-']) {
        return Err(Error::Invalid);
    }
    number.as_u64().ok_or(Error::Invalid)
}
fn u32_value(v: &Value) -> Result<u32> {
    u32::try_from(uint(v)?).map_err(|_| Error::Invalid)
}
fn identity<T: FromStr>(v: &Value) -> Result<T> {
    text(v)?.parse().map_err(|_| Error::Invalid)
}
fn hash(v: &Value) -> Result<[u8; 32]> {
    Ok(parse_hash(text(v)?)?)
}
fn bytes(v: &Value) -> Result<Vec<u8>> {
    let t = text(v)?;
    if t.len() % 2 != 0
        || !t
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Invalid);
    }
    (0..t.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&t[i..i + 2], 16).map_err(|_| Error::Invalid))
        .collect()
}
fn recipe(v: &Value) -> Result<Recipe> {
    let p = object(v, &["language_profile", "source"])?;
    Ok(Recipe::new(
        string(get(p, "language_profile")?)?,
        bytes(get(p, "source")?)?,
    )?)
}
fn scope(v: &Value) -> Result<Scope> {
    let p = object(v, &["realm", "run", "repo", "worktree", "binding"])?;
    Ok(Scope::new(
        identity(get(p, "realm")?)?,
        identity(get(p, "run")?)?,
        identity(get(p, "repo")?)?,
        identity(get(p, "worktree")?)?,
        identity(get(p, "binding")?)?,
    )?)
}
fn timestamp(v: &Value) -> Result<PosixTimestamp> {
    let p = object(v, &["seconds", "nanoseconds"])?;
    let number = get(p, "seconds")?.as_number().ok_or(Error::Invalid)?;
    if number.as_str().contains(['.', 'e', 'E']) || number.as_str() == "-0" {
        return Err(Error::Invalid);
    }
    Ok(PosixTimestamp::new(
        number.as_i64().ok_or(Error::Invalid)?,
        u32_value(get(p, "nanoseconds")?)?,
    )?)
}
fn metadata(v: &Value) -> Result<PosixMetadata> {
    let p = object(
        v,
        &[
            "device", "inode", "mode", "uid", "gid", "links", "mtime", "ctime",
        ],
    )?;
    Ok(PosixMetadata::new(
        PosixIdentity::new(uint(get(p, "device")?)?, uint(get(p, "inode")?)?),
        u32_value(get(p, "mode")?)?,
        u32_value(get(p, "uid")?)?,
        u32_value(get(p, "gid")?)?,
        uint(get(p, "links")?)?,
        timestamp(get(p, "mtime")?)?,
        timestamp(get(p, "ctime")?)?,
    )?)
}
fn content_bytes(p: &Map<String, Value>) -> Result<Vec<u8>> {
    let content = bytes(get(p, "content")?)?;
    if uint(get(p, "length")?)? != content.len() as u64
        || hash(get(p, "content_sha256")?)? != raw_sha256(&content)
    {
        return Err(Error::Invalid);
    }
    Ok(content)
}
fn capture(v: &Value) -> Result<Capture> {
    let p = object(
        v,
        &[
            "scope",
            "path",
            "content",
            "content_sha256",
            "length",
            "metadata_policy",
            "metadata",
        ],
    )?;
    if text(get(p, "metadata_policy")?)? != PosixMetadata::POLICY {
        return Err(Error::Invalid);
    }
    Ok(Capture::new(
        scope(get(p, "scope")?)?,
        string(get(p, "path")?)?,
        content_bytes(p)?,
        metadata(get(p, "metadata")?)?,
    )?)
}
fn candidate(v: &Value) -> Result<Candidate> {
    let p = object(v, &["capture", "content", "content_sha256", "length"])?;
    Ok(Candidate::new(
        identity(get(p, "capture")?)?,
        content_bytes(p)?,
    ))
}
fn artifact(v: &Value) -> Result<Artifact> {
    let p = object(v, &["profile", "sha256"])?;
    Ok(Artifact::new(
        string(get(p, "profile")?)?,
        hash(get(p, "sha256")?)?,
    )?)
}
fn splice(v: &Value) -> Result<Splice> {
    let p = object(v, &["start", "end", "replacement", "origin"])?;
    let origin = object(get(p, "origin")?, &["start", "end"])?;
    Ok(Splice::new(
        uint(get(p, "start")?)?,
        uint(get(p, "end")?)?,
        bytes(get(p, "replacement")?)?,
        RecipeSpan::new(uint(get(origin, "start")?)?, uint(get(origin, "end")?)?)?,
    )?)
}
fn read(v: &Value) -> Result<ReadRange> {
    let p = object(v, &["start", "bytes"])?;
    Ok(ReadRange::new(
        uint(get(p, "start")?)?,
        bytes(get(p, "bytes")?)?,
    )?)
}
fn list<T>(v: &Value, f: fn(&Value) -> Result<T>) -> Result<Vec<T>> {
    v.as_array().ok_or(Error::Invalid)?.iter().map(f).collect()
}
fn prepared(v: &Value) -> Result<PreparedEdit> {
    let p = object(
        v,
        &[
            "operation",
            "recipe",
            "capture",
            "candidate",
            "compiler",
            "kernel",
            "splices",
            "read_set",
        ],
    )?;
    if text(get(p, "operation")?)? != "replace-content" {
        return Err(Error::Invalid);
    }
    Ok(PreparedEdit::new(
        identity(get(p, "recipe")?)?,
        identity(get(p, "capture")?)?,
        identity(get(p, "candidate")?)?,
        artifact(get(p, "compiler")?)?,
        artifact(get(p, "kernel")?)?,
        list(get(p, "splices")?, splice)?,
        list(get(p, "read_set")?, read)?,
    )?)
}
fn request_key(v: &Value) -> Result<RequestKey> {
    let p = object(v, &["principal", "realm", "run", "opaque_key"])?;
    Ok(RequestKey::new(
        identity(get(p, "principal")?)?,
        identity(get(p, "realm")?)?,
        identity(get(p, "run")?)?,
        bytes(get(p, "opaque_key")?)?,
    )?)
}
fn observation(v: &Value) -> Result<EffectObservation> {
    let p = object(
        v,
        &[
            "intent",
            "stamp",
            "prepared",
            "outcome",
            "content",
            "durability",
            "metadata",
            "observed_sha256",
        ],
    )?;
    let outcome = match text(get(p, "outcome")?)? {
        "confirmed" => Outcome::Confirmed,
        "no-effect" => Outcome::NoEffect,
        "unknown" => Outcome::Unknown,
        "partial" => Outcome::Partial,
        _ => return Err(Error::Invalid),
    };
    let observed = get(p, "observed_sha256")?;
    let digest = if observed.is_null() {
        None
    } else {
        Some(hash(observed)?)
    };
    let content = match (text(get(p, "content")?)?, digest) {
        ("original", Some(hash)) => ContentObservation::Original(hash),
        ("candidate", Some(hash)) => ContentObservation::Candidate(hash),
        ("other", Some(hash)) => ContentObservation::Other(hash),
        ("unobserved", None) => ContentObservation::Unobserved,
        _ => return Err(Error::Invalid),
    };
    let durability = match text(get(p, "durability")?)? {
        "durable" => DurabilityObservation::Durable,
        "not-durable" => DurabilityObservation::NotDurable,
        "unknown" => DurabilityObservation::Unknown,
        _ => return Err(Error::Invalid),
    };
    let metadata = match text(get(p, "metadata")?)? {
        "preserved" => MetadataObservation::Preserved,
        "changed" => MetadataObservation::Changed,
        "unobserved" => MetadataObservation::Unobserved,
        _ => return Err(Error::Invalid),
    };
    Ok(EffectObservation::new(
        identity(get(p, "intent")?)?,
        identity(get(p, "stamp")?)?,
        identity(get(p, "prepared")?)?,
        outcome,
        content,
        durability,
        metadata,
    )?)
}
