//! Explicit runtime bounds.
use std::{error::Error, fmt};

/// An integer outside a quantity's runtime range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuantityError {
    kind: &'static str,
}

impl QuantityError {
    pub fn kind(self) -> &'static str {
        self.kind
    }
}

impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "integer outside {} runtime range", self.kind)
    }
}

impl Error for QuantityError {}

/// A count of bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ByteCount(u64);

impl ByteCount {
    /// Every `u64` is a byte count; the `Result` keeps the checked shape
    /// callers use for the other quantities they bound.
    pub fn new(value: u64) -> Result<Self, QuantityError> {
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ByteCount {
    type Error = QuantityError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
