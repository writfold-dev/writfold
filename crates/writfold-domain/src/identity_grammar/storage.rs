//! Omnigraph 0.14 public identities, validated without interpreting their meaning.
use super::spec::Alphabet;
#[cfg(verus_keep_ghost)]
use super::spec::*;
use super::{byte_in_range, range_matches};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

fn same_bytes(left: &[u8], right: &[u8]) -> (equal: bool)
    ensures equal == (left@ == right@),
{
    if left.len() != right.len() { return false; }
    let mut index = 0;
    while index < left.len()
        invariant
            left.len() == right.len(), index <= left.len(),
            forall|i: int| 0 <= i < index ==> left@[i] == right@[i],
        decreases left.len() - index,
    {
        if left[index] != right[index] { return false; }
        index += 1;
    }
    proof { assert(left@ =~= right@); }
    true
}

pub(crate) fn graph(value: &str) -> (accepted: bool)
    ensures accepted == graph_identity(value.spec_bytes()),
{
    let bytes = value.as_bytes();
    proof {
        broadcast use vstd::string::group_string_axioms;
        reveal_strlit("policies");
        reveal_strlit("healthz");
        reveal_strlit("openapi");
        reveal_strlit("graphs");
        assert(vstd::string::is_ascii("policies"));
        assert(vstd::string::is_ascii("healthz"));
        assert(vstd::string::is_ascii("openapi"));
        assert(vstd::string::is_ascii("graphs"));
        assert("policies".spec_bytes() =~= seq![b'p', b'o', b'l', b'i', b'c', b'i', b'e', b's']);
        assert("healthz".spec_bytes() =~= seq![b'h', b'e', b'a', b'l', b't', b'h', b'z']);
        assert("openapi".spec_bytes() =~= seq![b'o', b'p', b'e', b'n', b'a', b'p', b'i']);
        assert("graphs".spec_bytes() =~= seq![b'g', b'r', b'a', b'p', b'h', b's']);
    }
    !bytes.is_empty() && bytes.len() <= 64
        && range_matches(bytes, 0, bytes.len(), Alphabet::Graph)
        && !same_bytes(bytes, "policies".as_bytes()) && !same_bytes(bytes, "healthz".as_bytes())
        && !same_bytes(bytes, "openapi".as_bytes()) && !same_bytes(bytes, "graphs".as_bytes())
}

fn ulid_part(bytes: &[u8], start: usize, end: usize) -> (accepted: bool)
    requires start <= end <= bytes.len(),
    ensures accepted == ulid_language(bytes@, start as int, end as int),
{
    end - start == 26 && byte_in_range(bytes[start], b'0', b'7')
        && range_matches(bytes, start, end, Alphabet::Crockford)
}

fn history_slot(bytes: &[u8], start: usize, end: usize) -> (accepted: bool)
    requires start <= end <= bytes.len(),
    ensures accepted == history_slot_language(bytes@, start as int, end as int),
{
    let length = end - start;
    if length == 0 || length > 5
        || !range_matches(bytes, start, end, Alphabet::Digit) {
        return false;
    }
    if length > 1 && bytes[start] == b'0' { return false; }
    if length < 5 { return true; }
    // Five canonical decimal digits are below 16384 exactly in this prefix range.
    bytes[start] == b'1' && (bytes[start + 1] < b'6'
        || (bytes[start + 1] == b'6' && (bytes[start + 2] < b'3'
            || (bytes[start + 2] == b'3' && (bytes[start + 3] < b'8'
                || (bytes[start + 3] == b'8' && bytes[start + 4] < b'4'))))))
}

pub(crate) fn graph_commit(value: &str) -> (accepted: bool)
    ensures accepted == graph_commit_identity(value.spec_bytes()),
{
    let bytes = value.as_bytes();
    let length = bytes.len();
    if ulid_part(bytes, 0, length) { return true; }
    if !(59..=63).contains(&length) { return false; }
    bytes[0] == b'h' && bytes[1] == b'b' && bytes[2] == b'1' && bytes[3] == b'.'
        && ulid_part(bytes, 4, 30) && bytes[30] == b'.'
        && history_slot(bytes, 31, length - 27) && bytes[length - 27] == b'.'
        && ulid_part(bytes, length - 26, length)
}

} // verus!
