//! Actual comparison/clone implementations over the admitted numeric primitives.
use super::Integer;
use std::cmp::Ordering;
use vstd::prelude::*;

verus! {

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Integer {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for Integer {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(if self@ < other@ { Ordering::Less }
             else if self@ > other@ { Ordering::Greater }
             else { Ordering::Equal })
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for Integer {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering {
        if self@ < other@ { Ordering::Less }
        else if self@ > other@ { Ordering::Greater }
        else { Ordering::Equal }
    }
}

impl Clone for Integer {
    #[inline]
    fn clone(&self) -> (result: Self)
        ensures result == *self,
    { Self(self.0.clone()) }
}

impl PartialEq for Integer {
    #[inline]
    fn eq(&self, other: &Self) -> (result: bool)
        ensures result == (self@ == other@),
    { self.0 == other.0 }
}

impl Eq for Integer {}

impl PartialOrd for Integer {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> (result: Option<Ordering>)
        ensures result == Some(if self@ < other@ { Ordering::Less }
            else if self@ > other@ { Ordering::Greater } else { Ordering::Equal }),
    { Some(self.cmp(other)) }
}

impl Ord for Integer {
    #[inline]
    fn cmp(&self, other: &Self) -> (result: Ordering)
        ensures
            (result is Less) <==> self@ < other@,
            (result is Equal) <==> self@ == other@,
            (result is Greater) <==> self@ > other@,
    { self.0.cmp(&other.0) }
}

} // verus!
