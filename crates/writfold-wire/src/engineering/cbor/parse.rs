use super::{Error, IntegerRef, Token, TokenValue};
use crate::{Budget, Limits};

struct Frame<'a> {
    remaining: usize,
    start: usize,
    key_pending: bool,
    map: bool,
    previous_key: Option<&'a [u8]>,
}

struct Parser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Parser<'a> {
    fn take(&mut self, length: u64) -> Result<&'a [u8], Error> {
        let end = usize::try_from(length)
            .ok()
            .and_then(|length| self.offset.checked_add(length))
            .filter(|&end| end <= self.bytes.len())
            .ok_or(Error::Syntax {
                offset: self.offset,
                reason: "truncated item",
            })?;
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn header(&mut self) -> Result<(u8, u64), Error> {
        let start = self.offset;
        let byte = self.take(1)?[0];
        let major = byte >> 5;
        let info = byte & 31;
        let (width, minimum) = match info {
            0..=23 => return Ok((major, u64::from(info))),
            24 => (1, 24),
            25 => (2, 256),
            26 => (4, 65_536),
            27 => (8, 4_294_967_296),
            _ => {
                return Err(Error::Syntax {
                    offset: start,
                    reason: "indefinite or reserved additional information",
                });
            }
        };
        // Floating-point arguments are not integer headers in this profile.
        if major == 7 {
            return Err(Error::Syntax {
                offset: start,
                reason: "unsupported simple value or float",
            });
        }
        let mut value = 0_u64;
        for &byte in self.take(width)? {
            value = (value << 8) | u64::from(byte);
        }
        if value < minimum {
            return Err(Error::NonCanonical {
                offset: start,
                reason: "non-shortest header",
            });
        }
        Ok((major, value))
    }

    fn text(&mut self, length: u64) -> Result<&'a str, Error> {
        let offset = self.offset;
        std::str::from_utf8(self.take(length)?).map_err(|error| Error::Unicode {
            offset: offset + error.valid_up_to(),
        })
    }

    fn key(&mut self, previous: &mut Option<&'a [u8]>) -> Result<&'a str, Error> {
        let start = self.offset;
        let (major, length) = self.header()?;
        if major != 3 {
            return Err(Error::Syntax {
                offset: start,
                reason: "map key must be text",
            });
        }
        let text = self.text(length)?;
        let key = &self.bytes[start..self.offset];
        if let Some(prior) = previous {
            match (*prior).cmp(key) {
                std::cmp::Ordering::Equal => return Err(Error::DuplicateKey { offset: start }),
                std::cmp::Ordering::Greater => {
                    return Err(Error::NonCanonical {
                        offset: start,
                        reason: "map keys out of encoded-byte order",
                    });
                }
                std::cmp::Ordering::Less => {}
            }
        }
        *previous = Some(key);
        Ok(text)
    }

    fn bignum(&mut self) -> Result<&'a [u8], Error> {
        let start = self.offset;
        let (major, length) = self.header()?;
        if major != 2 {
            return Err(Error::Syntax {
                offset: start,
                reason: "bignum requires byte string",
            });
        }
        let bytes = self.take(length)?;
        if bytes.len() <= 8 || bytes[0] == 0 {
            return Err(Error::NonCanonical {
                offset: start,
                reason: "nonminimal bignum magnitude",
            });
        }
        Ok(bytes)
    }
}

pub(super) struct Walker<'a> {
    parser: Parser<'a>,
    stack: Vec<Frame<'a>>,
    nodes: usize,
    limits: Limits,
}

impl<'a> Walker<'a> {
    pub(super) fn new(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.bytes() {
            return Err(Error::Limit {
                budget: Budget::Bytes,
                offset: limits.bytes(),
            });
        }
        Ok(Self {
            parser: Parser { bytes, offset: 0 },
            stack: Vec::new(),
            nodes: 0,
            limits,
        })
    }

    pub(super) fn offset(&self) -> usize {
        self.parser.offset
    }

    fn token(&self, start: usize, value: TokenValue<'a>) -> Token<'a> {
        Token {
            value,
            bytes: &self.parser.bytes[start..self.parser.offset],
            offset: start,
        }
    }

    pub(super) fn next_token(&mut self) -> Result<Option<Token<'a>>, Error> {
        if self.stack.last().is_some_and(|frame| frame.remaining == 0)
            && let Some(frame) = self.stack.pop()
        {
            let value = if frame.map {
                TokenValue::EndMap
            } else {
                TokenValue::EndArray
            };
            return Ok(Some(self.token(frame.start, value)));
        }
        if self.nodes != 0 && self.stack.is_empty() {
            return if self.parser.offset == self.parser.bytes.len() {
                Ok(None)
            } else {
                Err(Error::TrailingInput {
                    offset: self.parser.offset,
                })
            };
        }
        if let Some(frame) = self.stack.last_mut() {
            if frame.map && frame.key_pending {
                let start = self.parser.offset;
                let text = self.parser.key(&mut frame.previous_key)?;
                frame.key_pending = false;
                return Ok(Some(self.token(start, TokenValue::Key(text))));
            }
            frame.remaining -= 1;
            frame.key_pending = true;
        }
        self.value()
    }

    fn value(&mut self) -> Result<Option<Token<'a>>, Error> {
        let start = self.parser.offset;
        if self.nodes == self.limits.nodes() {
            return Err(Error::Limit {
                budget: Budget::Nodes,
                offset: start,
            });
        }
        self.nodes += 1;
        let (major, argument) = self.parser.header()?;
        let value = match major {
            0 => TokenValue::Integer(IntegerRef::Unsigned(argument)),
            1 => TokenValue::Integer(IntegerRef::Negative(argument)),
            2 => TokenValue::Bytes(self.parser.take(argument)?),
            3 => TokenValue::Text(self.parser.text(argument)?),
            4 | 5 => {
                if self.stack.len() == self.limits.depth() {
                    return Err(Error::Limit {
                        budget: Budget::Depth,
                        offset: start,
                    });
                }
                let remaining = usize::try_from(argument)
                    .ok()
                    .filter(|&size| size <= self.limits.container_entries())
                    .ok_or(Error::Limit {
                        budget: Budget::ContainerEntries,
                        offset: start,
                    })?;
                // No allocation is based on an advertised container size.
                self.stack.push(Frame {
                    remaining,
                    start,
                    key_pending: true,
                    map: major == 5,
                    previous_key: None,
                });
                if major == 4 {
                    TokenValue::Array(remaining)
                } else {
                    TokenValue::Map(remaining)
                }
            }
            6 if argument == 2 => {
                TokenValue::Integer(IntegerRef::PositiveBig(self.parser.bignum()?))
            }
            6 if argument == 3 => {
                TokenValue::Integer(IntegerRef::NegativeBig(self.parser.bignum()?))
            }
            7 if argument == 20 => TokenValue::Bool(false),
            7 if argument == 21 => TokenValue::Bool(true),
            7 if argument == 22 => TokenValue::Null,
            _ => {
                return Err(Error::Syntax {
                    offset: start,
                    reason: "unsupported tag or simple value",
                });
            }
        };
        Ok(Some(self.token(start, value)))
    }
}

pub(super) fn validate(bytes: &[u8], limits: Limits) -> Result<(), Error> {
    let mut walker = Walker::new(bytes, limits)?;
    while walker.next_token()?.is_some() {}
    Ok(())
}
