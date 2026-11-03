//! Internal deterministic ordering; only textual equality affects decisions.
use std::cmp::Ordering;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::utf8::{encode_utf8, encode_utf8_decode_utf8};

pub(super) type Key<'a> = (&'a str, &'a str);

verus! {
#[cfg(verus_keep_ghost)]
pub type KeyView = (Seq<char>, Seq<char>);
pub open spec fn bytes_le(left: Seq<u8>, right: Seq<u8>) -> bool
    decreases left.len() + right.len(),
{
    left.len() == 0 || (right.len() > 0 && (left[0] < right[0]
        || (left[0] == right[0] && bytes_le(left.skip(1), right.skip(1)))))
}
pub open spec fn text_le(left: Seq<char>, right: Seq<char>) -> bool {
    bytes_le(encode_utf8(left), encode_utf8(right))
}
pub open spec fn key_le(left: KeyView, right: KeyView) -> bool {
    text_le(left.0, right.0) && (left.0 != right.0 || text_le(left.1, right.1))
}
proof fn bytes_order(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
    ensures bytes_le(a, a), bytes_le(a, b) || bytes_le(b, a),
        bytes_le(a, b) && bytes_le(b, a) ==> a == b,
        bytes_le(a, b) && bytes_le(b, c) ==> bytes_le(a, c),
    decreases a.len() + b.len() + c.len(),
{
    if a.len() > 0 && b.len() > 0 && c.len() > 0 {
        bytes_order(a.skip(1), b.skip(1), c.skip(1));
    } else if a.len() > 0 && b.len() > 0 {
        bytes_order(a.skip(1), b.skip(1), c);
    } else if a.len() > 0 {
        bytes_order(a.skip(1), b, c);
    }
    if bytes_le(a, b) && bytes_le(b, a) {
        if a.len() > 0 {
            assert(b.len() > 0);
            assert(a[0] == b[0]);
            assert(bytes_le(a.skip(1), b.skip(1)));
            assert(bytes_le(b.skip(1), a.skip(1)));
            assert(a.skip(1) == b.skip(1));
            assert(a.len() == b.len());
            assert forall|i: int| 0 <= i < a.len() implies #[trigger] a[i] == b[i] by {
                if i > 0 { assert(a.skip(1)[i - 1] == b.skip(1)[i - 1]); }
            }
        }
        assert(a =~= b);
    }
}
proof fn key_order_laws(a: KeyView, b: KeyView, c: KeyView)
    ensures key_le(a, a), key_le(a, b) || key_le(b, a),
        key_le(a, b) && key_le(b, a) ==> a == b,
        key_le(a, b) && key_le(b, c) ==> key_le(a, c),
{
    bytes_order(encode_utf8(a.0), encode_utf8(b.0), encode_utf8(c.0));
    bytes_order(encode_utf8(a.1), encode_utf8(b.1), encode_utf8(c.1));
    encode_utf8_decode_utf8(a.0);
    encode_utf8_decode_utf8(b.0);
    encode_utf8_decode_utf8(c.0);
    encode_utf8_decode_utf8(a.1);
    encode_utf8_decode_utf8(b.1);
    encode_utf8_decode_utf8(c.1);
}
pub broadcast proof fn key_reflexive(a: KeyView)
    ensures #[trigger] key_le(a, a),
{ key_order_laws(a, a, a); }
pub broadcast proof fn key_total(a: KeyView, b: KeyView)
    ensures #[trigger] key_le(a, b) || #[trigger] key_le(b, a),
{ key_order_laws(a, b, b); }
pub broadcast proof fn key_antisymmetric(a: KeyView, b: KeyView)
    requires #[trigger] key_le(a, b), #[trigger] key_le(b, a),
    ensures a == b,
{ key_order_laws(a, b, b); }
pub broadcast proof fn key_transitive(a: KeyView, b: KeyView, c: KeyView)
    requires #[trigger] key_le(a, b), #[trigger] key_le(b, c),
    ensures key_le(a, c),
{ key_order_laws(a, b, c); }
#[cfg(verus_keep_ghost)]
pub broadcast group key_order { key_reflexive, key_total, key_antisymmetric, key_transitive }
fn compare_text(left: &str, right: &str) -> (order: Ordering)
    ensures (order != Ordering::Greater) == text_le(left@, right@),
        (order != Ordering::Less) == text_le(right@, left@),
        (order == Ordering::Equal) == (left@ == right@),
{
    let a = left.as_bytes();
    let b = right.as_bytes();
    proof {
        assert(a@.skip(0) =~= a@);
        assert(b@.skip(0) =~= b@);
    }
    let mut i = 0;
    while i < a.len() && i < b.len()
        invariant i <= a.len(), i <= b.len(),
            a@ == encode_utf8(left@), b@ == encode_utf8(right@),
            bytes_le(a@, b@) == bytes_le(a@.skip(i as int), b@.skip(i as int)),
            bytes_le(b@, a@) == bytes_le(b@.skip(i as int), a@.skip(i as int)),
            forall|j: int| 0 <= j < i ==> #[trigger] a@[j] == b@[j],
        decreases a.len() - i,
    {
        if a[i] < b[i] { return Ordering::Less; }
        if a[i] > b[i] { return Ordering::Greater; }
        proof {
            assert(a@.skip(i as int).skip(1) =~= a@.skip(i as int + 1));
            assert(b@.skip(i as int).skip(1) =~= b@.skip(i as int + 1));
        }
        i += 1;
    }
    if a.len() < b.len() { return Ordering::Less; }
    if a.len() > b.len() { return Ordering::Greater; }
    proof {
        assert(a@ =~= b@);
        encode_utf8_decode_utf8(left@);
        encode_utf8_decode_utf8(right@);
    }
    Ordering::Equal
}
pub(super) fn compare(left: Key<'_>, right: Key<'_>) -> (order: Ordering)
    ensures (order != Ordering::Greater) == key_le(left.deep_view(), right.deep_view()),
        (order != Ordering::Less) == key_le(right.deep_view(), left.deep_view()),
        (order == Ordering::Equal) == (left.deep_view() == right.deep_view()),
{
    match compare_text(left.0, right.0) {
        Ordering::Equal => compare_text(left.1, right.1),
        order => order,
    }
}
}
