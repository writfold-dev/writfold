use super::NativeError;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

const MAX_CHUNKS: usize = 65_536;

/// The native response observer wraps successful bodies in a stream. Admit its
/// ordinary chunked framing, not extensions, trailers or stacked transfer codes.
pub(super) fn decode(
    socket: &mut TcpStream,
    prefix: Vec<u8>,
    limit: u64,
    check: &mut impl FnMut() -> Result<(), NativeError>,
) -> Result<Vec<u8>, NativeError> {
    let mut input = Input::new(socket, prefix);
    let mut body = Vec::new();
    let mut chunks = 0;
    loop {
        check()?;
        let length = input.length(check)?;
        if length == 0 {
            input.crlf(check)?;
            if input.position != input.filled {
                return Err(NativeError::Protocol("trailing chunked response bytes"));
            }
            return Ok(body);
        }
        if chunks == MAX_CHUNKS {
            return Err(NativeError::Budget("response chunks"));
        }
        chunks += 1;
        if length > limit.saturating_sub(body.len() as u64) {
            return Err(NativeError::Budget("response body"));
        }
        let length = usize::try_from(length).map_err(|_| NativeError::Budget("response body"))?;
        let required = body
            .len()
            .checked_add(length)
            .ok_or(NativeError::Budget("response body"))?;
        if required > body.capacity() {
            let ceiling = usize::try_from(limit).unwrap_or(usize::MAX);
            let capacity = required.max(body.capacity().saturating_mul(2)).min(ceiling);
            body.try_reserve_exact(capacity - body.len())
                .map_err(|_| NativeError::Budget("response allocation"))?;
        }
        let mut remaining = length;
        while remaining != 0 {
            check()?;
            input.fill(check)?;
            let count = remaining.min(input.filled - input.position);
            body.extend_from_slice(&input.bytes[input.position..input.position + count]);
            input.position += count;
            remaining -= count;
        }
        input.crlf(check)?;
    }
}

struct Input<'a> {
    socket: &'a mut TcpStream,
    bytes: Vec<u8>,
    position: usize,
    filled: usize,
}

impl<'a> Input<'a> {
    fn new(socket: &'a mut TcpStream, mut bytes: Vec<u8>) -> Self {
        let filled = bytes.len();
        bytes.resize(bytes.len().max(8192), 0);
        Self {
            socket,
            bytes,
            position: 0,
            filled,
        }
    }

    fn fill(
        &mut self,
        check: &mut impl FnMut() -> Result<(), NativeError>,
    ) -> Result<(), NativeError> {
        if self.position < self.filled {
            return Ok(());
        }
        loop {
            check()?;
            match self.socket.read(&mut self.bytes) {
                Ok(0) => return Err(NativeError::Protocol("truncated chunked response")),
                Ok(count) => {
                    self.position = 0;
                    self.filled = count;
                    return Ok(());
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn byte(
        &mut self,
        check: &mut impl FnMut() -> Result<(), NativeError>,
    ) -> Result<u8, NativeError> {
        self.fill(check)?;
        let byte = self.bytes[self.position];
        self.position += 1;
        Ok(byte)
    }

    fn crlf(
        &mut self,
        check: &mut impl FnMut() -> Result<(), NativeError>,
    ) -> Result<(), NativeError> {
        if self.byte(check)? != b'\r' || self.byte(check)? != b'\n' {
            return Err(NativeError::Protocol(
                "invalid chunk terminator or unsupported trailers",
            ));
        }
        Ok(())
    }

    fn length(
        &mut self,
        check: &mut impl FnMut() -> Result<(), NativeError>,
    ) -> Result<u64, NativeError> {
        let mut length = 0u64;
        let mut digits = 0;
        loop {
            let byte = self.byte(check)?;
            if byte == b'\r' && digits != 0 {
                if self.byte(check)? != b'\n' {
                    return Err(NativeError::Protocol("invalid chunk size terminator"));
                }
                return Ok(length);
            }
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => {
                    return Err(NativeError::Protocol(
                        "invalid chunk size or unsupported extension",
                    ));
                }
            };
            if digits == 16 {
                return Err(NativeError::Budget("chunk size line"));
            }
            digits += 1;
            length = length
                .checked_mul(16)
                .and_then(|value| value.checked_add(u64::from(digit)))
                .ok_or(NativeError::Budget("chunk size overflow"))?;
        }
    }
}
