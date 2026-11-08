//! Restricted deterministic CBOR bytes, not schema validity or authenticity.
mod encode;
mod integer;
mod parse;
pub(crate) use encode::argument_header;

use std::borrow::Cow;
use writfold_domain::Integer;

use crate::{Budget, Limits};

/// Borrowed values: arrays retain order and map keys are exact UTF-8 text.
#[derive(Debug)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Unsigned(u64),
    Signed(i64),
    Integer(&'a Integer),
    Bytes(&'a [u8]),
    Text(&'a str),
    Array(&'a [Value<'a>]),
    Map(&'a [(&'a str, Value<'a>)]),
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    #[error("{budget:?} budget exceeded at byte {offset}")]
    Limit { budget: Budget, offset: usize },
    #[error("invalid CBOR at byte {offset}: {reason}")]
    Syntax { offset: usize, reason: &'static str },
    #[error("noncanonical CBOR at byte {offset}: {reason}")]
    NonCanonical { offset: usize, reason: &'static str },
    #[error("invalid UTF-8 at byte {offset}")]
    Unicode { offset: usize },
    #[error("duplicate CBOR map key at byte {offset}")]
    DuplicateKey { offset: usize },
    #[error("trailing CBOR input at byte {offset}")]
    TrailingInput { offset: usize },
}

/// Privately validated canonical bytes. Validation does not authenticate a sender
/// or establish a payload schema; callers may hash these exact bytes afterward.
#[derive(Debug)]
pub struct RawCbor<'a> {
    bytes: Cow<'a, [u8]>,
}

impl<'a> RawCbor<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        parse::validate(bytes, limits)?;
        Ok(Self {
            bytes: Cow::Borrowed(bytes),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes.into_owned()
    }
}

/// Encode without recursive traversal or a redundant validation pass.
pub fn encode(value: &Value<'_>, limits: Limits) -> Result<RawCbor<'static>, Error> {
    encode::encode(value, limits).map(|bytes| RawCbor {
        bytes: Cow::Owned(bytes),
    })
}
