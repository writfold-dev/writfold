use crate::ByteOffset;
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinateUnit { Byte, Utf16 }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectorError { ReversedBounds }

/// Half-open offsets. Construction checks ordering, not object length or unit support.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selector {
    first: ByteOffset,
    last: ByteOffset,
    unit: CoordinateUnit,
}
impl View for Selector {
    type V = (nat, nat, CoordinateUnit);
    closed spec fn view(&self) -> Self::V { (self.first@, self.last@, self.unit) }
}
impl Selector {
    pub fn new(first: ByteOffset, last: ByteOffset, unit: CoordinateUnit)
        -> (result: Result<Self, SelectorError>)
        ensures result.is_ok() <==> first@ <= last@,
            result.is_ok() ==> result.unwrap()@ == (first@, last@, unit),
    {
        if first.get() > last.get() { Err(SelectorError::ReversedBounds) }
        else { Ok(Self { first, last, unit }) }
    }
    pub fn first(self) -> (value: ByteOffset)
        ensures value@ == self@.0,
    { self.first }
    pub fn last(self) -> (value: ByteOffset)
        ensures value@ == self@.1,
    { self.last }
    pub fn unit(self) -> (value: CoordinateUnit)
        ensures value == self@.2,
    { self.unit }
    pub fn same(self, other: Self) -> (equal: bool)
        ensures equal == (self@ == other@),
    {
        self.first.get() == other.first.get() && self.last.get() == other.last.get()
            && self.unit == other.unit
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchBudget { positions: usize, candidates: usize }
impl SearchBudget {
    pub closed spec fn position_limit(self) -> nat { self.positions as nat }
    pub closed spec fn candidate_limit(self) -> nat { self.candidates as nat }
    pub fn new(positions: usize, candidates: usize) -> (value: Self)
        ensures value.position_limit() == positions, value.candidate_limit() == candidates,
    { Self { positions, candidates } }
    pub fn positions(self) -> (value: usize)
        ensures value == self.position_limit(),
    { self.positions }
    pub fn candidates(self) -> (value: usize)
        ensures value == self.candidate_limit(),
    { self.candidates }
}

/// Coverage is measured by scanning, never asserted by an untrusted completeness bit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coverage { Unavailable, Measured { examined: usize, total: u128 } }
impl Coverage {
    pub fn complete(self) -> (complete: bool)
        ensures complete == match self {
            Self::Unavailable => false,
            Self::Measured { examined, total } => examined as u128 == total,
        },
    {
        match self {
            Self::Unavailable => false,
            Self::Measured { examined, total } => examined as u128 == total,
        }
    }
}
}
crate::records::scalar_views!(CoordinateUnit, Coverage);
