use std::{
    collections::TryReserveError,
    io::{self, BufRead},
};
use thiserror::Error;
use writfold_domain::ByteCount;

#[derive(Debug, Error)]
pub enum FrameReadError {
    #[error("frame byte budget is not representable on this platform")]
    Budget,
    #[error("reading a request frame failed: {0}")]
    Io(#[source] io::Error),
    #[error("allocating request frame storage failed: {0}")]
    Allocation(#[source] TryReserveError),
    #[error("frame reader cannot continue after a previous failure")]
    Failed,
}

#[derive(Debug, Eq, PartialEq)]
pub enum InputFrame<'a> {
    Bytes(&'a [u8]),
    Oversized,
}

/// Newline-delimited bytes, with a budget including the newline itself.
///
/// Storage grows on demand up to the budget and is reused between calls. The
/// caller owns the buffering policy of `R`. Returned bytes borrow this reader
/// until its next mutable use. Any non-Interrupted error permanently fails the
/// reader: a partial request's suffix must never become a separate request.
pub struct FrameReader<R> {
    reader: R,
    bytes: Vec<u8>,
    maximum: usize,
    failed: bool,
}

impl<R: BufRead> FrameReader<R> {
    pub fn new(reader: R, maximum: ByteCount) -> Result<Self, FrameReadError> {
        let maximum = usize::try_from(maximum.get()).map_err(|_| FrameReadError::Budget)?;
        Ok(Self {
            reader,
            bytes: Vec::new(),
            maximum,
            failed: false,
        })
    }

    pub fn next_frame(&mut self) -> Result<Option<InputFrame<'_>>, FrameReadError> {
        if self.failed {
            return Err(FrameReadError::Failed);
        }
        self.bytes.clear();
        let mut oversized = false;
        loop {
            let available = match self.reader.fill_buf() {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    self.failed = true;
                    return Err(FrameReadError::Io(error));
                }
            };
            if available.is_empty() {
                return Ok(if oversized {
                    Some(InputFrame::Oversized)
                } else if self.bytes.is_empty() {
                    None
                } else {
                    Some(InputFrame::Bytes(&self.bytes))
                });
            }
            let newline = available.iter().position(|byte| *byte == b'\n');
            let consumed = newline.map_or(available.len(), |index| index + 1);
            if !oversized {
                if consumed > self.maximum - self.bytes.len() {
                    oversized = true;
                    self.bytes.clear();
                } else {
                    let needed = self.bytes.len() + consumed;
                    if needed > self.bytes.capacity() {
                        let capacity = needed
                            .max(self.bytes.capacity().saturating_mul(2))
                            .min(self.maximum);
                        if let Err(error) =
                            self.bytes.try_reserve_exact(capacity - self.bytes.len())
                        {
                            self.failed = true;
                            return Err(FrameReadError::Allocation(error));
                        }
                    }
                    self.bytes.extend_from_slice(&available[..consumed]);
                }
            }
            self.reader.consume(consumed);
            if newline.is_some() {
                return Ok(Some(if oversized {
                    InputFrame::Oversized
                } else {
                    InputFrame::Bytes(&self.bytes)
                }));
            }
        }
    }
}
