use super::{NativeError, NativeLimits, RawResponse};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_RESPONSE_HEADER_BYTES: u64 = 65_536;
const MAX_HEADER_FIELDS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Framing {
    Length(usize),
    Chunked,
}

pub(super) fn stop(deadline: Instant, cancelled: &AtomicBool) -> Result<(), NativeError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(NativeError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(NativeError::Timeout);
    }
    Ok(())
}

pub(super) fn segment(value: &str) -> Result<String, NativeError> {
    if value.is_empty()
        || value.len() > 4096
        || value == "."
        || value == ".."
        || value.chars().any(char::is_control)
    {
        return Err(NativeError::Input("invalid route segment"));
    }
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    Ok(encoded)
}

pub(super) fn connect(
    address: SocketAddr,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<TcpStream, NativeError> {
    stop(deadline, cancelled)?;
    // No retry: this is only the owned IPv4 loopback listener. A short connect
    // bound also prevents cancellation from waiting for the whole call deadline.
    let timeout = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_millis(50));
    if timeout.is_zero() {
        return Err(NativeError::Timeout);
    }
    let socket = TcpStream::connect_timeout(&address, timeout)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

pub(super) fn exchange(
    socket: &mut TcpStream,
    header: &[u8],
    body: &[u8],
    limits: NativeLimits,
    mut check: impl FnMut() -> Result<(), NativeError>,
) -> Result<RawResponse, NativeError> {
    for bytes in [header, body] {
        let mut sent = 0;
        while sent < bytes.len() {
            check()?;
            match socket.write(&bytes[sent..]) {
                Ok(0) => return Err(NativeError::Protocol("request write closed")),
                Ok(count) => sent += count,
                Err(error) if pending(&error) => std::thread::sleep(Duration::from_millis(1)),
                Err(error) => return Err(error.into()),
            }
        }
    }
    let mut headers = Vec::new();
    let mut buffer = [0; 8192];
    let mut scanned = 0;
    let header_limit = limits.response_header.get().min(MAX_RESPONSE_HEADER_BYTES);
    let end = loop {
        check()?;
        if let Some(end) = header_end(&headers[scanned..]) {
            break scanned + end;
        }
        scanned = headers.len().saturating_sub(3);
        let remaining = header_limit.saturating_sub(headers.len() as u64);
        if remaining == 0 {
            return Err(NativeError::Budget("response header"));
        }
        let count = remaining.min(buffer.len() as u64) as usize;
        match socket.read(&mut buffer[..count]) {
            Ok(0) => return Err(NativeError::Protocol("truncated response header")),
            Ok(count) => headers.extend_from_slice(&buffer[..count]),
            Err(error) if pending(&error) => std::thread::sleep(Duration::from_millis(1)),
            Err(error) => return Err(error.into()),
        }
    };
    let (status, framing) = parse_headers(&headers[..end], &mut check)?;
    let prefix = headers.split_off(end);
    let body = match framing {
        Framing::Chunked => {
            super::chunked::decode(socket, prefix, limits.response_body.get(), &mut check)?
        }
        Framing::Length(length) => {
            if length as u64 > limits.response_body.get() {
                return Err(NativeError::Budget("response body"));
            }
            let mut body = prefix;
            if body.len() > length {
                return Err(NativeError::Protocol("trailing response bytes"));
            }
            body.try_reserve_exact(length - body.len())
                .map_err(|_| NativeError::Budget("response allocation"))?;
            while body.len() < length {
                check()?;
                let count = (length - body.len()).min(buffer.len());
                match socket.read(&mut buffer[..count]) {
                    Ok(0) => return Err(NativeError::Protocol("truncated response body")),
                    Ok(count) => body.extend_from_slice(&buffer[..count]),
                    Err(error) if pending(&error) => std::thread::sleep(Duration::from_millis(1)),
                    Err(error) => return Err(error.into()),
                }
            }
            body
        }
    };
    // Connection: close is mandatory. Reading its EOF detects response
    // smuggling/trailing data rather than silently discarding another frame.
    loop {
        check()?;
        match socket.read(&mut buffer[..1]) {
            Ok(0) => break,
            Ok(_) => return Err(NativeError::Protocol("trailing response bytes")),
            Err(error) if pending(&error) => std::thread::sleep(Duration::from_millis(1)),
            Err(error) => return Err(error.into()),
        }
    }
    check()?;
    Ok(RawResponse {
        status,
        headers,
        body,
    })
}

fn pending(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
    )
}

fn header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .map(|index| index + 4)
}

