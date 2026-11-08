//! Closed canonical ledger-head bytes. Structural decoding does not validate
//! the referenced record, historical prefix, origin, retention or live ownership.
use super::cbor::{self, Decoder, Value as C};
use crate::Limits;
use writfold_domain::engineering::ledger::{
    LedgerHead, LedgerMaterial, LedgerOwner, LedgerPosition, ReducerProfile, StreamKind,
};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest, LedgerEventDigest};
use writfold_domain::{ByteCount, LedgerSequence, OwnershipGeneration};

pub const NAMESPACE: &str = "writfold-engineering";
pub const HEAD_KIND: &str = "ledger-head";
pub const REDUCER_PROFILE: &str = "writfold-engineering/1";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Decode(#[from] cbor::DecodeError),
    #[error(transparent)]
    Encode(#[from] cbor::Error),
    #[error(transparent)]
    Identity(#[from] writfold_domain::IdentityError),
    #[error(transparent)]
    Quantity(#[from] writfold_domain::QuantityError),
    #[error("invalid ledger-head field: {0}")]
    Field(&'static str),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// No source strings or fingerprints are copied into an intermediate value tree.
pub fn encode_head(head: &LedgerHead, limits: Limits) -> Result<cbor::RawCbor<'static>> {
    let position = head.position();
    let material = head.material();
    let owner = head.owner();
    let position_fields = [
        ("realm", C::Text(position.realm().as_str())),
        ("stream", C::Text(position.stream().as_str())),
        ("kind", C::Text(stream_kind(*position.kind()))),
        ("sequence", C::Unsigned(position.sequence().get())),
    ];
    let material_fields = [
        ("record", C::Text(material.record().as_str())),
        ("event", C::Bytes(material.event().as_digest())),
        ("blob", C::Bytes(material.blob().as_digest())),
        ("bytes", C::Unsigned(material.bytes().get())),
    ];
    let owner_fields = [
        ("instance", C::Text(owner.instance().as_str())),
        ("generation", C::Unsigned(owner.generation().get())),
    ];
    let profile = match head.profile() {
        ReducerProfile::EngineeringV1 => REDUCER_PROFILE,
    };
    let root = [
        ("namespace", C::Text(NAMESPACE)),
        ("schema_version", C::Unsigned(1)),
        ("kind", C::Text(HEAD_KIND)),
        ("position", C::Map(&position_fields)),
        ("material", C::Map(&material_fields)),
        ("owner", C::Map(&owner_fields)),
        ("profile", C::Text(profile)),
    ];
    Ok(cbor::encode(&C::Map(&root), limits)?)
}

/// Require every schema field, canonical key order, complete container endings
/// and full input consumption before returning a structurally typed candidate.
pub fn decode_head(bytes: &[u8], limits: Limits) -> Result<LedgerHead> {
    let mut decoder = Decoder::new(bytes, limits)?;
    map(&mut decoder, 7)?;
    key(&mut decoder, "kind")?;
    text(&mut decoder, HEAD_KIND, "kind")?;
    key(&mut decoder, "owner")?;
    let owner = decode_owner(&mut decoder)?;
    key(&mut decoder, "profile")?;
    text(&mut decoder, REDUCER_PROFILE, "profile")?;
    key(&mut decoder, "material")?;
    let material = decode_material(&mut decoder)?;
    key(&mut decoder, "position")?;
    let position = decode_position(&mut decoder)?;
    key(&mut decoder, "namespace")?;
    text(&mut decoder, NAMESPACE, "namespace")?;
    key(&mut decoder, "schema_version")?;
    if decoder.unsigned()? != 1 {
        return Err(Error::Field("schema_version"));
    }
    decoder.end_map()?;
    decoder.finish()?;
    Ok(LedgerHead::new(
        position,
        material,
        ReducerProfile::EngineeringV1,
        owner,
    ))
}

fn decode_position(decoder: &mut Decoder<'_>) -> Result<LedgerPosition> {
    map(decoder, 4)?;
    key(decoder, "kind")?;
    let kind = match decoder.text()? {
        "control" => StreamKind::Control,
        "run" => StreamKind::Run,
        "learning" => StreamKind::Learning,
        _ => return Err(Error::Field("stream kind")),
    };
    key(decoder, "realm")?;
    let realm = decoder.text()?.parse()?;
    key(decoder, "stream")?;
    let stream = decoder.text()?.parse()?;
    key(decoder, "sequence")?;
    let sequence = LedgerSequence::new(decoder.unsigned()?)?;
    decoder.end_map()?;
    Ok(LedgerPosition::new(realm, stream, kind, sequence))
}

fn decode_material(decoder: &mut Decoder<'_>) -> Result<LedgerMaterial> {
    map(decoder, 4)?;
    key(decoder, "blob")?;
    let blob = BlobDigest::from_digest(digest(decoder)?);
    key(decoder, "bytes")?;
    let bytes = ByteCount::new(decoder.unsigned()?)?;
    key(decoder, "event")?;
    let event = LedgerEventDigest::from_digest(digest(decoder)?);
    key(decoder, "record")?;
    let record = decoder.text()?.parse()?;
    decoder.end_map()?;
    Ok(LedgerMaterial::new(record, event, blob, bytes))
}

fn decode_owner(decoder: &mut Decoder<'_>) -> Result<LedgerOwner> {
    map(decoder, 2)?;
    key(decoder, "instance")?;
    let instance = decoder.text()?.parse()?;
    key(decoder, "generation")?;
    let generation = OwnershipGeneration::new(decoder.unsigned()?)?;
    decoder.end_map()?;
    Ok(LedgerOwner::new(instance, generation))
}

fn map(decoder: &mut Decoder<'_>, count: usize) -> Result<()> {
    if decoder.map()? != count {
        return Err(Error::Field("map fields"));
    }
    Ok(())
}
fn key(decoder: &mut Decoder<'_>, expected: &'static str) -> Result<()> {
    if decoder.key()? != expected {
        return Err(Error::Field(expected));
    }
    Ok(())
}
fn text(decoder: &mut Decoder<'_>, expected: &str, field: &'static str) -> Result<()> {
    if decoder.text()? != expected {
        return Err(Error::Field(field));
    }
    Ok(())
}
fn digest(decoder: &mut Decoder<'_>) -> Result<[u8; 32]> {
    decoder
        .bytes()?
        .try_into()
        .map_err(|_| Error::Field("fingerprint bytes"))
}
fn stream_kind(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Control => "control",
        StreamKind::Run => "run",
        StreamKind::Learning => "learning",
    }
}
