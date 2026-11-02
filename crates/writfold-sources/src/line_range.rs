use crate::{CodecError, unicode};
use std::cmp::Ordering;

/// Positive arbitrary-precision line number, stored as canonical ASCII decimal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineNumber {
    digits: String,
}

impl LineNumber {
    pub fn as_decimal(&self) -> &str {
        &self.digits
    }

    /// Explicit checked conversion for consumers with a bounded address space.
    pub fn to_usize(&self) -> Option<usize> {
        self.digits.parse().ok()
    }
}

impl Ord for LineNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        self.digits
            .len()
            .cmp(&other.digits.len())
            .then_with(|| self.digits.cmp(&other.digits))
    }
}

impl PartialOrd for LineNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange {
    first: LineNumber,
    last: LineNumber,
}

impl LineRange {
    pub fn first(&self) -> &LineNumber {
        &self.first
    }
    pub fn last(&self) -> &LineNumber {
        &self.last
    }
}

fn number(raw: &str) -> Option<LineNumber> {
    let mut digits = String::with_capacity(raw.len());
    for character in raw.chars() {
        let digit = unicode::decimal(character)?;
        if digit != 0 || !digits.is_empty() {
            digits.push(char::from(b'0' + digit));
        }
    }
    if digits.is_empty() {
        None
    } else {
        Some(LineNumber { digits })
    }
}

pub fn parse_range(value: &str) -> Result<LineRange, CodecError> {
    let invalid = || {
        CodecError::malformed(format!(
            "bad line range {value:?}, expected positive 'a-b' with a <= b"
        ))
    };
    let (first, last) = value.split_once('-').ok_or_else(invalid)?;
    let first = number(first).ok_or_else(invalid)?;
    let last = number(last).ok_or_else(invalid)?;
    if last < first {
        return Err(invalid());
    }
    Ok(LineRange { first, last })
}
