//! Declarative byte languages for the immutable Python identity patterns.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy)]
pub(crate) enum Alphabet { Digit, Alphanumeric, SlugEnd, Slug, Token, Hex, Dolt }

pub(crate) open spec fn digit(byte: u8) -> bool { b'0' <= byte <= b'9' }
pub(crate) open spec fn lower(byte: u8) -> bool { b'a' <= byte <= b'z' }
pub(crate) open spec fn alphanumeric(byte: u8) -> bool {
    digit(byte) || lower(byte) || b'A' <= byte <= b'Z'
}
pub(crate) open spec fn slug_end(byte: u8) -> bool { digit(byte) || lower(byte) }
pub(crate) open spec fn alphabet(byte: u8, class: Alphabet) -> bool {
    match class {
        Alphabet::Digit => digit(byte),
        Alphabet::Alphanumeric => alphanumeric(byte),
        Alphabet::SlugEnd => slug_end(byte),
        Alphabet::Slug => slug_end(byte) || byte == b'-',
        Alphabet::Token => alphanumeric(byte) || byte == b'_' || byte == b'.'
            || byte == b':' || byte == b'-',
        Alphabet::Hex => digit(byte) || b'a' <= byte <= b'f',
        Alphabet::Dolt => digit(byte) || b'a' <= byte <= b'v',
    }
}
pub(crate) open spec fn range_language(bytes: Seq<u8>, start: int, end: int, class: Alphabet) -> bool {
    0 <= start <= end <= bytes.len()
        && forall|i: int| start <= i < end ==> alphabet(#[trigger] bytes[i], class)
}
pub(crate) open spec fn token_language(bytes: Seq<u8>) -> bool {
    bytes.len() > 0 && alphanumeric(bytes[0])
        && range_language(bytes, 0, bytes.len() as int, Alphabet::Token)
}
pub(crate) open spec fn slug_language(bytes: Seq<u8>, start: int, end: int) -> bool {
    0 <= start < end <= bytes.len() && slug_end(bytes[start]) && slug_end(bytes[end - 1])
        && range_language(bytes, start, end, Alphabet::Slug)
}

// A relationship is one or more nonempty slugs separated by exactly two
// underscores. Each underscore must belong to a pair between slug-end bytes.
pub(crate) open spec fn relationship_position(bytes: Seq<u8>, i: int) -> bool {
    0 <= i < bytes.len() && (alphabet(bytes[i], Alphabet::Slug) ||
        (bytes[i] == b'_' && (
            (0 < i && i + 2 < bytes.len() && slug_end(bytes[i - 1])
                && bytes[i + 1] == b'_' && slug_end(bytes[i + 2])) ||
            (1 < i && i + 1 < bytes.len() && slug_end(bytes[i - 2])
                && bytes[i - 1] == b'_' && slug_end(bytes[i + 1])))))
}
pub(crate) open spec fn relationship_language(bytes: Seq<u8>) -> bool {
    bytes.len() > 0 && slug_end(bytes[0]) && slug_end(bytes[bytes.len() - 1])
        && forall|i: int| 0 <= i < bytes.len() ==> #[trigger] relationship_position(bytes, i)
}
pub(crate) open spec fn proposal_separator(bytes: Seq<u8>, cut: int) -> bool {
    0 <= cut < bytes.len() && bytes[cut] == b'/'
        && slug_language(bytes, 0, cut)
        && slug_language(bytes, cut + 1, bytes.len() as int)
}
pub(crate) open spec fn proposal_language(bytes: Seq<u8>) -> bool {
    token_language(bytes) || exists|cut: int| proposal_separator(bytes, cut)
}
pub(crate) open spec fn investigation_language(bytes: Seq<u8>) -> bool {
    token_language(bytes) || (
        bytes.len() > 16 && bytes[4] == b'/' && bytes[9] == b'-'
        && bytes[12] == b'-' && bytes[15] == b'-'
        && range_language(bytes, 0, 4, Alphabet::Digit)
        && range_language(bytes, 5, 9, Alphabet::Digit)
        && range_language(bytes, 10, 12, Alphabet::Digit)
        && range_language(bytes, 13, 15, Alphabet::Digit)
        && slug_language(bytes, 16, bytes.len() as int))
}

pub(crate) open spec fn slug_identity(bytes: Seq<u8>) -> bool {
    slug_language(bytes, 0, bytes.len() as int)
}
pub(crate) open spec fn digest_identity(bytes: Seq<u8>) -> bool {
    bytes.len() == 64 && range_language(bytes, 0, 64, Alphabet::Hex)
}
pub(crate) open spec fn release_identity(bytes: Seq<u8>) -> bool {
    (bytes.len() == 40 || bytes.len() == 64)
        && range_language(bytes, 0, bytes.len() as int, Alphabet::Hex)
}
pub(crate) open spec fn dolt_identity(bytes: Seq<u8>) -> bool {
    bytes.len() == 32 && range_language(bytes, 0, 32, Alphabet::Dolt)
}
pub(crate) open spec fn opaque_identity(bytes: Seq<u8>) -> bool { bytes.len() > 0 }

} // verus!
