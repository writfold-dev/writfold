//! Pure, IO-free lexical lookup over a complete pinned snapshot. Backends that
//! materialize a bounded snapshot (Dolt in B0) share it; a backend with a native
//! index must still return identical pages for the same pinned records.
use super::{
    Capability, LookupCursor, LookupField, LookupHit, LookupPage, LookupRequest, MatchMode, Order,
    RecordKey, RecordKind, Refusal,
};
use sha2::{Digest, Sha256};
use writfold_domain::ContentDigest;
use writfold_domain::citation::Coverage;

/// One record of a complete pinned snapshot, borrowed from the backend's rows.
#[derive(Clone, Copy, Debug)]
pub struct ScanRecord<'a> {
    pub key: &'a str,
    pub kind: &'a str,
    pub content: &'a [u8],
}

/// SHA-256 of exact retained bytes, as a lowercase hexadecimal content digest.
pub fn content_digest(bytes: &[u8]) -> Result<ContentDigest, Refusal> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 15)]));
    }
    ContentDigest::try_from(encoded).map_err(|_| Refusal::Backend("content digest encoding".into()))
}

/// Refuse unsupported modes and foreign cursors before any backend IO.
pub fn admit_lookup<P: Eq>(
    pin: &P,
    request: &LookupRequest,
    cursor: Option<&LookupCursor<P>>,
    capabilities: &[Capability],
) -> Result<(), Refusal> {
    let needed = match request.mode() {
        MatchMode::Exact => Capability::ExactLookup,
        MatchMode::Prefix | MatchMode::Substring => Capability::LexicalLookup,
        MatchMode::Fuzzy => Capability::Fuzzy,
        MatchMode::Semantic => Capability::Semantic,
    };
    if !capabilities.contains(&needed) {
        return Err(Refusal::Unsupported(needed));
    }
    if request.order() == Order::Ranked && !capabilities.contains(&Capability::Ranking) {
        return Err(Refusal::Unsupported(Capability::Ranking));
    }
    if let Some(cursor) = cursor
        && (cursor.pin() != pin || cursor.request() != request)
    {
        return Err(Refusal::CursorMismatch);
    }
    Ok(())
}

/// Scan records in strict binary key order. Stops after `limit` hits; coverage
/// counts every record examined from the start of the key order, so a resumed
/// page reports the cumulative scan, and complete coverage means no cursor.
pub fn scan<P: Clone + Eq>(
    pin: &P,
    records: &[ScanRecord<'_>],
    request: &LookupRequest,
    cursor: Option<&LookupCursor<P>>,
) -> Result<LookupPage<P>, Refusal> {
    admit_lookup(
        pin,
        request,
        cursor,
        &[Capability::ExactLookup, Capability::LexicalLookup],
    )?;
    if records
        .windows(2)
        .any(|pair| pair[0].key.as_bytes() >= pair[1].key.as_bytes())
    {
        return Err(Refusal::Backend(
            "snapshot records are not in strict key order".into(),
        ));
    }
    let start = cursor.map_or(0, |cursor| {
        records
            .partition_point(|record| record.key.as_bytes() <= cursor.after().as_str().as_bytes())
    });
    let mut hits = Vec::new();
    let mut examined = start;
    for record in records.iter().skip(start) {
        examined += 1;
        if matches(record, request) {
            hits.push(hit(record)?);
            if hits.len() == request.limit() {
                break;
            }
        }
    }
    let total = records.len();
    let next = if examined < total {
        let last = records
            .get(examined.wrapping_sub(1))
            .ok_or_else(|| Refusal::Backend("scan position outside snapshot".into()))?;
        Some(LookupCursor::new(
            pin.clone(),
            request.clone(),
            backend_key(last.key)?,
        ))
    } else {
        None
    };
    let coverage = Coverage::Measured {
        examined,
        total: total as u128,
    };
    Ok(LookupPage::new(pin.clone(), hits, coverage, next))
}

fn matches(record: &ScanRecord<'_>, request: &LookupRequest) -> bool {
    let value = match request.field() {
        LookupField::Key => record.key.as_bytes(),
        LookupField::Kind => record.kind.as_bytes(),
        LookupField::Content => record.content,
    };
    let needle = request.needle().as_bytes();
    match request.mode() {
        MatchMode::Exact => value == needle,
        MatchMode::Prefix => value.starts_with(needle),
        MatchMode::Substring => value.windows(needle.len()).any(|window| window == needle),
        MatchMode::Fuzzy | MatchMode::Semantic => false,
    }
}

fn hit(record: &ScanRecord<'_>) -> Result<LookupHit, Refusal> {
    let kind = RecordKind::new(record.kind.to_owned())
        .map_err(|_| Refusal::Backend("record kind outside the boundary".into()))?;
    LookupHit::new(backend_key(record.key)?, kind, record.content)
}

fn backend_key(key: &str) -> Result<RecordKey, Refusal> {
    RecordKey::new(key.to_owned())
        .map_err(|_| Refusal::Backend("record key outside the boundary".into()))
}
