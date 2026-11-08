use std::borrow::Cow;

use super::{Error, IntegerRef, Kind, RawCbor, Token, TokenValue, parse::Walker};
use crate::Limits;

pub type DecodeResult<T, E = DecodeError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum DecodeError {
    #[error(transparent)]
    Cbor(#[from] Error),
    #[error("expected {expected}, found {actual:?} at byte {offset}")]
    Type {
        offset: usize,
        expected: &'static str,
        actual: Kind,
    },
    #[error("expected {expected} at end of input (byte {offset})")]
    EndOfInput {
        offset: usize,
        expected: &'static str,
    },
    #[error("unconsumed CBOR token at byte {offset}")]
    Unconsumed { offset: usize },
    #[error("CBOR decoder cannot continue after an error")]
    Failed,
}

/// Single-pass, borrowed decoding of the same restricted profile as `RawCbor`.
/// Only the bounded container stack is allocated; payloads borrow the input.
/// Validation is incremental: tokens do not imply validity of unread bytes.
/// Any error makes this cursor unusable, including a schema-level type error.
///
/// Closed schema readers should require exact opening array/map lengths, read
/// every field (and exact map key) with typed methods, require closing tokens,
/// then call `finish`. Only its returned `RawCbor` admits the complete bytes for
/// fingerprinting; neither decoding nor fingerprinting grants authority.
/// Container depth, entry, byte and node budgets match `RawCbor::parse` exactly.
pub struct Decoder<'a> {
    bytes: &'a [u8],
    walker: Walker<'a>,
    failed: bool,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8], limits: Limits) -> DecodeResult<Self> {
        Ok(Self {
            bytes,
            walker: Walker::new(bytes, limits)?,
            failed: false,
        })
    }

    /// Read one token, or `None` only after a complete, canonical root value.
    /// Closing tokens must be consumed even when their container is empty.
    pub fn next_token(&mut self) -> DecodeResult<Option<Token<'a>>> {
        if self.failed {
            return Err(DecodeError::Failed);
        }
        self.walker.next_token().map_err(|error| {
            self.failed = true;
            DecodeError::Cbor(error)
        })
    }

    fn read<T>(
        &mut self,
        expected: &'static str,
        convert: impl FnOnce(Token<'a>) -> Option<T>,
    ) -> DecodeResult<T> {
        let token = self.next_token()?;
        let error = match token {
            Some(token) => {
                if let Some(value) = convert(token) {
                    return Ok(value);
                }
                DecodeError::Type {
                    offset: token.offset,
                    expected,
                    actual: token.value.kind(),
                }
            }
            None => DecodeError::EndOfInput {
                offset: self.walker.offset(),
                expected,
            },
        };
        self.failed = true;
        Err(error)
    }

    pub fn null(&mut self) -> DecodeResult<()> {
        self.read("null", |token| {
            (token.value == TokenValue::Null).then_some(())
        })
    }

    pub fn boolean(&mut self) -> DecodeResult<bool> {
        self.read("Boolean", |token| match token.value {
            TokenValue::Bool(value) => Some(value),
            _ => None,
        })
    }

    pub fn integer(&mut self) -> DecodeResult<IntegerRef<'a>> {
        self.read("integer", |token| match token.value {
            TokenValue::Integer(value) => Some(value),
            _ => None,
        })
    }

    pub fn unsigned(&mut self) -> DecodeResult<u64> {
        self.read("native unsigned integer", |token| match token.value {
            TokenValue::Integer(IntegerRef::Unsigned(value)) => Some(value),
            _ => None,
        })
    }

    pub fn bytes(&mut self) -> DecodeResult<&'a [u8]> {
        self.read("byte string", |token| match token.value {
            TokenValue::Bytes(value) => Some(value),
            _ => None,
        })
    }

    pub fn text(&mut self) -> DecodeResult<&'a str> {
        self.read("text", |token| match token.value {
            TokenValue::Text(value) => Some(value),
            _ => None,
        })
    }

    pub fn array(&mut self) -> DecodeResult<usize> {
        self.read("array", |token| match token.value {
            TokenValue::Array(length) => Some(length),
            _ => None,
        })
    }

    pub fn map(&mut self) -> DecodeResult<usize> {
        self.read("map", |token| match token.value {
            TokenValue::Map(length) => Some(length),
            _ => None,
        })
    }

    pub fn key(&mut self) -> DecodeResult<&'a str> {
        self.read("map key", |token| match token.value {
            TokenValue::Key(value) => Some(value),
            _ => None,
        })
    }

    /// Consume the closing token and borrow the entire encoded array.
    pub fn end_array(&mut self) -> DecodeResult<&'a [u8]> {
        self.read("end of array", |token| {
            (token.value == TokenValue::EndArray).then_some(token.bytes)
        })
    }

    /// Consume the closing token and borrow the entire encoded map.
    pub fn end_map(&mut self) -> DecodeResult<&'a [u8]> {
        self.read("end of map", |token| {
            (token.value == TokenValue::EndMap).then_some(token.bytes)
        })
    }

    /// Require every token to have been consumed, including container endings.
    /// Return the admitted original bytes without a second pass or payload copy.
    pub fn finish(mut self) -> DecodeResult<RawCbor<'a>> {
        if let Some(token) = self.next_token()? {
            return Err(DecodeError::Unconsumed {
                offset: token.offset,
            });
        }
        Ok(RawCbor {
            bytes: Cow::Borrowed(self.bytes),
        })
    }
}
