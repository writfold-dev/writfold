//! Borrowed arithmetic preserves signed, unbounded mathematical values.
use super::Integer;
#[cfg(verus_keep_ghost)]
use super::primitive_specs::{borrowed_arithmetic_domain, from_primitive};
use std::ops::{Add, Sub};
use vstd::prelude::*;

verus! {

#[cfg(verus_keep_ghost)]
impl<'a, 'b> vstd::std_specs::ops::AddSpecImpl<&'b Integer> for &'a Integer {
    open spec fn obeys_add_spec() -> bool { true }
    open spec fn add_req(self, rhs: &'b Integer) -> bool { true }
    closed spec fn add_spec(self, rhs: &'b Integer) -> Integer {
        Integer(from_primitive(self@ + rhs@))
    }
}

#[cfg(verus_keep_ghost)]
impl<'a, 'b> vstd::std_specs::ops::SubSpecImpl<&'b Integer> for &'a Integer {
    open spec fn obeys_sub_spec() -> bool { true }
    open spec fn sub_req(self, rhs: &'b Integer) -> bool { true }
    closed spec fn sub_spec(self, rhs: &'b Integer) -> Integer {
        Integer(from_primitive(self@ - rhs@))
    }
}

impl Add<&Integer> for &Integer {
    type Output = Integer;
    #[inline]
    fn add(self, rhs: &Integer) -> (result: Integer)
        ensures result@ == self@ + rhs@,
    {
        proof { borrowed_arithmetic_domain(&self.0, &rhs.0); }
        Integer(<&num_bigint::BigInt as Add<&num_bigint::BigInt>>::add(&self.0, &rhs.0))
    }
}

impl Sub<&Integer> for &Integer {
    type Output = Integer;
    #[inline]
    fn sub(self, rhs: &Integer) -> (result: Integer)
        ensures result@ == self@ - rhs@,
    {
        proof { borrowed_arithmetic_domain(&self.0, &rhs.0); }
        Integer(<&num_bigint::BigInt as Sub<&num_bigint::BigInt>>::sub(&self.0, &rhs.0))
    }
}

} // verus!