pub(super) fn parse_headers(
    bytes: &[u8],
    check: &mut impl FnMut() -> Result<(), NativeError>,
) -> Result<(u16, Framing), NativeError> {
    check()?;
    if bytes.len() as u64 > MAX_RESPONSE_HEADER_BYTES {
        return Err(NativeError::Budget("response header"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| NativeError::Protocol("non-ASCII response headers"))?;
    if !text.is_ascii() || !text.ends_with("\r\n\r\n") {
        return Err(NativeError::Protocol("header framing"));
    }
    let mut lines = text[..text.len() - 4].split("\r\n");
    let status = status_line(
        lines
            .next()
            .ok_or(NativeError::Protocol("missing status"))?,
    )?;
    let mut length = None;
    let mut chunked = false;
    let mut contract = false;
    let mut close = false;
    let mut seen = [None::<(&str, &str)>; MAX_HEADER_FIELDS];
    for (index, line) in lines.enumerate() {
        check()?;
        if index == MAX_HEADER_FIELDS {
            return Err(NativeError::Budget("response header fields"));
        }
        let (name, value) = header_field(line)?;
        if let Some((_, previous)) = seen[..index]
            .iter()
            .flatten()
            .find(|(previous, _)| previous.eq_ignore_ascii_case(name))
            && (*previous != value
                || name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("omnigraph-http-api")
                || name.eq_ignore_ascii_case("transfer-encoding"))
        {
            return Err(NativeError::Protocol("duplicate ambiguous header"));
        }
        seen[index] = Some((name, value));
        if name.eq_ignore_ascii_case("content-encoding") {
            return Err(NativeError::Protocol("unsupported response coding"));
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            if !value.eq_ignore_ascii_case("chunked") {
                return Err(NativeError::Protocol("unsupported transfer coding"));
            }
            chunked = true;
        }
        if name.eq_ignore_ascii_case("content-length") {
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(NativeError::Protocol("invalid content length"));
            }
            length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| NativeError::Protocol("content length overflow"))?,
            );
        }
        if name.eq_ignore_ascii_case("omnigraph-http-api") {
            if value != "0.14" {
                return Err(NativeError::Protocol("wrong HTTP contract"));
            }
            contract = true;
        }
        if name.eq_ignore_ascii_case("connection") {
            if !value.eq_ignore_ascii_case("close") {
                return Err(NativeError::Protocol("response connection not close"));
            }
            close = true;
        }
    }
    Ok((status, response_framing(length, chunked, contract, close)?))
}

fn response_framing(
    length: Option<usize>,
    chunked: bool,
    contract: bool,
    close: bool,
) -> Result<Framing, NativeError> {
    if !contract || !close {
        return Err(NativeError::Protocol("missing contract or close header"));
    }
    match (length, chunked) {
        (Some(length), false) => Ok(Framing::Length(length)),
        (None, true) => Ok(Framing::Chunked),
        _ => Err(NativeError::Protocol(
            "missing or ambiguous response framing",
        )),
    }
}

pub(super) fn announcement(bytes: &[u8]) -> Result<Option<SocketAddr>, NativeError> {
    const PREFIX: &[u8] = b"OMNIGRAPH_LISTEN_ADDR=";
    let mut address = None;
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        if !line.ends_with(b"\n") {
            continue;
        }
        if let Some(value) = line.strip_prefix(PREFIX) {
            if address.is_some() || value.len() > 64 {
                return Err(NativeError::Protocol(
                    "duplicate or oversized listener announcement",
                ));
            }
            let value = std::str::from_utf8(&value[..value.len() - 1])
                .map_err(|_| NativeError::Protocol("invalid listener address"))?;
            let parsed: SocketAddr = value
                .parse()
                .map_err(|_| NativeError::Protocol("invalid listener address"))?;
            if !matches!(parsed, SocketAddr::V4(v4) if *v4.ip() == std::net::Ipv4Addr::LOCALHOST && v4.port() != 0)
            {
                return Err(NativeError::Protocol("listener is not IPv4 loopback"));
            }
            address = Some(parsed);
        }
    }
    Ok(address)
}

fn status_line(line: &str) -> Result<u16, NativeError> {
    let bytes = line.as_bytes();
    if bytes.len() < 13
        || !line.starts_with("HTTP/1.1 ")
        || bytes[12] != b' '
        || !bytes[9..12].iter().all(u8::is_ascii_digit)
        || bytes[13..].iter().any(|byte| !(32..=126).contains(byte))
    {
        return Err(NativeError::Protocol("invalid status line"));
    }
    let status = line[9..12]
        .parse::<u16>()
        .map_err(|_| NativeError::Protocol("invalid status"))?;
    if !(200..=599).contains(&status) || (300..=399).contains(&status) {
        return Err(NativeError::Protocol("interim response or redirect"));
    }
    Ok(status)
}

fn header_field(line: &str) -> Result<(&str, &str), NativeError> {
    let (name, value) = line
        .split_once(':')
        .ok_or(NativeError::Protocol("malformed header"))?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        || value.bytes().any(|b| b < 32 && b != b'\t' || b == 127)
    {
        return Err(NativeError::Protocol("invalid header characters"));
    }
    Ok((name, value.trim_matches([' ', '\t'])))
}
