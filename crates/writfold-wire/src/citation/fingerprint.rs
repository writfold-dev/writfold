use super::{CitationDescriptor, CitationWireError, GitSource, Result, algorithm};
use crate::{
    Limits,
    engineering::cbor::{self, RawCbor, Value},
};
use sha2::{Digest, Sha256};
use writfold_domain::{
    CitationBindingDigest, CitationCorrespondenceDigest, CitationRepresentationDigest,
    ContentDigest,
};

fn hex(digest: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    text
}

fn separated<T: TryFrom<String>>(prefix: &[u8], payload: &RawCbor<'_>) -> Result<T> {
    let mut hash = Sha256::new();
    hash.update(prefix);
    hash.update(payload.as_bytes());
    T::try_from(hex(&hash.finalize())).map_err(|_| CitationWireError::Identity("fingerprint"))
}

/// Ordinary SHA-256 of exact selected bytes, not a representation/binding ID and
/// not the engineering/1 domain-separated blob fingerprint.
pub fn selection_digest(bytes: &[u8]) -> Result<ContentDigest> {
    ContentDigest::try_from(hex(&Sha256::digest(bytes)))
        .map_err(|_| CitationWireError::Identity("selection_digest"))
}

fn native<'a>(source: &GitSource, value: &'a str) -> [(&'static str, Value<'a>); 2] {
    [
        ("algorithm", Value::Text(algorithm(source.format()))),
        ("value", Value::Text(value)),
    ]
}

fn representation<'a>(
    source: &'a GitSource,
    revision: &'a [(&'static str, Value<'a>)],
    blob: &'a [(&'static str, Value<'a>)],
) -> [(&'static str, Value<'a>); 9] {
    let reference = source.reference();
    [
        ("schema_version", Value::Unsigned(1)),
        ("realm", Value::Text(reference.realm().as_str())),
        ("repository", Value::Text(reference.repository().as_str())),
        ("revision", Value::Map(revision)),
        ("path", Value::Text(reference.path())),
        ("blob", Value::Map(blob)),
        ("capture_digest", Value::Text(reference.content().as_str())),
        ("profile", Value::Text(super::GIT_BLOB_PROFILE)),
        ("kind", Value::Text("representation")),
    ]
}

/// Hash the exact native algorithm/provenance/profile material. Access, physical
/// locator paths, alias tables and future graph commits are not manufactured.
pub fn representation_fingerprint(
    source: &GitSource,
    limits: Limits,
) -> Result<CitationRepresentationDigest> {
    let revision = native(source, source.reference().revision().as_str());
    let blob = native(source, source.blob().as_str());
    let fields = representation(source, &revision, &blob);
    let payload = cbor::encode(&Value::Map(&fields), limits)?;
    separated(b"writfold-citation/1/representation\0", &payload)
}

/// The logical CitationId is an independent input, not this digest: the preimage
/// contains neither its resulting fingerprint, a signature nor a future release.
/// Evidence role, context, exact coordinates and selection digest remain material.
pub fn binding_fingerprint(
    descriptor: &CitationDescriptor,
    limits: Limits,
) -> Result<CitationBindingDigest> {
    let source = descriptor.source();
    let revision = native(source, source.reference().revision().as_str());
    let blob = native(source, source.blob().as_str());
    let representation = representation(source, &revision, &blob);
    let selector = [
        ("unit", Value::Text("byte")),
        (
            "first",
            Value::Unsigned(descriptor.selector().first().get()),
        ),
        ("last", Value::Unsigned(descriptor.selector().last().get())),
    ];
    let fields = [
        ("schema_version", Value::Unsigned(1)),
        ("kind", Value::Text("binding")),
        ("id", Value::Text(descriptor.id().as_str())),
        ("release", Value::Text(descriptor.release().as_str())),
        ("context", Value::Text(descriptor.context().as_str())),
        ("role", Value::Text(descriptor.role().as_str())),
        ("representation", Value::Map(&representation)),
        ("selector", Value::Map(&selector)),
        (
            "selection_digest",
            Value::Text(descriptor.selection_digest().as_str()),
        ),
    ];
    let payload = cbor::encode(&Value::Map(&fields), limits)?;
    separated(b"writfold-citation/1/binding\0", &payload)
}

/// Byte-boundary primitive, like engineering::fingerprint: the payload must be
/// canonical CBOR, but schema, statement, source closure and actual computation
/// are the producer's separate obligations. Hashing cannot certify a comparison.
/// No authoritative correspondence-record writer is introduced by this function.
pub fn correspondence_payload_fingerprint(
    payload: &RawCbor<'_>,
) -> Result<CitationCorrespondenceDigest> {
    separated(b"writfold-citation/1/correspondence\0", payload)
}
