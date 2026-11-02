//! Explicit runtime bounds. No conversion reinterprets negative or oversized integers.
use crate::Integer;
use std::{error::Error, fmt};

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

macro_rules! quantity {
    ($name:ident, $minimum:literal) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(u64);
        impl $name {
            pub fn new(value: u64) -> Result<Self, QuantityError> {
                if ($minimum..=u64::MAX).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(QuantityError {
                        kind: stringify!($name),
                    })
                }
            }
            pub fn get(self) -> u64 {
                self.0
            }
        }
        impl TryFrom<u64> for $name {
            type Error = QuantityError;
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
        impl TryFrom<&Integer> for $name {
            type Error = QuantityError;
            fn try_from(value: &Integer) -> Result<Self, Self::Error> {
                let raw = value.to_u64().ok_or(QuantityError {
                    kind: stringify!($name),
                })?;
                Self::new(raw)
            }
        }
        impl From<$name> for Integer {
            fn from(value: $name) -> Self {
                Integer::from(value.get())
            }
        }
    };
}
quantity!(ByteCount, 0);
quantity!(RowLimit, 1);
quantity!(PageSize, 1);
quantity!(WitnessDepth, 0);
quantity!(EpochSeconds, 0);
quantity!(DurationSeconds, 0);
