//! Byte-exact ASCII grammars; opaque identifiers preserve arbitrary UTF-8.
pub(crate) mod spec;
mod storage;
use spec::Alphabet;
#[cfg(verus_keep_ghost)]
use spec::*;
pub(crate) use storage::{graph, graph_commit};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

// One modelled range primitive covers ASCII and Dolt's wider base-32 alphabet.
#[inline]
fn byte_in_range(byte: u8, first: u8, last: u8) -> (accepted: bool)
    ensures accepted == (first <= byte <= last),
{
    (first..=last).contains(&byte)
}

fn byte_matches(byte: u8, class: Alphabet) -> (accepted: bool)
    ensures accepted == alphabet(byte, class),
{
    let digit = byte_in_range(byte, b'0', b'9');
    let lower = byte_in_range(byte, b'a', b'z');
    let alphanumeric = digit || lower || byte_in_range(byte, b'A', b'Z');
    match class {
        Alphabet::Digit => digit,
        Alphabet::Alphanumeric => alphanumeric,
        Alphabet::SlugEnd => digit || lower,
        Alphabet::Slug => digit || lower || byte == b'-',
        Alphabet::Token => alphanumeric || byte == b'_' || byte == b'.'
            || byte == b':' || byte == b'-',
        Alphabet::Hex => digit || byte_in_range(byte, b'a', b'f'),
        Alphabet::Dolt => digit || byte_in_range(byte, b'a', b'v'),
        Alphabet::Graph => alphanumeric || byte == b'-',
        Alphabet::Crockford => digit || byte_in_range(byte, b'A', b'H')
            || byte_in_range(byte, b'J', b'K') || byte_in_range(byte, b'M', b'N')
            || byte_in_range(byte, b'P', b'T') || byte_in_range(byte, b'V', b'Z'),
    }
}

