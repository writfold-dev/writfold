//! Stable dependency union without copying duplicate entries.
#[cfg(verus_keep_ghost)]
use super::footprint::{extend_concat, extend_push, extend_unique_spec, push_unique_spec};
use vstd::prelude::*;
use writfold_domain::*;

verus! {

fn push_unique(left: &mut Vec<Dependency>, value: &Dependency)
    ensures final(left).deep_view() == push_unique_spec(old(left).deep_view(), value@),
{
    let ghost before = left.deep_view();
    proof { assert(before.len() == left.len()); }
    let mut index = 0;
    while index < left.len()
        invariant left.deep_view() == before, before.len() == left.len(), index <= left.len(),
            forall|i: int| 0 <= i < index ==> before[i] != value@,
        decreases left.len() - index,
    {
        if left[index].eq(value) {
            proof { assert(before[index as int] == value@); }
            return;
        }
        index += 1;
    }
    left.push(value.clone());
}

fn extend(left: &mut Vec<Dependency>, right: &[Dependency])
    ensures final(left).deep_view() == extend_unique_spec(old(left).deep_view(), right.deep_view()),
{
    let ghost before = left.deep_view();
    let mut index = 0;
    proof { assert(right.deep_view().len() == right.len()); }
    while index < right.len()
        invariant index <= right.len(), right.deep_view().len() == right.len(),
            left.deep_view() == extend_unique_spec(before, right.deep_view().take(index as int)),
        decreases right.len() - index,
    {
        let value = &right[index];
        proof { assert(value@ == right.deep_view()[index as int]); }
        push_unique(left, value);
        proof {
            let prefix = right.deep_view().take(index as int);
            assert(right.deep_view().take(index as int + 1) =~= prefix.push(value@));
            extend_push(before, prefix, value@);
        }
        index += 1;
    }
    proof { assert(right.deep_view().take(right.len() as int) == right.deep_view()); }
}

pub fn dependencies_union(left: &[Dependency], right: &[Dependency]) -> (dependencies: Vec<Dependency>)
    ensures dependencies.deep_view() == extend_unique_spec(Seq::empty(), left.deep_view() + right.deep_view()),
{
    let mut result = Vec::new();
    proof { assert(result.deep_view() =~= Seq::<DependencyView>::empty()); }
    extend(&mut result, left);
    proof { assert(result.deep_view() == extend_unique_spec(Seq::empty(), left.deep_view())); }
    extend(&mut result, right);
    proof {
        assert(result.deep_view() == extend_unique_spec(
            extend_unique_spec(Seq::empty(), left.deep_view()), right.deep_view()));
        extend_concat(Seq::empty(), left.deep_view(), right.deep_view());
    }
    result
}

} // verus!
