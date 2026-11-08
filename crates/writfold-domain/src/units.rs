//! Explicit runtime bounds. No conversion reinterprets negative or oversized integers.
use crate::Integer;
use std::{error::Error, fmt};
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuantityError {
    kind: &'static str,
}
#[inline]
fn at_least(value: u64, minimum: u64) -> (accepted: bool)
    ensures accepted == (value >= minimum),
{
    value >= minimum
}
impl QuantityError {
    pub closed spec fn kind_value(self) -> &'static str { self.kind }
    pub fn kind(self) -> (kind: &'static str)
        ensures kind == self.kind_value(),
    {
        self.kind
    }
}
} // verus!
impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "integer outside {} runtime range", self.kind)
    }
}
impl Error for QuantityError {}

macro_rules! quantity {
    ($name:ident, $minimum:literal) => {
        verus! {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(u64);
        impl View for $name {
            type V = nat;
            closed spec fn view(&self) -> nat { self.0 as nat }
        }
        impl $name {
            pub closed spec fn from_raw_spec(value: u64) -> Result<Self, QuantityError> {
                if value >= $minimum { Ok(Self(value)) }
                else { Err(QuantityError { kind: stringify!($name) }) }
            }
            pub fn new(value: u64) -> (result: Result<Self, QuantityError>)
                ensures
                    result.is_ok() <==> value >= $minimum,
                    result.is_ok() ==> result.unwrap()@ == value as nat,
                    result == Self::from_raw_spec(value),
            {
                if at_least(value, $minimum) {
                    Ok(Self(value))
                } else {
                    Err(QuantityError {
                        kind: stringify!($name),
                    })
                }
            }
            pub fn get(self) -> (value: u64)
                ensures value as nat == self@,
            {
                self.0
            }
        }
        #[cfg(verus_keep_ghost)]
        impl vstd::std_specs::convert::TryFromSpecImpl<u64> for $name {
            open spec fn obeys_try_from_spec() -> bool { true }
            closed spec fn try_from_spec(value: u64) -> Result<Self, QuantityError> {
                Self::from_raw_spec(value)
            }
        }
        #[cfg(verus_keep_ghost)]
        impl vstd::std_specs::convert::TryFromSpecImpl<&Integer> for $name {
            open spec fn obeys_try_from_spec() -> bool { true }
            closed spec fn try_from_spec(value: &Integer) -> Result<Self, QuantityError> {
                if $minimum <= value@ <= u64::MAX { Ok(Self(value@ as u64)) }
                else { Err(QuantityError { kind: stringify!($name) }) }
            }
        }
        #[cfg(verus_keep_ghost)]
        impl vstd::std_specs::convert::FromSpecImpl<$name> for Integer {
            open spec fn obeys_from_spec() -> bool { true }
            closed spec fn from_spec(value: $name) -> Self {
                <Self as vstd::std_specs::convert::FromSpec<u64>>::from_spec(value.0)
            }
        }
        impl TryFrom<u64> for $name {
            type Error = QuantityError;
            fn try_from(value: u64) -> (result: Result<Self, Self::Error>)
                ensures
                    result.is_ok() <==> value >= $minimum,
                    result.is_ok() ==> result.unwrap()@ == value as nat,
            {
                Self::new(value)
            }
        }
        impl TryFrom<&Integer> for $name {
            type Error = QuantityError;
            fn try_from(value: &Integer) -> (result: Result<Self, Self::Error>)
                ensures
                    result.is_ok() <==> $minimum <= value@ <= u64::MAX,
                    result.is_ok() ==> result.unwrap()@ as int == value@,
            {
                match value.to_u64() {
                    Some(raw) => Self::new(raw),
                    None => Err(QuantityError { kind: stringify!($name) }),
                }
            }
        }
        impl From<$name> for Integer {
            fn from(value: $name) -> (result: Self)
                ensures result@ == value@ as int,
            {
                Integer::from(value.get())
            }
        }
        }
    };
}
quantity!(ByteCount, 0);
quantity!(ByteOffset, 0);
quantity!(RowLimit, 1);
quantity!(PageSize, 1);
quantity!(WitnessDepth, 0);
quantity!(EpochSeconds, 0);
quantity!(DurationSeconds, 0);
quantity!(NativeFormatVersion, 1);
// Structural counters do not establish current ownership or ledger validity.
quantity!(LedgerSequence, 1);
quantity!(OwnershipGeneration, 1);
// Procedure graph order, call rank and positive declared cost; no runtime budget.
quantity!(ProcedureNodeId, 1);
quantity!(ProcedureRank, 0);
quantity!(ExecutionCost, 1);

crate::records::scalar_views!(
    NativeFormatVersion,
    ByteCount,
    LedgerSequence,
    OwnershipGeneration,
    ProcedureNodeId,
    ProcedureRank,
    ExecutionCost
);