fn range_matches(bytes: &[u8], start: usize, end: usize, class: Alphabet) -> (accepted: bool)
    requires start <= end <= bytes.len(),
    ensures accepted == range_language(bytes@, start as int, end as int, class),
{
    let mut index = start;
    while index < end
        invariant
            start <= index <= end <= bytes.len(),
            forall|i: int| start <= i < index ==> alphabet(#[trigger] bytes@[i], class),
        decreases end - index,
    {
        if !byte_matches(bytes[index], class) { return false; }
        index += 1;
    }
    true
}

pub(crate) fn token(value: &str) -> (accepted: bool)
    ensures accepted == token_language(value.spec_bytes()),
{
    let bytes = value.as_bytes();
    !bytes.is_empty() && byte_matches(bytes[0], Alphabet::Alphanumeric)
        && range_matches(bytes, 0, bytes.len(), Alphabet::Token)
}

fn slug_part(bytes: &[u8], start: usize, end: usize) -> (accepted: bool)
    requires start <= end <= bytes.len(),
    ensures accepted == slug_language(bytes@, start as int, end as int),
{
    start < end && byte_matches(bytes[start], Alphabet::SlugEnd)
        && byte_matches(bytes[end - 1], Alphabet::SlugEnd)
        && range_matches(bytes, start, end, Alphabet::Slug)
}

pub(crate) fn slug(value: &str) -> (accepted: bool)
    ensures accepted == slug_language(value.spec_bytes(), 0, value.spec_bytes().len() as int),
{
    let bytes = value.as_bytes();
    slug_part(bytes, 0, bytes.len())
}

fn relationship_byte(bytes: &[u8], index: usize) -> (accepted: bool)
    requires index < bytes.len(),
    ensures accepted == relationship_position(bytes@, index as int),
{
    if byte_matches(bytes[index], Alphabet::Slug) { return true; }
    if bytes[index] != b'_' { return false; }
    let length = bytes.len();
    let leading = index > 0 && length > 2 && index < length - 2
        && byte_matches(bytes[index - 1], Alphabet::SlugEnd)
        && bytes[index + 1] == b'_'
        && byte_matches(bytes[index + 2], Alphabet::SlugEnd);
    let trailing = index > 1 && index < length - 1
        && byte_matches(bytes[index - 2], Alphabet::SlugEnd)
        && bytes[index - 1] == b'_'
        && byte_matches(bytes[index + 1], Alphabet::SlugEnd);
    leading || trailing
}

pub(crate) fn relationship(value: &str) -> (accepted: bool)
    ensures accepted == relationship_language(value.spec_bytes()),
{
    let bytes = value.as_bytes();
    if bytes.is_empty() || !byte_matches(bytes[0], Alphabet::SlugEnd)
        || !byte_matches(bytes[bytes.len() - 1], Alphabet::SlugEnd) { return false; }
    let mut index = 0;
    while index < bytes.len()
        invariant
            index <= bytes.len(),
            bytes.len() > 0,
            bytes@ == value.spec_bytes(),
            slug_end(bytes@[0]), slug_end(bytes@[bytes.len() - 1]),
            forall|i: int| 0 <= i < index ==> #[trigger] relationship_position(bytes@, i),
        decreases bytes.len() - index,
    {
        if !relationship_byte(bytes, index) {
            proof {
                assert(!relationship_position(value.spec_bytes(), index as int));
                assert(!relationship_language(value.spec_bytes()));
            }
            return false;
        }
        index += 1;
    }
    true
}

pub(crate) fn proposal(value: &str) -> (accepted: bool)
    ensures accepted == proposal_language(value.spec_bytes()),
{
    if token(value) { return true; }
    let bytes = value.as_bytes();
    let mut cut = 0;
    while cut < bytes.len() && bytes[cut] != b'/'
        invariant
            cut <= bytes.len(),
            !token_language(bytes@),
            forall|i: int| 0 <= i < cut ==> bytes@[i] != b'/',
        decreases bytes.len() - cut,
    { cut += 1; }
    if cut == bytes.len() { return false; }
    let accepted = slug_part(bytes, 0, cut) && slug_part(bytes, cut + 1, bytes.len());
    proof {
        assert forall|other: int| proposal_separator(bytes@, other) implies other == cut by {
            if other < cut {
                assert(bytes@[other] != b'/');
            } else if other > cut {
                assert(range_language(bytes@, 0, other, Alphabet::Slug));
                assert(alphabet(bytes@[cut as int], Alphabet::Slug));
            }
        }
        assert(accepted == proposal_separator(bytes@, cut as int));
        if exists|other: int| proposal_separator(bytes@, other) {
            let other = choose|other: int| proposal_separator(bytes@, other);
            assert(other == cut);
        }
    }
    accepted
}

pub(crate) fn investigation(value: &str) -> (accepted: bool)
    ensures accepted == investigation_language(value.spec_bytes()),
{
    if token(value) { return true; }
    let bytes = value.as_bytes();
    bytes.len() > 16 && bytes[4] == b'/' && bytes[9] == b'-'
        && bytes[12] == b'-' && bytes[15] == b'-'
        && range_matches(bytes, 0, 4, Alphabet::Digit)
        && range_matches(bytes, 5, 9, Alphabet::Digit)
        && range_matches(bytes, 10, 12, Alphabet::Digit)
        && range_matches(bytes, 13, 15, Alphabet::Digit)
        && slug_part(bytes, 16, bytes.len())
}

pub(crate) fn opaque(value: &str) -> (accepted: bool)
    ensures accepted == (value.spec_bytes().len() > 0),
{
    !value.is_empty()
}

pub(crate) fn lower_hex(value: &str, size: usize) -> (accepted: bool)
    ensures accepted == (value.spec_bytes().len() == size
        && range_language(value.spec_bytes(), 0, size as int, Alphabet::Hex)),
{
    let bytes = value.as_bytes();
    bytes.len() == size && range_matches(bytes, 0, size, Alphabet::Hex)
}

pub(crate) fn digest(value: &str) -> (accepted: bool)
    ensures accepted == (value.spec_bytes().len() == 64
        && range_language(value.spec_bytes(), 0, 64, Alphabet::Hex)),
{
    lower_hex(value, 64)
}

pub(crate) fn release(value: &str) -> (accepted: bool)
    ensures accepted == ((value.spec_bytes().len() == 40 || value.spec_bytes().len() == 64)
        && range_language(value.spec_bytes(), 0, value.spec_bytes().len() as int, Alphabet::Hex)),
{
    lower_hex(value, 40) || lower_hex(value, 64)
}

pub(crate) fn dolt(value: &str) -> (accepted: bool)
    ensures accepted == (value.spec_bytes().len() == 32
        && range_language(value.spec_bytes(), 0, 32, Alphabet::Dolt)),
{
    let bytes = value.as_bytes();
    bytes.len() == 32 && range_matches(bytes, 0, 32, Alphabet::Dolt)
}

} // verus!
