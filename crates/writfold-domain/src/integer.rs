//! Signed, unbounded semantic integers, independent of OS time and size units.
use num_bigint::{BigInt, ParseBigIntError};
use num_traits::ToPrimitive;
use std::{
    fmt,
    ops::{Add, Sub},
    str::FromStr,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Integer(BigInt);

impl Integer {
    pub fn from_bigint(value: BigInt) -> Self {
        Self(value)
    }
    pub fn as_bigint(&self) -> &BigInt {
        &self.0
    }
    pub fn into_bigint(self) -> BigInt {
        self.0
    }
    pub fn to_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }
    pub fn to_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }
    pub fn is_negative(&self) -> bool {
        self.0.sign() == num_bigint::Sign::Minus
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
        impl From<$primitive> for Integer {
            fn from(value: $primitive) -> Self { Self(BigInt::from(value)) }
        }
    )+};
}
primitive_integers!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

impl Add<&Integer> for &Integer {
    type Output = Integer;
    fn add(self, rhs: &Integer) -> Integer {
        Integer(&self.0 + &rhs.0)
    }
}
impl Sub<&Integer> for &Integer {
    type Output = Integer;
    fn sub(self, rhs: &Integer) -> Integer {
        Integer(&self.0 - &rhs.0)
    }
}
