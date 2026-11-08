use serde_json::Value;
use writfold_domain::engineering::ledger::{
    LedgerHead, LedgerMaterial, LedgerOwner, LedgerPosition, ReducerProfile, StreamKind,
};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest, LedgerEventDigest};
use writfold_domain::{ByteCount, LedgerSequence, OwnershipGeneration};

pub type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
pub struct Golden {
    pub name: String,
    pub head: LedgerHead,
    pub bytes: Vec<u8>,
    pub checksum: Vec<u8>,
}
pub struct Refusal {
    pub name: String,
    pub bytes: Vec<u8>,
}

pub fn fixtures() -> Result<(Vec<Golden>, Vec<Refusal>)> {
    let source: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/qualification/ledger-head-v1.json"
    ))?;
    let shared = &source["shared"];
    let event: [u8; 32] = hex(text(shared, "event_digest_hex")?)?
        .try_into()
        .map_err(|_| "event fixture width")?;
    let blob: [u8; 32] = hex(text(shared, "blob_digest_hex")?)?
        .try_into()
        .map_err(|_| "blob fixture width")?;
    let positive = source["vectors"]
        .as_array()
        .ok_or("missing golden vectors")?
        .iter()
        .map(|row| {
            let kind = match text(row, "stream_kind")? {
                "control" => StreamKind::Control,
                "run" => StreamKind::Run,
                "learning" => StreamKind::Learning,
                _ => return Err("unknown fixture stream kind".into()),
            };
            let position = LedgerPosition::new(
                text(shared, "realm")?.parse()?,
                text(shared, "stream")?.parse()?,
                kind,
                LedgerSequence::new(number(row, "sequence")?)?,
            );
            let material = LedgerMaterial::new(
                text(shared, "record")?.parse()?,
                LedgerEventDigest::from_digest(event),
                BlobDigest::from_digest(blob),
                ByteCount::new(number(row, "record_bytes")?)?,
            );
            let owner = LedgerOwner::new(
                text(shared, "instance")?.parse()?,
                OwnershipGeneration::new(number(row, "ownership_generation")?)?,
            );
            Ok(Golden {
                name: text(row, "name")?.to_owned(),
                head: LedgerHead::new(position, material, ReducerProfile::EngineeringV1, owner),
                bytes: hex(text(row, "canonical_cbor_hex")?)?,
                checksum: hex(text(row, "raw_sha256")?)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let negative = source["negative_vectors"]
        .as_array()
        .ok_or("missing refusal vectors")?
        .iter()
        .map(|row| {
            Ok(Refusal {
                name: text(row, "name")?.to_owned(),
                bytes: hex(text(row, "canonical_cbor_hex")?)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((positive, negative))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing fixture text {key}").into())
}
fn number(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| format!("missing fixture integer {key}").into())
}
fn hex(text: &str) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return Err("incomplete fixture hex byte".into());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}
