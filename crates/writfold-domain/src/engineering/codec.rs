use super::FingerprintError;
use std::fmt;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {
pub(super) open spec fn hex(byte: u8) -> bool {
    b'0' <= byte <= b'9' || b'a' <= byte <= b'f'
}

pub(super) open spec fn nibble_value(byte: u8) -> int {
    if b'0' <= byte <= b'9' { byte - b'0' } else { byte - b'a' + 10 }
}

pub(super) open spec fn accepts(bytes: Seq<u8>, prefix: Seq<u8>) -> bool {
    bytes.len() == prefix.len() + 64
        && (forall|i: int| 0 <= i < prefix.len() ==> bytes[i] == #[trigger] prefix[i])
        && (forall|i: int| prefix.len() <= i < bytes.len() ==> hex(#[trigger] bytes[i]))
}

pub(super) open spec fn decoded(bytes: Seq<u8>, prefix: Seq<u8>, digest: Seq<u8>) -> bool {
    digest.len() == 32 && forall|i: int| 0 <= i < 32 ==> #[trigger] digest[i] ==
        16 * nibble_value(bytes[prefix.len() + 2 * i])
            + nibble_value(bytes[prefix.len() + 2 * i + 1])
}

fn nibble(byte: u8) -> (result: Option<u8>)
    ensures
        result.is_some() <==> hex(byte),
        result.is_some() ==> result.unwrap() <= 15,
        result.is_some() ==> result.unwrap() as int == nibble_value(byte),
{
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

pub(super) fn parse(text: &str, prefix: &[u8]) -> (result: Result<[u8; 32], FingerprintError>)
    requires prefix.len() <= 64,
    ensures
        result.is_ok() <==> accepts(text.spec_bytes(), prefix@),
        result.is_ok() ==> decoded(text.spec_bytes(), prefix@, result.unwrap()@),
{
    let bytes = text.as_bytes();
    let start = prefix.len();
    if bytes.len() != start + 64 { return Err(FingerprintError); }
    let mut at = 0;
    while at < start
        invariant
            start == prefix.len(), start <= 64, bytes.len() == start + 64,
            bytes@ == text.spec_bytes(), 0 <= at <= start,
            forall|i: int| 0 <= i < at ==> bytes[i] == #[trigger] prefix[i],
        decreases start - at,
    {
        if bytes[at] != prefix[at] { return Err(FingerprintError); }
        at += 1;
    }
    let mut digest = [0u8; 32];
    let mut index = 0;
    while index < 32
        invariant
            start == prefix.len(), start <= 64, bytes.len() == start + 64,
            bytes@ == text.spec_bytes(), 0 <= index <= 32,
            forall|i: int| 0 <= i < start ==> bytes[i] == #[trigger] prefix[i],
            forall|i: int| start <= i < start + 2 * index ==> hex(#[trigger] bytes[i]),
            forall|i: int| 0 <= i < index ==> #[trigger] digest[i] ==
                16 * nibble_value(bytes[start + 2 * i]) + nibble_value(bytes[start + 2 * i + 1]),
        decreases 32 - index,
    {
        let offset = start + 2 * index;
        let high = match nibble(bytes[offset]) {
            Some(value) => value,
            None => { return Err(FingerprintError); }
        };
        let low = match nibble(bytes[offset + 1]) {
            Some(value) => value,
            None => { return Err(FingerprintError); }
        };
        digest[index] = high * 16 + low;
        proof {
            assert forall|i: int| start <= i < start + 2 * (index + 1)
                implies hex(#[trigger] bytes[i]) by {
                if i >= offset { assert(i == offset || i == offset + 1); }
            }
        }
        index += 1;
    }
    Ok(digest)
}
} // verus!

// Formatting follows the existing domain adapter boundary; no temporary String.
pub(super) fn display(prefix: &str, digest: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    const HEX: [&str; 16] = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "b", "c", "d", "e", "f",
    ];
    f.write_str(prefix)?;
    for byte in digest {
        f.write_str(HEX[(byte >> 4) as usize])?;
        f.write_str(HEX[(byte & 15) as usize])?;
    }
    Ok(())
}
