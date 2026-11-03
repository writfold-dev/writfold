//! In-place heapsort over borrowed keys; no sorting scratch allocation.
use std::cmp::Ordering;
use vstd::prelude::*;
use super::order::{compare, Key};
#[cfg(verus_keep_ghost)]
use super::order::{key_le, key_order, KeyView};

verus! {
broadcast use {key_order, vstd::seq_lib::group_seq_properties};
proof fn swap_counts(values: Seq<KeyView>, i: int, j: int)
    requires 0 <= i < values.len(), 0 <= j < values.len(),
    ensures swapped(values, i, j).to_multiset() == values.to_multiset(),
{
    broadcast use vstd::multiset::group_multiset_axioms;
    values.lemma_index_contains(i);
    values.lemma_index_contains(j);
    if i == j || values[i] == values[j] {
        assert(swapped(values, i, j) =~= values);
    } else {
        let first = values.update(i, values[j]);
        vstd::seq_lib::to_multiset_update(values, i, values[j]);
        vstd::seq_lib::to_multiset_update(first, j, values[i]);
        assert(first[j] == values[j]);
        assert forall|value: KeyView| swapped(values, i, j).to_multiset().count(value)
            == values.to_multiset().count(value) by {
            if value == values[i] {
                assert(values.to_multiset().count(value) > 0);
            } else if value == values[j] {
                assert(values.to_multiset().count(value) > 0);
            }
        }
        assert(swapped(values, i, j).to_multiset() =~= values.to_multiset());
    }
}
pub open spec fn sorted(values: Seq<KeyView>) -> bool {
    forall|i: int, j: int| 0 <= i < j < values.len() ==> key_le(values[i], values[j])
}
spec fn parent(child: int) -> int { if child > 0 { (child - 1) / 2 } else { -1 } }
spec fn heap_from(values: Seq<KeyView>, end: int, start: int) -> bool {
    forall|i: int| 0 < i < end && parent(i) >= start ==> key_le(values[i], values[parent(i)])
}
spec fn heap_except(values: Seq<KeyView>, end: int, root: int, hole: int) -> bool {
    forall|i: int| 0 < i < end && parent(i) >= root && parent(i) != hole ==> key_le(values[i], values[parent(i)])
}
spec fn above_hole(values: Seq<KeyView>, end: int, root: int, hole: int) -> bool {
    parent(hole) >= root ==> forall|i: int| 0 < i < end && parent(i) == hole ==> key_le(values[i], values[parent(hole)])
}
spec fn swapped(values: Seq<KeyView>, i: int, j: int) -> Seq<KeyView> {
    values.update(i, values[j]).update(j, values[i])
}
proof fn swap_permutation(values: Seq<KeyView>, i: int, j: int, start: int, end: int)
    requires 0 <= start <= i < end <= values.len(), start <= j < end,
    ensures swapped(values, i, j).to_multiset() == values.to_multiset(),
        swapped(values, i, j).subrange(start, end).to_multiset() == values.subrange(start, end).to_multiset(),
{
    swap_counts(values, i, j);
    swap_counts(values.subrange(start, end), i - start, j - start);
    assert(swapped(values, i, j).subrange(start, end)
        =~= swapped(values.subrange(start, end), i - start, j - start));
}
fn swap(values: &mut Vec<Key<'_>>, i: usize, j: usize)
    requires i < old(values).len(), j < old(values).len(),
    ensures final(values).deep_view() == swapped(old(values).deep_view(), i as int, j as int),
        final(values).len() == old(values).len(),
{
    let left = values[i];
    let right = values[j];
    values.set(i, right);
    values.set(j, left);
}
proof fn sift_step(values: Seq<KeyView>, end: int, root: int, hole: int, child: int)
    requires 0 <= root <= hole < end <= values.len(), hole < child < end, parent(child) == hole,
        heap_except(values, end, root, hole), above_hole(values, end, root, hole),
        key_le(values[hole], values[child]),
        forall|i: int| 0 < i < end && parent(i) == hole ==> key_le(values[i], values[child]),
    ensures heap_except(swapped(values, hole, child), end, root, child),
        above_hole(swapped(values, hole, child), end, root, child),
{
    let after = swapped(values, hole, child);
    assert forall|i: int| 0 < i < end && parent(i) >= root && parent(i) != child
        implies key_le(after[i], after[parent(i)]) by {
        if i == hole {
            assert(key_le(values[child], values[parent(hole)]));
        } else if i == child {
            assert(key_le(values[hole], values[child]));
        } else if parent(i) == hole {
            assert(key_le(values[i], values[child]));
        } else {
            assert(after[i] == values[i]);
            assert(after[parent(i)] == values[parent(i)]);
        }
    }
    assert forall|i: int| 0 < i < end && parent(i) == child
        implies key_le(after[i], after[parent(child)]) by {
        assert(key_le(values[i], values[child]));
    }
}
fn sift_down(values: &mut Vec<Key<'_>>, end: usize, root: usize)
    requires root < end <= old(values).len(), heap_from(old(values).deep_view(), end as int, root + 1),
    ensures heap_from(final(values).deep_view(), end as int, root as int),
        final(values).len() == old(values).len(),
        final(values).deep_view().to_multiset() == old(values).deep_view().to_multiset(),
        final(values).deep_view().subrange(root as int, end as int).to_multiset()
            == old(values).deep_view().subrange(root as int, end as int).to_multiset(),
        forall|i: int| 0 <= i < old(values).len() && !(root <= i < end) ==> final(values).deep_view()[i] == old(values).deep_view()[i],
{
    let mut hole = root;
    while hole < end / 2
        invariant root <= hole < end <= values.len(), values.len() == old(values).len(),
            heap_except(values.deep_view(), end as int, root as int, hole as int),
            above_hole(values.deep_view(), end as int, root as int, hole as int),
            values.deep_view().to_multiset() == old(values).deep_view().to_multiset(),
            values.deep_view().subrange(root as int, end as int).to_multiset()
                == old(values).deep_view().subrange(root as int, end as int).to_multiset(),
            forall|i: int| 0 <= i < values.len() && !(root <= i < end) ==> values.deep_view()[i] == old(values).deep_view()[i],
        decreases end - hole,
    {
        let left = 2 * hole + 1;
        let right = left + 1;
        let mut child = left;
        if right < end {
            proof {
                assert(values[left as int].deep_view() == values.deep_view()[left as int]);
                assert(values[right as int].deep_view() == values.deep_view()[right as int]);
            }
            if compare(values[left], values[right]) == Ordering::Less { child = right; }
        }
        proof {
            assert(key_le(values.deep_view()[left as int], values.deep_view()[child as int]));
            if right < end {
                assert(key_le(values.deep_view()[right as int], values.deep_view()[child as int]));
            }
            assert forall|i: int| 0 < i < end && parent(i) == hole
                implies key_le(values.deep_view()[i], values.deep_view()[child as int]) by {
                assert(i == 2 * hole + 1 || i == 2 * hole + 2);
            }
        }
        proof {
            assert(values[hole as int].deep_view() == values.deep_view()[hole as int]);
            assert(values[child as int].deep_view() == values.deep_view()[child as int]);
        }
        if compare(values[hole], values[child]) != Ordering::Less {
            proof {
                assert forall|i: int| 0 < i < end && parent(i) >= root
                    implies key_le(values.deep_view()[i], values.deep_view()[parent(i)]) by {
                    if parent(i) == hole {
                        assert(key_le(values.deep_view()[i], values.deep_view()[child as int]));
                        assert(key_le(values.deep_view()[child as int], values.deep_view()[hole as int]));
                    }
                }
            }
            return;
        }
        let ghost before = values.deep_view();
        swap(values, hole, child);
        proof {
            sift_step(before, end as int, root as int, hole as int, child as int);
            swap_permutation(before, hole as int, child as int, root as int, end as int);
        }
        hole = child;
    }
}
proof fn heap_maximum(values: Seq<KeyView>, end: int, i: int)
    requires 0 <= i < end <= values.len(), heap_from(values, end, 0),
    ensures key_le(values[i], values[0]),
    decreases i,
{
    if i > 0 {
        heap_maximum(values, end, parent(i));
    }
}
fn build_heap(values: &mut Vec<Key<'_>>)
    ensures final(values).len() == old(values).len(), heap_from(final(values).deep_view(), final(values).len() as int, 0),
        final(values).deep_view().to_multiset() == old(values).deep_view().to_multiset(),
{
    let end = values.len();
    let mut root = end / 2;
    while root > 0
        invariant root <= end / 2, values.len() == end, end == old(values).len(),
            heap_from(values.deep_view(), end as int, root as int),
            values.deep_view().to_multiset() == old(values).deep_view().to_multiset(),
        decreases root,
    {
        root -= 1;
        sift_down(values, end, root);
    }
}
proof fn prefix_bounds(before: Seq<KeyView>, after: Seq<KeyView>, end: int)
    requires 0 <= end <= before.len(), before.len() == after.len(),
        before.take(end).to_multiset() == after.take(end).to_multiset(),
        forall|i: int| end <= i < before.len() ==> before[i] == after[i],
        forall|i: int, j: int| 0 <= i < end <= j < before.len() ==> key_le(before[i], before[j]),
    ensures forall|i: int, j: int| 0 <= i < end <= j < after.len() ==> key_le(after[i], after[j]),
{
    assert forall|i: int, j: int| 0 <= i < end <= j < after.len() implies key_le(after[i], after[j]) by {
        assert(after.take(end)[i] == after[i]);
        after.take(end).lemma_index_contains(i);
        assert(after.take(end).contains(after[i]));
        assert(before.take(end).contains(after[i]));
        let at = choose|at: int| 0 <= at < end && before[at] == after[i];
        assert(key_le(before[at], before[j]));
    }
}
proof fn extract_maximum(before: Seq<KeyView>, end: int)
    requires 1 < end <= before.len(), heap_from(before, end, 0), sorted(before.skip(end)),
        forall|i: int, j: int| 0 <= i < end <= j < before.len() ==> key_le(before[i], before[j]),
    ensures heap_from(swapped(before, 0, end - 1), end - 1, 1),
        sorted(swapped(before, 0, end - 1).skip(end - 1)),
        forall|i: int, j: int| 0 <= i < end - 1 <= j < before.len() ==>
            key_le(swapped(before, 0, end - 1)[i], swapped(before, 0, end - 1)[j]),
{
    let after = swapped(before, 0, end - 1);
    assert forall|i: int| 0 < i < end - 1 && parent(i) >= 1 implies key_le(after[i], after[parent(i)]) by {
        assert(after[i] == before[i]);
        assert(after[parent(i)] == before[parent(i)]);
    }
    assert forall|i: int, j: int| 0 <= i < j < after.skip(end - 1).len()
        implies key_le(after.skip(end - 1)[i], after.skip(end - 1)[j]) by {
        if i == 0 {
            assert(key_le(before[0], before[j + end - 1]));
        } else {
            assert(key_le(before.skip(end)[i - 1], before.skip(end)[j - 1]));
        }
    }
    assert forall|i: int, j: int| 0 <= i < end - 1 <= j < before.len()
        implies key_le(after[i], after[j]) by {
        if j == end - 1 {
            if i == 0 { heap_maximum(before, end, end - 1); }
            else { heap_maximum(before, end, i); }
        } else if i == 0 {
            assert(key_le(before[end - 1], before[j]));
        } else {
            assert(key_le(before[i], before[j]));
        }
    }
}
pub(super) fn sort(values: &mut Vec<Key<'_>>)
    ensures final(values).len() == old(values).len(), sorted(final(values).deep_view()),
        final(values).deep_view().to_multiset() == old(values).deep_view().to_multiset(),
{
    build_heap(values);
    let mut end = values.len();
    while end > 1
        invariant end <= values.len(), values.len() == old(values).len(),
            heap_from(values.deep_view(), end as int, 0), sorted(values.deep_view().skip(end as int)),
            forall|i: int, j: int| 0 <= i < end <= j < values.len() ==> key_le(values.deep_view()[i], values.deep_view()[j]),
            values.deep_view().to_multiset() == old(values).deep_view().to_multiset(),
        decreases end,
    {
        let ghost before = values.deep_view();
        end -= 1;
        swap(values, 0, end);
        proof {
            swap_permutation(before, 0, end as int, 0, before.len() as int);
            extract_maximum(before, end as int + 1);
        }
        let ghost before_sift = values.deep_view();
        sift_down(values, end, 0);
        proof {
            assert(values.deep_view().skip(end as int) =~= before_sift.skip(end as int));
            prefix_bounds(before_sift, values.deep_view(), end as int);
        }
    }
    proof {
        assert forall|i: int, j: int| 0 <= i < j < values.len()
            implies key_le(values.deep_view()[i], values.deep_view()[j]) by {
            if i >= end {
                assert(key_le(values.deep_view().skip(end as int)[i - end],
                    values.deep_view().skip(end as int)[j - end]));
            }
        }
    }
}
pub(super) proof fn permutation_unique(left: Seq<KeyView>, right: Seq<KeyView>)
    requires left.to_multiset() == right.to_multiset(),
    ensures left.no_duplicates() == right.no_duplicates(),
{
    if left.no_duplicates() {
        left.lemma_multiset_has_no_duplicates();
        right.lemma_multiset_has_no_duplicates_conv();
    }
    if right.no_duplicates() {
        right.lemma_multiset_has_no_duplicates();
        left.lemma_multiset_has_no_duplicates_conv();
    }
}
pub(super) proof fn permutation_membership(left: Seq<KeyView>, right: Seq<KeyView>)
    requires left.to_multiset() == right.to_multiset(),
    ensures left.to_set() == right.to_set(),
{
    assert forall|value: KeyView| left.to_set().contains(value) == right.to_set().contains(value) by {
        assert(left.contains(value) == (left.to_multiset().count(value) > 0));
        assert(right.contains(value) == (right.to_multiset().count(value) > 0));
    }
    assert(left.to_set() =~= right.to_set());
}
pub(super) fn adjacent_unique(values: &Vec<Key<'_>>) -> (unique: bool)
    requires sorted(values.deep_view()),
    ensures unique == values.deep_view().no_duplicates(),
{
    let mut i = 1;
    while i < values.len()
        invariant 1 <= i <= values.len() + 1, sorted(values.deep_view()),
            forall|p: int, q: int| 0 <= p < q < i && q < values.len() ==> values.deep_view()[p] != values.deep_view()[q],
        decreases values.len() + 1 - i,
    {
        proof {
            assert(values[i as int - 1].deep_view() == values.deep_view()[i as int - 1]);
            assert(values[i as int].deep_view() == values.deep_view()[i as int]);
        }
        if compare(values[i - 1], values[i]) == Ordering::Equal {
            proof {
                assert(values.deep_view()[i as int - 1] == values.deep_view()[i as int]);
                assert(!values.deep_view().no_duplicates());
            }
            return false;
        }
        proof {
            assert forall|p: int| 0 <= p < i implies values.deep_view()[p] != values.deep_view()[i as int] by {
                if p < i - 1 && values.deep_view()[p] == values.deep_view()[i as int] {
                    assert(key_le(values.deep_view()[i as int - 1], values.deep_view()[i as int]));
                    assert(key_le(values.deep_view()[i as int], values.deep_view()[i as int - 1]));
                    assert(values.deep_view()[i as int - 1] == values.deep_view()[i as int]);
                }
            }
        }
        i += 1;
    }
    true
}
pub(super) fn unique(values: Vec<Key<'_>>) -> (unique: bool)
    ensures unique == values.deep_view().no_duplicates(),
{
    let mut keys = values;
    let ghost original = keys.deep_view();
    sort(&mut keys);
    proof { permutation_unique(original, keys.deep_view()); }
    adjacent_unique(&keys)
}
}
