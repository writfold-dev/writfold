//! Declarative public identifier languages at Omnigraph 0.14.0.
#[cfg(verus_keep_ghost)]
use super::{Alphabet, range_language};
use vstd::prelude::*;

verus! {

pub(crate) open spec fn graph_identity(bytes: Seq<u8>) -> bool {
    0 < bytes.len() <= 64
        && range_language(bytes, 0, bytes.len() as int, Alphabet::Graph)
        && bytes != seq![b'p', b'o', b'l', b'i', b'c', b'i', b'e', b's']
        && bytes != seq![b'h', b'e', b'a', b'l', b't', b'h', b'z']
        && bytes != seq![b'o', b'p', b'e', b'n', b'a', b'p', b'i']
        && bytes != seq![b'g', b'r', b'a', b'p', b'h', b's']
}

pub(crate) open spec fn ulid_language(bytes: Seq<u8>, start: int, end: int) -> bool {
    0 <= start < end <= bytes.len() && end - start == 26
        && b'0' <= bytes[start] <= b'7'
        && range_language(bytes, start, end, Alphabet::Crockford)
}

pub(crate) open spec fn history_slot_language(bytes: Seq<u8>, start: int, end: int) -> bool {
    0 <= start < end <= bytes.len() && end - start <= 5
        && range_language(bytes, start, end, Alphabet::Digit)
        && (end - start == 1 || bytes[start] != b'0')
        && (end - start < 5 ||
            10000 * (bytes[start] as int - b'0' as int)
                + 1000 * (bytes[start + 1] as int - b'0' as int)
                + 100 * (bytes[start + 2] as int - b'0' as int)
                + 10 * (bytes[start + 3] as int - b'0' as int)
                + (bytes[start + 4] as int - b'0' as int) < 16384)
}

pub(crate) open spec fn graph_commit_identity(bytes: Seq<u8>) -> bool {
    ulid_language(bytes, 0, bytes.len() as int) || (
        59 <= bytes.len() <= 63
            && bytes[0] == b'h' && bytes[1] == b'b' && bytes[2] == b'1' && bytes[3] == b'.'
            && ulid_language(bytes, 4, 30) && bytes[30] == b'.'
            && history_slot_language(bytes, 31, bytes.len() - 27)
            && bytes[bytes.len() - 27] == b'.'
            && ulid_language(bytes, bytes.len() - 26, bytes.len() as int))
}

} // verus!
