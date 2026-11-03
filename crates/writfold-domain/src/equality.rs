//! Content equality bridges for the pinned standard-library specifications.
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;

verus! {

/// Lift an element equality model to the library's vector equality contract.
pub broadcast proof fn vector_equality<T: DeepView + PartialEq>(left: &Vec<T>, right: &Vec<T>)
    requires
        forall|a: T, b: T| #[trigger] a.eq_spec(&b) == (a.deep_view() == b.deep_view()),
    ensures
        #[trigger] left.eq_spec(right) == (left.deep_view() == right.deep_view()),
{
    broadcast use vstd::std_specs::vec::group_vec_axioms;
    assert(left.deep_view().len() == left.len());
    assert(right.deep_view().len() == right.len());
    if left.eq_spec(right) {
        assert(left.deep_view() =~= right.deep_view());
    } else if left.deep_view() == right.deep_view() {
        assert(left.len() == right.len());
        assert forall|index: int| 0 <= index < left.len() implies
            left[index].eq_spec(&right[index]) by {
            assert(left[index].deep_view() == left.deep_view()[index]);
            assert(right[index].deep_view() == right.deep_view()[index]);
        }
        assert(left.eq_spec(right));
    }
}

/// String has a concrete equality contract but no foreign-trait SpecImpl in
/// the pinned vstd. Verify this allocation-free loop instead of assuming one.
pub fn strings_equal(left: &[String], right: &[String]) -> (equal: bool)
    ensures equal == (left.deep_view() == right.deep_view()),
{
    proof {
        assert(left.deep_view().len() == left.len());
        assert(right.deep_view().len() == right.len());
    }
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len()
        invariant
            left.len() == right.len(),
            index <= left.len(),
            forall|i: int| 0 <= i < index ==> left@[i].deep_view() == right@[i].deep_view(),
        decreases left.len() - index,
    {
        if !left[index].as_str().eq(right[index].as_str()) {
            proof {
                assert(left[index as int]@ != right[index as int]@);
                assert(left.deep_view()[index as int] != right.deep_view()[index as int]);
            }
            return false;
        }
        index += 1;
    }
    proof { assert(left.deep_view() =~= right.deep_view()); }
    true
}

} // verus!
