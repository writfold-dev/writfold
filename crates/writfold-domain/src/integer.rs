//! Signed, unbounded semantic integers, independent of OS time and size units.
mod arithmetic;
mod comparison;
#[cfg(verus_keep_ghost)]
mod primitive_specs;
use num_bigint::{BigInt, ParseBigIntError};
#[cfg(verus_keep_ghost)]
use primitive_specs::{from_primitive, mathematical};
use std::{fmt, str::FromStr};
use vstd::prelude::*;

verus! {

#[derive(Debug)]
pub struct Integer(BigInt);

impl View for Integer {
    type V = int;
    closed spec fn view(&self) -> int { mathematical(self.0) }
}

impl DeepView for Integer {
    type V = int;
    open spec fn deep_view(&self) -> int { self@ }
}

impl Integer {
    pub fn from_bigint(value: BigInt) -> (result: Self)
        ensures result@ == mathematical(value),
    {
        Self(value)
    }
    pub fn as_bigint(&self) -> (result: &BigInt)
        ensures mathematical(*result) == self@,
    {
        &self.0
    }
    pub fn into_bigint(self) -> (result: BigInt)
        ensures mathematical(result) == self@,
    {
        self.0
    }
    pub fn to_u64(&self) -> (result: Option<u64>)
        ensures
            result.is_some() <==> 0 <= self@ <= u64::MAX,
            result.is_some() ==> result.unwrap() as int == self@,
    {
        u64::try_from(&self.0).ok()
    }
    pub fn to_i64(&self) -> (result: Option<i64>)
        ensures
            result.is_some() <==> i64::MIN <= self@ <= i64::MAX,
            result.is_some() ==> result.unwrap() as int == self@,
    {
        i64::try_from(&self.0).ok()
    }
    pub fn is_negative(&self) -> (result: bool)
        ensures result == (self@ < 0),
    {
        matches!(self.0.sign(), num_bigint::Sign::Minus)
    }
}

} // verus!

impl std::hash::Hash for Integer {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0, state);
    }
}

impl FromStr for Integer {
    type Err = ParseBigIntError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! primitive_integers {
    ($($primitive:ty),+ $(,)?) => {$(
        verus! {
        #[cfg(verus_keep_ghost)]
        impl vstd::std_specs::convert::FromSpecImpl<$primitive> for Integer {
            open spec fn obeys_from_spec() -> bool { true }
            closed spec fn from_spec(value: $primitive) -> Self {
                Self(from_primitive(value as int))
            }
        }
        impl From<$primitive> for Integer {
            fn from(value: $primitive) -> (result: Self)
                ensures result@ == value as int,
            { Self(BigInt::from(value)) }
        }
        }
    )+};
}
primitive_integers!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);
