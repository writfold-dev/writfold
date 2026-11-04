use super::Error;
use crate::{Budget, Limits};

struct Frame<'a> {
    remaining: usize,
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

    fn text(&mut self, length: u64) -> Result<(), Error> {
        let offset = self.offset;
        std::str::from_utf8(self.take(length)?).map_err(|error| Error::Unicode {
            offset: offset + error.valid_up_to(),
        })?;
        Ok(())
    }

    fn key(&mut self, previous: &mut Option<&'a [u8]>) -> Result<(), Error> {
        let start = self.offset;
        let (major, length) = self.header()?;
        if major != 3 {
            return Err(Error::Syntax {
                offset: start,
                reason: "map key must be text",
            });
        }
        self.text(length)?;
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
        Ok(())
    }

    fn bignum(&mut self) -> Result<(), Error> {
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
        Ok(())
    }
}

pub(super) fn validate(bytes: &[u8], limits: Limits) -> Result<(), Error> {
    if bytes.len() > limits.bytes() {
        return Err(Error::Limit {
            budget: Budget::Bytes,
            offset: limits.bytes(),
        });
    }
    let mut parser = Parser { bytes, offset: 0 };
    let mut stack: Vec<Frame<'_>> = Vec::new();
    let mut nodes = 0_usize;
    loop {
        while stack.last().is_some_and(|frame| frame.remaining == 0) {
            stack.pop();
        }
        if nodes != 0 && stack.is_empty() {
            return if parser.offset == bytes.len() {
                Ok(())
            } else {
                Err(Error::TrailingInput {
                    offset: parser.offset,
                })
            };
        }
        if let Some(frame) = stack.last_mut() {
            if frame.map {
                parser.key(&mut frame.previous_key)?;
            }
            frame.remaining -= 1;
        }
        let start = parser.offset;
        if nodes == limits.nodes() {
            return Err(Error::Limit {
                budget: Budget::Nodes,
                offset: start,
            });
        }
        nodes += 1;
        let (major, argument) = parser.header()?;
        match major {
            0 | 1 => {}
            2 => {
                parser.take(argument)?;
            }
            3 => parser.text(argument)?,
            4 | 5 => {
                if stack.len() == limits.depth() {
                    return Err(Error::Limit {
                        budget: Budget::Depth,
                        offset: start,
                    });
                }
                let remaining = usize::try_from(argument)
                    .ok()
                    .filter(|&size| size <= limits.container_entries())
                    .ok_or(Error::Limit {
                        budget: Budget::ContainerEntries,
                        offset: start,
                    })?;
                // No allocation is based on an advertised container size.
                stack.push(Frame {
                    remaining,
                    map: major == 5,
                    previous_key: None,
                });
            }
            6 if argument == 2 || argument == 3 => parser.bignum()?,
            7 if matches!(argument, 20..=22) => {}
            _ => {
                return Err(Error::Syntax {
                    offset: start,
                    reason: "unsupported tag or simple value",
                });
            }
        }
    }
}
