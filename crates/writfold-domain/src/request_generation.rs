//! Production generation arithmetic, verified directly rather than through a twin.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RequestGeneration {
    value: u64,
}

impl RequestGeneration {
    pub closed spec fn value(self) -> u64 {
        self.value
    }

    pub fn first() -> (result: Self)
        ensures result.value() == 0,
    {
        Self { value: 0 }
    }

    pub fn get(self) -> (result: u64)
        ensures result == self.value(),
    {
        self.value
    }

    pub fn from_u64(value: u64) -> (result: Self)
        ensures result.value() == value,
    {
        Self { value }
    }

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

#[cfg(test)]
mod tests {
    use super::RequestGeneration;

    #[test]
    fn generation_starts_at_zero_and_refuses_exhaustion() {
        assert_eq!(RequestGeneration::first().get(), 0);
        assert_eq!(
            RequestGeneration::first()
                .checked_next()
                .map(RequestGeneration::get),
            Some(1)
        );
        let penultimate = RequestGeneration {
            value: u64::MAX - 1,
        };
        assert_eq!(
            penultimate.checked_next().map(RequestGeneration::get),
            Some(u64::MAX)
        );
        assert_eq!(RequestGeneration { value: u64::MAX }.checked_next(), None);
    }
}
