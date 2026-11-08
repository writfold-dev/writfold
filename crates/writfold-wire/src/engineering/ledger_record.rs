//! Closed canonical bytes for one validated-ledger record (`writfold-local-ledger/1`).
//!
//! `[namespace, version, id, stream, sequence, [predecessor?], profile,
//!   payload bytes, historical bytes, [closure digest..]]`. The payload and
//! historical premises are opaque canonical bytes supplied by a `Part`
//! implementation; decoding requires them to re-encode identically. Decoding
//! establishes structure only: linkage, retention and validity are the
//! kernel's `history_check`.
use super::cbor::{self, Decoder, Value as C};
use crate::Limits;
use writfold_domain::LedgerSequence;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_kernel::ledger::Record;

pub const NAMESPACE: &str = "writfold-ledger-record";
pub const SCHEMA_VERSION: u64 = 1;
const FIELDS: usize = 10;
const ENGINEERING_V1: u64 = 1;

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
    #[error("invalid ledger record field: {0}")]
    Field(&'static str),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A closed record part with exactly one canonical byte spelling.
pub trait Part: Sized {
    fn encode(&self, limits: Limits) -> Result<Vec<u8>>;
    fn decode(bytes: &[u8], limits: Limits) -> Result<Self>;
}

pub fn encode<P: Part, H: Part>(record: &Record<P, H>, limits: Limits) -> Result<Vec<u8>> {
    let payload = record.payload.encode(limits)?;
    let historical = record.historical.encode(limits)?;
    let predecessor: Vec<C<'_>> = record
        .predecessor
        .iter()
        .map(|p| C::Text(p.as_str()))
        .collect();
    let closure: Vec<C<'_>> = record
        .closure
        .iter()
        .map(|d| C::Bytes(d.as_digest()))
        .collect();
    let fields = [
        C::Text(NAMESPACE),
        C::Unsigned(SCHEMA_VERSION),
        C::Text(record.id.as_str()),
        C::Text(record.stream.as_str()),
        C::Unsigned(record.sequence.get()),
        C::Array(&predecessor),
        C::Unsigned(match record.profile {
            ReducerProfile::EngineeringV1 => ENGINEERING_V1,
        }),
        C::Bytes(&payload),
        C::Bytes(&historical),
        C::Array(&closure),
    ];
    Ok(cbor::encode(&C::Array(&fields), limits)?.into_bytes())
}

/// Decode, then require the record to re-encode to exactly these bytes.
pub fn decode<P: Part, H: Part>(bytes: &[u8], limits: Limits) -> Result<Record<P, H>> {
    let record = decode_fields(bytes, limits)?;
    if encode(&record, limits)? != bytes {
        return Err(Error::Field("noncanonical record"));
    }
    Ok(record)
}

fn decode_fields<P: Part, H: Part>(bytes: &[u8], limits: Limits) -> Result<Record<P, H>> {
    let mut d = Decoder::new(bytes, limits)?;
    if d.array()? != FIELDS {
        return Err(Error::Field("field count"));
    }
    if d.text()? != NAMESPACE {
        return Err(Error::Field("namespace"));
    }
    if d.unsigned()? != SCHEMA_VERSION {
        return Err(Error::Field("schema version"));
    }
    let id = d.text()?.parse()?;
    let stream = d.text()?.parse()?;
    let sequence = LedgerSequence::new(d.unsigned()?)?;
    let predecessor = match d.array()? {
        0 => None,
        1 => Some(d.text()?.parse()?),
        _ => return Err(Error::Field("predecessor")),
    };
    d.end_array()?;
    let profile = match d.unsigned()? {
        ENGINEERING_V1 => ReducerProfile::EngineeringV1,
        _ => return Err(Error::Field("reducer profile")),
    };
    let payload = P::decode(d.bytes()?, limits)?;
    let historical = H::decode(d.bytes()?, limits)?;
    let mut closure = Vec::new();
    for _ in 0..d.array()? {
        let digest: [u8; 32] = d
            .bytes()?
            .try_into()
            .map_err(|_| Error::Field("closure digest"))?;
        closure.push(BlobDigest::from_digest(digest));
    }
    d.end_array()?;
    d.end_array()?;
    d.finish()?;
    Ok(Record {
        id,
        stream,
        sequence,
        predecessor,
        profile,
        payload,
        historical,
        closure,
    })
}
