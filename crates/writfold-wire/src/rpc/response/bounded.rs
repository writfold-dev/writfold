//! Shared fallible output storage for borrowed JSON encoders.
use serde::Serialize;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Failure {
    Limit,
    Allocation,
    Encoding,
}

pub(crate) struct Bounded {
    bytes: Vec<u8>,
    maximum: usize,
    failure: Option<Failure>,
}

impl Bounded {
    pub(crate) fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
            failure: None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn append(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        self.write_all(bytes)
            .map_err(|_| self.failure.unwrap_or(Failure::Encoding))
    }

    pub(crate) fn json<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Failure> {
        serde_json::to_writer(&mut *self, value)
            .map_err(|_| self.failure.unwrap_or(Failure::Encoding))
    }
}

impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            self.failure = Some(Failure::Limit);
            return Err(io::ErrorKind::WriteZero.into());
        }
        let required = self.bytes.len() + bytes.len();
        if required > self.bytes.capacity() {
            let capacity = required
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.maximum);
            if self
                .bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .is_err()
            {
                self.failure = Some(Failure::Allocation);
                return Err(io::ErrorKind::OutOfMemory.into());
            }
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
