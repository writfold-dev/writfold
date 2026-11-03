//! Sorted borrowed identities index generation-stamped visitation, without resets.
use super::order::{Key, compare};
#[cfg(verus_keep_ghost)]
use super::{
    order::{KeyView, key_le, key_order},
    sort::sorted,
};
use std::cmp::Ordering;
use vstd::prelude::*;

verus! {
broadcast use key_order;
pub open spec fn visited(keys: Seq<KeyView>, stamps: Seq<usize>, generation: usize) -> Set<KeyView> {
    keys.to_set().filter(|key: KeyView| exists|i: int| 0 <= i < keys.len() && i < stamps.len() && stamps[i] == generation && keys[i] == key)
}
pub(super) fn find(keys: &[Key<'_>], key: Key<'_>) -> (found: Option<usize>)
    requires sorted(keys.deep_view()),
    ensures match found {
        Some(index) => index < keys.len() && keys.deep_view()[index as int] == key.deep_view(),
        None => !keys.deep_view().contains(key.deep_view()),
    },
{
    let mut low = 0;
    let mut high = keys.len();
    while low < high
        invariant low <= high <= keys.len(), sorted(keys.deep_view()),
            forall|i: int| 0 <= i < low ==> keys.deep_view()[i] != key.deep_view(),
            forall|i: int| high <= i < keys.len() ==> keys.deep_view()[i] != key.deep_view(),
        decreases high - low,
    {
        let mid = low + (high - low) / 2;
        match compare(keys[mid], key) {
            Ordering::Equal => return Some(mid),
            Ordering::Less => {
                proof {
                    assert forall|i: int| low <= i <= mid implies keys.deep_view()[i] != key.deep_view() by {
                        if i < mid && keys.deep_view()[i] == key.deep_view() {
                            assert(key_le(key.deep_view(), keys.deep_view()[mid as int]));
                        }
                    }
                }
                low = mid + 1;
            }
            Ordering::Greater => {
                proof {
                    assert forall|i: int| mid <= i < high implies keys.deep_view()[i] != key.deep_view() by {
                        if mid < i && keys.deep_view()[i] == key.deep_view() {
                            assert(key_le(keys.deep_view()[mid as int], key.deep_view()));
                        }
                    }
                }
                high = mid;
            }
        }
    }
    None
}
pub(super) fn empty_stamps(length: usize) -> (stamps: Vec<usize>)
    ensures stamps.len() == length, forall|i: int| 0 <= i < length ==> stamps@[i] == 0,
{
    let mut stamps = Vec::with_capacity(length);
    while stamps.len() < length
        invariant stamps.len() <= length, forall|i: int| 0 <= i < stamps.len() ==> stamps@[i] == 0,
        decreases length - stamps.len(),
    {
        stamps.push(0);
    }
    stamps
}
pub(super) proof fn mark_visited(keys: Seq<KeyView>, stamps: Seq<usize>, index: int, generation: usize)
    requires keys.len() == stamps.len(), keys.no_duplicates(), 0 <= index < keys.len(), stamps[index] != generation,
    ensures !visited(keys, stamps, generation).contains(keys[index]),
        visited(keys, stamps.update(index, generation), generation) == visited(keys, stamps, generation).insert(keys[index]),
{
    assert(visited(keys, stamps.update(index, generation), generation) =~= visited(keys, stamps, generation).insert(keys[index]));
}
}
