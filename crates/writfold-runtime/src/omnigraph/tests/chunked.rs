use super::super::{NativeError, NativeLimits, RawResponse, http};
use super::headers;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use writfold_domain::ByteCount;

fn receive(
    payload: Vec<u8>,
    body_limit: u64,
) -> Result<Result<RawResponse, NativeError>, Box<dyn Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let mut socket = TcpStream::connect(listener.local_addr()?)?;
    socket.set_nonblocking(true)?;
    let mut response = headers("Transfer-Encoding: chunked\r\n");
    response.extend_from_slice(&payload);
    let peer = std::thread::spawn(move || -> std::io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.read_exact(&mut [0; 1])?;
        stream.write_all(&response)?;
        Ok(())
    });
    let count = ByteCount::new(16_384)?;
    let limits = NativeLimits {
        request_header: count,
        request_body: count,
        response_header: count,
        response_body: ByteCount::new(body_limit)?,
        stdout: count,
        stderr: count,
        shutdown_grace: Duration::from_secs(1),
    };
    let cancelled = AtomicBool::new(false);
    let deadline = Instant::now() + Duration::from_secs(2);
    let result = http::exchange(&mut socket, b"x", b"", limits, || {
        http::stop(deadline, &cancelled)
    });
    drop(socket);
    peer.join()
        .map_err(|_| std::io::Error::other("test peer panicked"))??;
    Ok(result)
}

#[test]
fn chunk_boundaries_do_not_change_the_native_json_value() -> Result<(), Box<dyn Error>> {
    let response = receive(b"3\r\n{\"n\r\n4\r\n\":7}\r\n0\r\n\r\n".to_vec(), 7)??;
    let value: serde_json::Value = serde_json::from_slice(&response.body)?;
    assert_eq!(value, serde_json::json!({"n":7}));
    Ok(())
}

#[test]
fn chunk_payload_survives_buffer_refills_without_truncation() -> Result<(), Box<dyn Error>> {
    let expected = "abcλ".repeat(2000);
    let json = serde_json::to_vec(&serde_json::json!({"text":expected}))?;
    let mut encoded = Vec::new();
    for piece in json.chunks(137) {
        write!(&mut encoded, "{:x}\r\n", piece.len())?;
        encoded.extend_from_slice(piece);
        encoded.extend_from_slice(b"\r\n");
    }
    encoded.extend_from_slice(b"0\r\n\r\n");
    let response = receive(encoded, json.len() as u64)??;
    let value: serde_json::Value = serde_json::from_slice(&response.body)?;
    assert_eq!(value, serde_json::json!({"text":expected}));
    Ok(())
}

#[test]
fn incomplete_ambiguous_and_oversized_chunks_never_become_success() -> Result<(), Box<dyn Error>> {
    for malformed in [
        b"1\r\nx\r\n".as_slice(),
        b"2\r\nx".as_slice(),
        b"g\r\nx\r\n0\r\n\r\n".as_slice(),
        b"1;x=y\r\nx\r\n0\r\n\r\n".as_slice(),
        b"1\r\nx!\n0\r\n\r\n".as_slice(),
        b"0\r\nX-Trailer: value\r\n\r\n".as_slice(),
        b"0\r\n\r\nextra".as_slice(),
    ] {
        assert!(receive(malformed.to_vec(), 7)?.is_err());
    }
    assert!(matches!(
        receive(b"8\r\n12345678\r\n0\r\n\r\n".to_vec(), 7)?,
        Err(NativeError::Budget("response body"))
    ));
    assert!(matches!(
        receive(b"ffffffffffffffff\r\n".to_vec(), 7)?,
        Err(NativeError::Budget("response body"))
    ));
    Ok(())
}
