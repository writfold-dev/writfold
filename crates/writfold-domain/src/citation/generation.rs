//! Alias-table epochs are independent of protocol request correlation.
//!
//! A transport request advancing does not rotate a citation table. The host
//! advances this epoch only when replacing/retiring its scoped handle mapping.
//! This value labels an epoch; it confers neither authority nor live ownership.
//!
//! ```compile_fail
//! use writfold_domain::{CitationTableGeneration, RequestGeneration};
//! fn restore_table(_: CitationTableGeneration) {}
//! restore_table(RequestGeneration::from_u64(7));
//! ```
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CitationTableGeneration {
    value: u64,
}

impl CitationTableGeneration {
    pub closed spec fn value(self) -> u64 { self.value }

    pub fn from_u64(value: u64) -> (result: Self)
        ensures result.value() == value,
    { Self { value } }

    pub fn get(self) -> (value: u64)
        ensures value == self.value(),
    { self.value }

    pub fn checked_next(self) -> (result: Option<Self>)
        ensures
            result.is_none() <==> self.value() == u64::MAX,
            result.is_some() ==> result.unwrap().value() == self.value() + 1,
    {
        if self.value == u64::MAX {
            None
        } else {
            Some(Self { value: self.value + 1 })
        }
    }
}
}
